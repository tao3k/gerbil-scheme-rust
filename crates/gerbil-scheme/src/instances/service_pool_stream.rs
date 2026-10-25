//! A bounded pool source feeds existing fixed-owner native service producers.
use super::{
    InstanceDispatcher, InstanceError, SchemeServiceRequest, SchemeServiceStream,
    SchemeServiceStreamConfig, SchemeServiceSupplyAdmission,
    service_stream_runner::ServiceProducer,
    service_stream_source::{Shared, Source},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
/// Explicit opt-in pool supply; limits do not change dispatcher defaults.
#[derive(Clone, Debug)]
pub struct SchemeServicePoolStreamConfig {
    /// Unique fixed owners. Registration uses sorted order to avoid lock cycles.
    pub owners: Vec<usize>,
    /// Native requests per owner, retained through projection/root retirement.
    pub maximum_active_per_owner: usize,
    /// Aggregate declared work across the entire uncommitted input, not per owner.
    pub maximum_staged_work: usize,
    /// One absolute deadline covers registration and every uncommitted request.
    pub timeout: Duration,
}
impl InstanceDispatcher {
    /// Supply independent native-typed tasks to whichever owner makes progress.
    /// No message has a fixed owner until that owner's active/work/slot credits
    /// and final lifecycle fence succeed. Accepted Scheme state never migrates.
    /// Input count is bounded by the pool's existing queue capacity. Payload bytes
    /// remain application-owned; this is a count/declared-work bound.
    /// All registrations complete before supply starts. Dropping registration or
    /// the stream cancels only uncommitted work. Any participating owner's close
    /// rejects remaining shared work; already committed work drains normally.
    /// This API does not promise a scheduler time slice or global FIFO completion.
    /// Delivery positions retain input order regardless of the execution owner.
    /// # Errors
    /// Empty/duplicate owners, invalid bounds, affinity, missing native service,
    /// incompatible work capacity, registration timeout/close or full inbox.
    pub async fn submit_scheme_service_pool_stream<M: SchemeServiceRequest>(
        &self,
        messages: Vec<M>,
        config: SchemeServicePoolStreamConfig,
    ) -> Result<SchemeServiceStream<M::Output>, InstanceError> {
        self.submit_scheme_service_pool_stream_with_admission(
            messages,
            config,
            SchemeServiceSupplyAdmission::Queued,
        )
        .await
    }
    /// Explicit reservation policy for bounded pool supply. All lifecycle,
    /// capacity, affinity and Scheme execution contracts match the ordinary API.
    /// `OwnerLocal` avoids the per-job wrapper by retaining active credit in the
    /// native retirement guard. Ready credits need no temporary future; genuine
    /// contention retains the existing FIFO waiter and wake protocol. A held
    /// producer registration stabilizes capacity identity; cached admission is
    /// invalidated inside the pool close fence. This mode is experimental.
    /// `RetirementRefill` additionally refills after projecting the full completion
    /// snapshot, bounded by retired count and active limit. Shared short work
    /// takes priority. Each attempt is bounded; Scheme owns execution. Existing
    /// active/native limits and immediate credit publication remain unchanged.
    /// `ProgressFair` offers work to a less loaded peer only when local capacity
    /// is exhausted. Mailbox receipt is a revocable Rust claim: a ready owner
    /// may withdraw it and immediately clear the dedicated target offer slot.
    /// Offers hold no native credit and consume no ordinary inbox space. Short
    /// claims precede offer entry; the running owner acquires native credits. The first
    /// native entry fence, before input construction, fixes ownership. Cancel,
    /// deadline and close reject unentered offers; entered Scheme work drains.
    /// Withdrawal clears the slot immediately; an old claim cannot clear its
    /// replacement or execute a withdrawn payload. Source signals follow price
    /// changes and terminal depletion rather than every offer transition.
    /// It requires explicitly enabled native progress on every configured owner;
    /// it does not use retirement refill or change native execution budgets.
    /// # Errors
    /// The same validation, registration, expiry and close errors as ordinary supply.
    pub async fn submit_scheme_service_pool_stream_with_admission<M: SchemeServiceRequest>(
        &self,
        messages: Vec<M>,
        config: SchemeServicePoolStreamConfig,
        mode: SchemeServiceSupplyAdmission,
    ) -> Result<SchemeServiceStream<M::Output>, InstanceError> {
        self.pool_stream_delivery(messages, config, mode, false, None)
            .await
    }
    /// Deliver results in ready-owner rotation with stable input indices, avoiding
    /// cross-owner FIFO head-of-line blocking. Reuses the same bounded input,
    /// native credits, revocable offers and producer registrations. Result queues
    /// contain at most one output per original input, no native root credit.
    /// Dropping the handle cancels staged work; accepted Scheme work still drains.
    /// # Errors
    /// The same validation, registration and close errors as FIFO pool supply.
    pub async fn submit_scheme_service_pool_ready_stream<M: SchemeServiceRequest>(
        &self,
        messages: Vec<M>,
        config: SchemeServicePoolStreamConfig,
        mode: SchemeServiceSupplyAdmission,
    ) -> Result<super::SchemeServiceReadyStream<M::Output>, InstanceError> {
        self.pool_stream_delivery(messages, config, mode, true, None)
            .await
            .map(super::SchemeServiceReadyStream)
    }
    /// Prepare lazy, root-free Rust inputs as bounded client service turns, then
    /// register the existing pool producers. One deadline covers preparation,
    /// registration and uncommitted input. Count uses the existing pool inbox
    /// bound; declared work is validated by the existing staging transaction.
    /// Each iterator step must have bounded CPU/bytes and must not enter Scheme.
    /// The driver services signalled client progress between steps. Owners keep
    /// running independently; client consumption never grants native capacity.
    /// # Errors
    /// Preparation error/expiry and the ordinary ready-stream validation errors.
    pub async fn prepare_scheme_service_pool_ready_stream<I, M, P>(
        &self,
        messages: I,
        config: SchemeServicePoolStreamConfig,
        mode: SchemeServiceSupplyAdmission,
        driver: &mut super::InstanceClientProgressDriver<P>,
    ) -> Result<super::SchemeServiceReadyStream<M::Output>, InstanceError>
    where
        I: ExactSizeIterator<Item = Result<M, InstanceError>>,
        M: SchemeServiceRequest,
        P: std::future::Future,
    {
        let deadline =
            Instant::now()
                .checked_add(config.timeout)
                .ok_or(InstanceError::InvalidConfig(
                    "service pool deadline overflow",
                ))?;
        let messages = driver
            .prepare_bounded(messages, self.admission.queue_capacity, deadline)
            .await?;
        driver
            .wait(self.pool_stream_delivery(messages, config, mode, true, Some(deadline)))
            .await
            .map(super::SchemeServiceReadyStream)
    }
    async fn pool_stream_delivery<M: SchemeServiceRequest>(
        &self,
        messages: Vec<M>,
        mut config: SchemeServicePoolStreamConfig,
        mode: SchemeServiceSupplyAdmission,
        ready: bool,
        deadline: Option<Instant>,
    ) -> Result<SchemeServiceStream<M::Output>, InstanceError> {
        if mode == SchemeServiceSupplyAdmission::ProgressFair {
            self.require_progress_owners(&config.owners)?;
        }
        let (source, mut stream, capacities, deadline) =
            self.prepare_pool(messages, &mut config, ready, deadline)?;
        if mode == SchemeServiceSupplyAdmission::ProgressFair {
            source
                .fair
                .set(super::Fair::default())
                .map_err(|_| InstanceError::InvalidConfig("duplicate progress source"))?;
        }
        for (owner, capacity) in config.owners.into_iter().zip(capacities) {
            self.register_producer(owner, capacity, deadline, |registration| {
                let producer = ServiceProducer::new(
                    self,
                    registration,
                    config.maximum_active_per_owner,
                    Source::Pool(Arc::clone(&source)),
                    Arc::clone(&stream.cancelled),
                    mode,
                );
                stream.signals.push(Arc::downgrade(&producer.signal));
                source
                    .signals
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(Arc::downgrade(&producer.signal));
                Box::new(producer)
            })
            .await?;
        }
        source.ready.store(true, Ordering::Release);
        stream.wake();
        Ok(stream)
    }
    pub(super) fn pool_capacity<M: SchemeServiceRequest>(
        &self,
        owner: usize,
        messages: &[M],
        maximum: usize,
    ) -> Result<Arc<super::capacity::Capacity>, InstanceError> {
        let capacity = self.producer_owner(owner)?;
        if capacity.work.is_none() || maximum > capacity.slot_limit {
            return Err(InstanceError::InvalidConfig(
                "service pool active/work bound",
            ));
        }
        messages.iter().try_for_each(|message| {
            self.validate_service_on(owner, message)?;
            capacity.units(message.work_units()).map(|_| ())
        })?;
        Ok(capacity)
    }
    fn prepare_pool<M: SchemeServiceRequest>(
        &self,
        messages: Vec<M>,
        config: &mut SchemeServicePoolStreamConfig,
        ready: bool,
        deadline: Option<Instant>,
    ) -> Result<Prepared<M::Output>, InstanceError> {
        let deadline =
            match deadline {
                Some(deadline) => deadline,
                None => Instant::now().checked_add(config.timeout).ok_or(
                    InstanceError::InvalidConfig("service pool deadline overflow"),
                )?,
            };
        if Instant::now() >= deadline {
            return Err(InstanceError::Timeout);
        }
        config.owners.sort_unstable();
        if config.owners.is_empty()
            || config.owners.windows(2).any(|w| w[0] == w[1])
            || messages.is_empty()
            || messages.len() > self.admission.queue_capacity
            || config.maximum_active_per_owner == 0
        {
            return Err(InstanceError::InvalidConfig(
                "service pool count/owner bound",
            ));
        }
        if config.timeout.is_zero() {
            return Err(InstanceError::Timeout);
        }
        // Freeze all application declarations once, before any registration.
        let messages: Vec<_> = messages
            .into_iter()
            .map(super::service_contract::Checked::new)
            .collect::<Result<_, _>>()?;
        let capacities = config
            .owners
            .iter()
            .map(|&owner| self.pool_capacity(owner, &messages, config.maximum_active_per_owner))
            .collect::<Result<Vec<_>, _>>()?;
        let delivery =
            ready.then(|| super::service_ready::channels(&config.owners, messages.len()));
        let (requests, pending) = self.stage_service_delivery(
            messages,
            SchemeServiceStreamConfig {
                owner: config.owners[0],
                maximum_active: config.maximum_active_per_owner,
                maximum_staged_work: config.maximum_staged_work,
                timeout: config.timeout,
            },
            &capacities[0],
            delivery.as_ref().map(|(send, _)| send),
        )?;
        let source = Shared::new(requests);
        let stream = SchemeServiceStream {
            pending,
            ready: delivery.map(|(_, receive)| receive),
            cancelled: Arc::new(AtomicBool::new(false)),
            signals: Vec::new(),
        };
        Ok((source, stream, capacities, deadline))
    }
}
type Prepared<R> = (
    Arc<Shared>,
    SchemeServiceStream<R>,
    Vec<Arc<super::capacity::Capacity>>,
    Instant,
);
