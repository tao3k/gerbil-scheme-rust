//! Incremental publication into the existing bounded pool source and producers.
use super::{
    InstanceClientProgressDriver, InstanceDispatcher, InstanceError, SchemeServicePoolStreamConfig,
    SchemeServiceReadyStream, SchemeServiceRequest, SchemeServiceStream, SchemeServiceStreamConfig,
    SchemeServiceSupplyAdmission,
    service_stream_runner::ServiceProducer,
    service_stream_source::{Shared, Source},
};
use std::{
    collections::VecDeque,
    future::Future,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

// This guard exists before the first registration can be polled. Cancellation,
// panic or partial registration failure seals/rejects the source and wakes all
// installed producers even before their weak signals reach the returned stream.
struct Publishing {
    source: Arc<Shared>,
    cancelled: Arc<AtomicBool>,
    armed: bool,
}
impl Drop for Publishing {
    fn drop(&mut self) {
        if self.armed {
            self.cancelled.store(true, Ordering::Release);
            Source::Pool(Arc::clone(&self.source)).reject(&InstanceError::Cancelled);
        }
    }
}
struct Plan<R> {
    config: SchemeServicePoolStreamConfig,
    count: usize,
    window: usize,
    deadline: Instant,
    source: Arc<Shared>,
    cancelled: Arc<AtomicBool>,
    capacities: Vec<Arc<super::capacity::Capacity>>,
    send: Arc<super::service_ready::Channels<R>>,
}
#[derive(Default)]
struct Cursor {
    sequence: usize,
    work: usize,
}
type Signals = Vec<Weak<super::service_stream_runner::Signal>>;
impl InstanceDispatcher {
    /// Publish bounded root-free preparation windows while producers register
    /// and previously published inputs enter native service. An empty open
    /// source keeps its existing registration; seal is monotonic end-of-input.
    /// The preparation window derives from aggregate existing active capacity.
    /// Client progress is serviced once per window, not once per input item.
    /// Existing count and cumulative declared-work limits cover the whole source.
    /// This API neither bounds application payload bytes nor preempts a Rust
    /// preparation callback. Each callback must be bounded and must not enter Scheme.
    ///
    /// This opt-in API permits native acceptance before all registrations or
    /// input validation finish. A later error can therefore follow native side
    /// effects from a valid prefix. Only unentered input is cancelled; accepted
    /// Scheme work retains its owner, roots and credits through normal drain.
    /// A single absolute deadline covers preparation, registration and admission.
    /// No default, Scheme scheduling, handler budget or result-release rule changes.
    /// # Errors
    /// Invalid bounds/owners, input error, registration failure, timeout or close.
    pub async fn pipeline_scheme_service_pool_ready_stream<I, M, P>(
        &self,
        mut messages: I,
        config: SchemeServicePoolStreamConfig,
        mode: SchemeServiceSupplyAdmission,
        driver: &mut InstanceClientProgressDriver<P>,
    ) -> Result<SchemeServiceReadyStream<M::Output>, InstanceError>
    where
        I: ExactSizeIterator<Item = Result<M, InstanceError>>,
        M: SchemeServiceRequest,
        P: Future,
    {
        let (plan, mut stream) = Plan::new::<M>(self, messages.len(), config, mode)?;
        let mut publishing = Publishing {
            source: Arc::clone(&plan.source),
            cancelled: Arc::clone(&plan.cancelled),
            armed: true,
        };
        let mut registering = InstanceClientProgressDriver::new(plan.register(self, mode));
        plan.publish_windows(self, &mut messages, driver, &mut registering)
            .await?;
        plan.source.seal();
        stream.signals = driver.wait(registering.finish()).await?;
        publishing.armed = false;
        Ok(SchemeServiceReadyStream(stream))
    }
}
impl<R: Send + 'static> Plan<R> {
    fn new<M: SchemeServiceRequest<Output = R>>(
        dispatch: &InstanceDispatcher,
        count: usize,
        mut config: SchemeServicePoolStreamConfig,
        mode: SchemeServiceSupplyAdmission,
    ) -> Result<(Self, SchemeServiceStream<R>), InstanceError> {
        let deadline =
            Instant::now()
                .checked_add(config.timeout)
                .ok_or(InstanceError::InvalidConfig(
                    "service pool deadline overflow",
                ))?;
        config.owners.sort_unstable();
        if count == 0
            || count > dispatch.admission.queue_capacity
            || config.owners.is_empty()
            || config.owners.windows(2).any(|w| w[0] == w[1])
            || config.maximum_active_per_owner == 0
        {
            return Err(InstanceError::InvalidConfig(
                "service pool count/owner bound",
            ));
        }
        if config.timeout.is_zero() {
            return Err(InstanceError::Timeout);
        }
        if mode == SchemeServiceSupplyAdmission::ProgressFair {
            dispatch.require_progress_owners(&config.owners)?;
        }
        let capacities = config
            .owners
            .iter()
            .map(|&owner| dispatch.pool_capacity::<M>(owner, &[], config.maximum_active_per_owner))
            .collect::<Result<Vec<_>, _>>()?;
        let window = config
            .owners
            .len()
            .checked_mul(config.maximum_active_per_owner)
            .ok_or(InstanceError::InvalidConfig(
                "service preparation window overflow",
            ))?
            .min(count);
        let source = Shared::opening();
        if mode == SchemeServiceSupplyAdmission::ProgressFair {
            source
                .fair
                .set(super::Fair::default())
                .map_err(|_| InstanceError::InvalidConfig("duplicate progress source"))?;
        }
        let (send, receive) = super::service_ready::channels(&config.owners, count);
        let cancelled = Arc::new(AtomicBool::new(false));
        let stream = SchemeServiceStream {
            pending: VecDeque::new(),
            ready: Some(receive),
            cancelled: Arc::clone(&cancelled),
            signals: Vec::new(),
        };
        Ok((
            Self {
                config,
                count,
                window,
                deadline,
                source,
                cancelled,
                capacities,
                send,
            },
            stream,
        ))
    }
    async fn register(
        &self,
        dispatch: &InstanceDispatcher,
        mode: SchemeServiceSupplyAdmission,
    ) -> Result<Signals, InstanceError> {
        let mut signals = Vec::with_capacity(self.config.owners.len());
        for (&owner, capacity) in self.config.owners.iter().zip(&self.capacities) {
            let mut signal = Weak::new();
            let result = dispatch
                .register_producer(owner, Arc::clone(capacity), self.deadline, |registration| {
                    let producer = ServiceProducer::new(
                        dispatch,
                        registration,
                        self.config.maximum_active_per_owner,
                        Source::Pool(Arc::clone(&self.source)),
                        Arc::clone(&self.cancelled),
                        mode,
                    );
                    signal = Arc::downgrade(&producer.signal);
                    self.source
                        .signals
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .push(signal.clone());
                    Box::new(producer)
                })
                .await;
            if let Err(error) = result {
                self.cancelled.store(true, Ordering::Release);
                Source::Pool(Arc::clone(&self.source)).reject(&error);
                return Err(error);
            }
            signals.push(signal);
        }
        Ok(signals)
    }
    async fn publish_windows<I, M, P, F>(
        &self,
        dispatch: &InstanceDispatcher,
        messages: &mut I,
        driver: &mut InstanceClientProgressDriver<P>,
        registering: &mut InstanceClientProgressDriver<F>,
    ) -> Result<(), InstanceError>
    where
        I: ExactSizeIterator<Item = Result<M, InstanceError>>,
        M: SchemeServiceRequest<Output = R>,
        P: Future,
        F: Future,
    {
        let mut cursor = Cursor::default();
        while cursor.sequence < self.count {
            // A blocked later registration never prevents a bounded publication
            // for already installed owners. No extra task or per-item polling.
            registering
                .wait(driver.wait(async { self.publish_window(dispatch, messages, &mut cursor) }))
                .await?;
        }
        Ok(())
    }
    fn publish_window<I, M>(
        &self,
        dispatch: &InstanceDispatcher,
        messages: &mut I,
        cursor: &mut Cursor,
    ) -> Result<(), InstanceError>
    where
        I: ExactSizeIterator<Item = Result<M, InstanceError>>,
        M: SchemeServiceRequest<Output = R>,
    {
        if Instant::now() >= self.deadline {
            return Err(InstanceError::Timeout);
        }
        let size = self.window.min(self.count - cursor.sequence);
        let batch = messages
            .by_ref()
            .take(size)
            .map(|message| message.and_then(super::service_contract::Checked::new))
            .collect::<Result<Vec<_>, _>>()?;
        if batch.len() != size {
            return Err(InstanceError::InvalidConfig(
                "Rust preparation iterator length",
            ));
        }
        for message in &batch {
            cursor.work = cursor.work.checked_add(message.work_units()).ok_or(
                InstanceError::InvalidConfig("service producer work overflow"),
            )?;
        }
        if cursor.work > self.config.maximum_staged_work {
            return Err(InstanceError::InvalidConfig(
                "service producer staged work bound",
            ));
        }
        for &owner in &self.config.owners {
            dispatch.pool_capacity(owner, &batch, self.config.maximum_active_per_owner)?;
        }
        let (requests, _) = dispatch.stage_service_delivery_indexed(
            batch,
            SchemeServiceStreamConfig {
                owner: self.config.owners[0],
                maximum_active: self.config.maximum_active_per_owner,
                maximum_staged_work: self.config.maximum_staged_work,
                timeout: self.config.timeout,
            },
            &self.capacities[0],
            Some(&self.send),
            cursor.sequence,
        )?;
        self.source
            .publish(requests, &self.cancelled, self.deadline)?;
        cursor.sequence += size;
        Ok(())
    }
}
