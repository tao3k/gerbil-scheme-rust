//! A bounded open source for the existing resident owner producer.
use super::{
    AsyncInstanceTicket, InstanceDispatcher, InstanceError, NativeRoot, NativeSession,
    SchemeServiceRequest, SchemeServiceStreamConfig, SchemeServiceSupplyAdmission,
    service_stream_runner::ServiceProducer,
    service_stream_source::{Shared, Source},
};
use std::{
    marker::PhantomData,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Explicit opt-in sustained short service on one fixed owner. The outstanding
/// bound includes queued input, native work and unread results. No batch merging,
/// new executor, native limit or default change is involved. Stateful Actor lanes
/// must keep using the owner-pinned API.
pub struct SchemeServiceContinuous<M: SchemeServiceRequest> {
    dispatch: InstanceDispatcher,
    source: Arc<Shared>,
    cancelled: Arc<AtomicBool>,
    outstanding: Arc<Semaphore>,
    capacity: Arc<super::capacity::Capacity>,
    config: SchemeServiceStreamConfig,
    deadline: Instant,
    marker: PhantomData<fn() -> M>,
}
/// One independently timed result. Dropping it cancels only pre-entry work;
/// accepted Scheme work retains native credits and roots until retirement.
pub struct SchemeServiceContinuousTicket<R> {
    ticket: Option<AsyncInstanceTicket<R>>,
    lease: Arc<ContinuousLease>,
    source: Weak<Shared>,
    started: Arc<AtomicBool>,
}
// A client timeout cannot free a window while its canceled input is still
// staged. Reuse the existing shared cancellation allocation for joint custody.
struct ContinuousLease {
    cancelled: AtomicBool,
    _permit: OwnedSemaphorePermit,
}
impl<R> SchemeServiceContinuousTicket<R> {
    /// Whether the owner has published an unread result.
    /// This observation does not consume the result or release window credit.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.ticket
            .as_ref()
            .is_none_or(|ticket| !ticket.response.is_empty())
    }

    /// Observe the original result without controlling native retirement.
    /// # Errors
    /// Observation timeout or the request's construction/execution/projection error.
    pub async fn wait_completed(
        mut self,
        timeout: Duration,
    ) -> Result<(R, Instant), InstanceError> {
        self.ticket
            .take()
            .ok_or(InstanceError::Closed)?
            .wait_completed(timeout)
            .await
    }
}
impl<R> Drop for SchemeServiceContinuousTicket<R> {
    fn drop(&mut self) {
        self.lease.cancelled.store(true, Ordering::Release);
        if !self.started.load(Ordering::Acquire)
            && let Some(source) = self.source.upgrade()
        {
            source.withdraw(&self.started);
        }
    }
}
impl InstanceDispatcher {
    /// Register one persistent producer before arrivals begin. `maximum_staged_work`
    /// is the total outstanding one-unit requests, including unread Rust results;
    /// it cannot exceed the existing pool queue bound. Native concurrency remains
    /// independently bounded by `maximum_active` and configured slot/work credits.
    /// The finite source lifetime is distinct from each request's absolute deadline.
    /// # Errors
    /// Invalid bounds, unavailable producer, closed owner, or registration timeout.
    pub async fn open_scheme_service_continuous<M: SchemeServiceRequest>(
        &self,
        config: SchemeServiceStreamConfig,
    ) -> Result<SchemeServiceContinuous<M>, InstanceError> {
        let deadline =
            Instant::now()
                .checked_add(config.timeout)
                .ok_or(InstanceError::InvalidConfig(
                    "continuous source deadline overflow",
                ))?;
        if config.timeout.is_zero() {
            return Err(InstanceError::Timeout);
        }
        let capacity = self.producer_owner(config.owner)?;
        if config.maximum_staged_work == 0
            || config.maximum_staged_work > self.admission.queue_capacity
            || config.maximum_active == 0
            || config.maximum_active > capacity.slot_limit
            || capacity.work.is_none()
        {
            return Err(InstanceError::InvalidConfig("continuous source bounds"));
        }
        let source = Shared::opening();
        let cancelled = Arc::new(AtomicBool::new(false));
        self.register_producer(
            config.owner,
            Arc::clone(&capacity),
            deadline,
            |registration| {
                let producer = ServiceProducer::new(
                    self,
                    registration,
                    config.maximum_active,
                    Source::Pool(Arc::clone(&source)),
                    Arc::clone(&cancelled),
                    SchemeServiceSupplyAdmission::OwnerLocal,
                );
                source
                    .signals
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(Arc::downgrade(&producer.signal));
                Box::new(producer)
            },
        )
        .await?;
        Ok(SchemeServiceContinuous {
            dispatch: self.clone(),
            source,
            cancelled,
            outstanding: Arc::new(Semaphore::new(config.maximum_staged_work)),
            capacity,
            config,
            deadline,
            marker: PhantomData,
        })
    }
}
impl<M: SchemeServiceRequest> SchemeServiceContinuous<M> {
    /// Attempt one real library admission, constructing owned Rust input only
    /// after a window credit is obtained. `QueueFull` is an explicit library
    /// rejection, never a successful arrival. The closure must be bounded and
    /// must not enter Scheme. Every accepted input retains its own result/expiry.
    /// # Errors
    /// Full outstanding window, closed/sealed source, invalid short request,
    /// preparation error or absolute expiry. No native roots exist at this point.
    pub fn try_submit_with(
        &self,
        deadline: Instant,
        prepare: impl FnOnce() -> Result<M, InstanceError>,
    ) -> Result<SchemeServiceContinuousTicket<M::Output>, InstanceError> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(InstanceError::Cancelled);
        }
        if Instant::now() >= deadline.min(self.deadline) {
            return Err(InstanceError::Timeout);
        }
        let permit = Arc::clone(&self.outstanding)
            .try_acquire_owned()
            .map_err(|e| match e {
                tokio::sync::TryAcquireError::Closed => InstanceError::Closed,
                tokio::sync::TryAcquireError::NoPermits => InstanceError::QueueFull,
            })?;
        self.submit_permitted(deadline, prepare, permit)
    }
    /// Wait for an existing outstanding-window credit without shedding on a
    /// full window. No input is prepared or rooted while waiting. The original
    /// absolute deadline includes admission wait; it is never restarted.
    /// Results must be consumed independently: unread results retain this credit.
    /// Dropping this future cancels only its admission wait.
    /// # Errors
    /// Original expiry, cancellation, source close or preparation/native-contract errors.
    pub async fn submit_with(
        &self,
        deadline: Instant,
        prepare: impl FnOnce() -> Result<M, InstanceError>,
    ) -> Result<SchemeServiceContinuousTicket<M::Output>, InstanceError> {
        self.check_submission(deadline)?;
        let permit = tokio::time::timeout_at(
            deadline.min(self.deadline).into(),
            Arc::clone(&self.outstanding).acquire_owned(),
        )
        .await
        .map_err(|_| InstanceError::Timeout)?
        .map_err(|_| {
            if self.cancelled.load(Ordering::Acquire) {
                InstanceError::Cancelled
            } else {
                InstanceError::Closed
            }
        })?;
        self.check_submission(deadline)?;
        self.submit_permitted(deadline, prepare, permit)
    }
    fn check_submission(&self, deadline: Instant) -> Result<(), InstanceError> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(InstanceError::Cancelled);
        }
        if Instant::now() >= deadline.min(self.deadline) {
            return Err(InstanceError::Timeout);
        }
        if self.outstanding.is_closed() {
            return Err(InstanceError::Closed);
        }
        Ok(())
    }
    fn submit_permitted(
        &self,
        deadline: Instant,
        prepare: impl FnOnce() -> Result<M, InstanceError>,
        permit: OwnedSemaphorePermit,
    ) -> Result<SchemeServiceContinuousTicket<M::Output>, InstanceError> {
        let message = super::service_contract::Checked::new(prepare()?)?;

        if !message.short_service() || message.lane().is_some() || message.work_units() != 1 {
            return Err(InstanceError::InvalidConfig(
                "continuous source requires stateless short requests",
            ));
        }
        let lease = Arc::new(ContinuousLease {
            cancelled: AtomicBool::new(false),
            _permit: permit,
        });
        let message = Expiring {
            message,
            deadline: deadline.min(self.deadline),
            lease: Arc::clone(&lease),
        };
        let (requests, mut pending) =
            self.dispatch
                .stage_service_requests(vec![message], self.config, &self.capacity)?;
        let started = Arc::clone(
            &requests
                .front()
                .ok_or(InstanceError::InvalidConfig("missing staged request"))?
                .started,
        );
        self.source.publish_bounded(
            requests,
            &self.cancelled,
            deadline.min(self.deadline),
            self.config.maximum_staged_work,
        )?;
        Ok(SchemeServiceContinuousTicket {
            ticket: pending.pop_front(),
            lease,
            source: Arc::downgrade(&self.source),
            started,
        })
    }
    /// Seal input while allowing queued and entered work to drain normally.
    pub fn seal(&self) {
        self.source.seal();
        self.outstanding.close();
    }
    /// Reject unentered input, retaining accepted native work until retirement.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.outstanding.close();
        Source::Pool(Arc::clone(&self.source)).reject(&InstanceError::Cancelled);
    }
}
impl<M: SchemeServiceRequest> Drop for SchemeServiceContinuous<M> {
    fn drop(&mut self) {
        self.cancel();
    }
}
struct Expiring<M> {
    message: M,
    deadline: Instant,
    lease: Arc<ContinuousLease>,
}
impl<M> Expiring<M> {
    fn check(&self) -> Result<(), InstanceError> {
        if self.lease.cancelled.load(Ordering::Acquire) {
            Err(InstanceError::Cancelled)
        } else if Instant::now() >= self.deadline {
            Err(InstanceError::Timeout)
        } else {
            Ok(())
        }
    }
}
impl<M: SchemeServiceRequest> SchemeServiceRequest for Expiring<M> {
    type Output = M::Output;
    fn work_units(&self) -> usize {
        self.message.work_units()
    }
    fn short_service(&self) -> bool {
        self.message.short_service()
    }
    fn entry_trace(&self) -> Option<Arc<super::SchemeServiceEntryTrace>> {
        self.message.entry_trace()
    }
    fn completion_trace(&self) -> Option<Arc<super::SchemeCompletionTrace>> {
        self.message.completion_trace()
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.check()?;
        self.message.construct(session)
    }
    fn construct_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &super::SchemeServiceInput<'_>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.check()?;
        self.message.construct_service(session, input)
    }
    fn enter_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &super::SchemeServiceInput<'_>,
    ) -> Result<super::SchemeServiceSubmission<'vm>, InstanceError> {
        self.check()?;
        self.message.enter_service(session, input)
    }
    fn project_service(
        self,
        session: &NativeSession<'_>,
        output: &super::SchemeServiceOutput<'_>,
    ) -> Result<Self::Output, InstanceError> {
        self.message.project_service(session, output)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        self.message.project(session, output)
    }
}
impl<M: SchemeServiceRequest> std::fmt::Debug for SchemeServiceContinuous<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemeServiceContinuous")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}
impl<R> std::fmt::Debug for SchemeServiceContinuousTicket<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemeServiceContinuousTicket")
            .finish_non_exhaustive()
    }
}
