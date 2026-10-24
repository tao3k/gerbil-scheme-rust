//! Explicit bounded owner-resident supply; Scheme still owns all execution.
use super::{AsyncInstanceTicket, InstanceDispatcher, InstanceError, SchemeServiceRequest};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
/// Independent staged work and accepted native concurrency are separate bounds.
#[derive(Clone, Copy, Debug)]
pub struct SchemeServiceStreamConfig {
    pub owner: usize,
    /// Maximum accepted requests from this stream, through native retirement.
    pub maximum_active: usize,
    /// Aggregate declared work in the supplied Rust messages, not a byte limit.
    pub maximum_staged_work: usize,
    /// Registration and every unadmitted request share this absolute deadline.
    pub timeout: Duration,
}
/// FIFO owned results plus cancellation of unadmitted independent work.
/// Dropping this handle cancels unadmitted work. Accepted Scheme work always drains.
/// A fully consumed handle retains no transport/image through its cancellation
/// signal. Unconsumed application outputs keep their ordinary resource ownership.
#[derive(Debug)]
pub struct SchemeServiceStream<R> {
    pub(super) pending: VecDeque<AsyncInstanceTicket<R>>,
    pub(super) ready: Option<super::service_ready::Receivers<R>>,
    pub(super) cancelled: Arc<AtomicBool>,
    pub(super) signals: Vec<Weak<super::service_stream_runner::Signal>>,
}
impl<R> SchemeServiceStream<R> {
    /// Reject staged work at the next admission boundary; committed entries drain.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.wake();
    }
    pub(super) fn wake(&self) {
        for signal in &self.signals {
            if let Some(signal) = signal.upgrade() {
                signal.wake();
            }
        }
    }
    /// Consume one FIFO delivery with a finite observation budget.
    /// # Errors
    /// A consumed ticket can report cancellation before admission, timeout, close,
    /// native/Scheme failure, or projection failure. Timing out this wait does
    /// not cancel other tickets or preempt the consumed ticket's accepted work.
    pub async fn next(&mut self, timeout: Duration) -> Option<Result<(R, Instant), InstanceError>> {
        Some(self.pending.pop_front()?.wait_completed(timeout).await)
    }
}
impl<R> Drop for SchemeServiceStream<R> {
    fn drop(&mut self) {
        self.cancel();
    }
}
impl InstanceDispatcher {
    /// Stage explicitly independent stateless requests on one fixed Scheme owner.
    /// Reuses the owner's existing single producer registration and slot/work
    /// semaphores. Every native entry reserves both budgets; the inbox is checked
    /// between entries. No Tokio executor runs on the owner. Input count cannot
    /// exceed native queue capacity; declared staged work and active native work
    /// have independent bounds. Arbitrary Rust payload bytes remain application
    /// owned; each constructed native payload/result retains existing byte bounds.
    /// Actor-affine or dependent requests must use ordinary owner-pinned APIs.
    /// Cancel/expiry/close rejects unadmitted work; committed entries retain roots and
    /// credits through projection. Commitment is the final capacity/owner/close
    /// fence, before construction. `PreparedCycle` may hold one Rust input and its
    /// reserved credits before that fence; it rechecks cancellation/deadline/close
    /// after ready-result retirement and creates no Scheme roots in preparation.
    /// Cancellation racing after the final fence cannot retract
    /// that entry. Registration stays occupied until all accepted work drains.
    /// # Errors
    /// Invalid counts/work/affinity, missing native service/work budget, exhausted
    /// registration deadline, closed owner, or a full native inbox.
    pub async fn submit_scheme_service_stream_on<M: SchemeServiceRequest>(
        &self,
        messages: Vec<M>,
        config: SchemeServiceStreamConfig,
    ) -> Result<SchemeServiceStream<M::Output>, InstanceError> {
        let deadline =
            Instant::now()
                .checked_add(config.timeout)
                .ok_or(InstanceError::InvalidConfig(
                    "service producer deadline overflow",
                ))?;
        if config.timeout.is_zero() {
            return Err(InstanceError::Timeout);
        }
        if messages.is_empty()
            || messages.len() > self.admission.queue_capacity
            || config.maximum_active == 0
        {
            return Err(InstanceError::InvalidConfig("service producer count bound"));
        }
        let capacity = self.producer_owner(config.owner)?;
        if capacity.work.is_none() || config.maximum_active > capacity.slot_limit {
            return Err(InstanceError::InvalidConfig(
                "service producer active/work bound",
            ));
        }
        let (requests, pending) = self.stage_service_requests(messages, config, &capacity)?;
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut signal = Weak::new();
        self.register_producer(config.owner, capacity, deadline, |registration| {
            let producer = super::service_stream_runner::ServiceProducer::new(
                self,
                registration,
                config.maximum_active,
                super::service_stream_source::Source::Owner(requests),
                Arc::clone(&cancelled),
                super::SchemeServiceSupplyAdmission::Queued,
            );
            signal = Arc::downgrade(&producer.signal);
            Box::new(producer)
        })
        .await?;
        Ok(SchemeServiceStream {
            pending,
            ready: None,
            cancelled,
            signals: vec![signal],
        })
    }
    pub(super) fn stage_service_requests<M: SchemeServiceRequest>(
        &self,
        messages: Vec<M>,
        config: SchemeServiceStreamConfig,
        capacity: &super::capacity::Capacity,
    ) -> Result<Staged<M::Output>, InstanceError> {
        self.stage_service_delivery(messages, config, capacity, None)
    }
    pub(super) fn stage_service_delivery<M: SchemeServiceRequest>(
        &self,
        messages: Vec<M>,
        config: SchemeServiceStreamConfig,
        capacity: &super::capacity::Capacity,
        ready: Option<&Arc<super::service_ready::Channels<M::Output>>>,
    ) -> Result<Staged<M::Output>, InstanceError> {
        let mut work = 0_usize;
        let mut requests = VecDeque::new();
        let mut pending = VecDeque::new();
        for (sequence, message) in messages.into_iter().enumerate() {
            let message = super::service_contract::Checked::new(message)?;
            if message.lane().is_some() {
                return Err(InstanceError::InvalidConfig(
                    "service producer requires independent stateless work",
                ));
            }
            self.validate_service_on(config.owner, &message)?;
            let units = message.work_units();
            capacity.units(units)?;
            work = work.checked_add(units).ok_or(InstanceError::InvalidConfig(
                "service producer work overflow",
            ))?;
            if work > config.maximum_staged_work {
                return Err(InstanceError::InvalidConfig(
                    "service producer staged work bound",
                ));
            }
            let trace = message.entry_trace();
            let short = message.short_service();
            let send = if let Some(channels) = ready {
                super::service_ready::Reply::Ready {
                    channels: Arc::clone(channels),
                    sequence,
                }
            } else {
                let (send, response) = tokio::sync::oneshot::channel();
                pending.push_back(AsyncInstanceTicket { response });
                send.into()
            };
            requests.push_back(super::Request {
                started: Arc::new(AtomicBool::new(false)),
                capacity: None,
                work_units: units,
                work: super::pool::Work::Service(Box::new(super::service::TypedJob {
                    message,
                    send,
                    submitted: std::cell::Cell::new(false),
                    lane: None,
                    short,
                    trace,
                })),
            });
        }
        Ok((requests, pending))
    }
}

type Staged<R> = (VecDeque<super::Request>, VecDeque<AsyncInstanceTicket<R>>);
