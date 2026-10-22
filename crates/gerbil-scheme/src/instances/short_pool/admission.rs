//! Configuration and cancel-safe publication under the pool close fence.
use super::{
    Destination, PendingGuard, Port, Queue, Queued, SchemeShortPool, SchemeShortPoolConfig, Source,
};
use crate::instances::{
    AsyncInstanceTicket, InstanceDispatcher, InstanceError, Request, RoutedSchemeTicket,
    SchemeRouteClass, SchemeServiceExecution, SchemeServiceRequest, pool::Work, service::TypedJob,
    service_contract::Checked,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{Semaphore, oneshot};
impl InstanceDispatcher {
    /// Clone the explicitly configured shared queue; absence preserves the old API.
    #[must_use]
    pub fn scheme_short_pool(&self) -> Option<SchemeShortPool> {
        self.admission
            .short_pool
            .get()
            .map(|source| SchemeShortPool {
                source: Arc::clone(source),
            })
    }

    /// Configure one shared late-binding short queue before submitting work.
    /// Existing producers retain their separate registration and native budgets.
    /// # Errors
    /// Rejects duplicate configuration, unsupported owners or invalid bounds.
    pub fn configure_scheme_short_pool(
        &self,
        config: SchemeShortPoolConfig,
    ) -> Result<SchemeShortPool, InstanceError> {
        let senders = self
            .admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let invalid = || InstanceError::InvalidConfig("short pool configuration");
        if self.admission.short_pool.get().is_some()
            || config.owners.is_empty()
            || config.maximum_pending == 0
            || config.maximum_pending > self.admission.queue_capacity
            || config.maximum_active_per_owner == 0
        {
            return Err(invalid());
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut destinations = Vec::new();
        let mut wakes = Vec::new();
        for owner in config.owners {
            let target = senders.owners.get(owner).ok_or_else(invalid)?;
            let capacity = target.capacity.as_ref().ok_or_else(invalid)?;
            let wake = target.wake.as_ref().ok_or_else(invalid)?;
            if !seen.insert(owner)
                || wake.short_pool.get().is_some()
                || wake.closed.load(Ordering::Acquire)
                || wake.failed.load(Ordering::Acquire)
                || wake.service_version.load(Ordering::Acquire) < 2
                || capacity.work.is_none()
                || config.maximum_active_per_owner > capacity.slot_limit
                || config.maximum_active_per_owner > capacity.work_limit
            {
                return Err(invalid());
            }
            destinations.push(Destination {
                owner,
                capacity: Arc::clone(capacity),
                active: Arc::new(Semaphore::new(config.maximum_active_per_owner)),
                wake: Arc::downgrade(wake),
                waiting: AtomicBool::new(false),
            });
            wakes.push((owner, Arc::clone(wake)));
        }
        let source = Arc::new(Source {
            admission: Arc::downgrade(&self.admission),
            destinations,
            queue: std::sync::Mutex::new(Queue::default()),
            selective: AtomicBool::new(false),
            pending_limit: config.maximum_pending,
            pending: Arc::new(Semaphore::new(config.maximum_pending)),
            length: AtomicUsize::new(0),
            next: AtomicUsize::new(0),
        });
        // Nothing is queued until all ports have been installed under this fence.
        install(&self.admission, &source, wakes)?;
        drop(senders);
        source.owner_closed();
        Ok(SchemeShortPool { source })
    }
}
impl SchemeShortPool {
    /// Wait for a running owner to claim this Rust payload and native credits.
    /// Cancellation/deadline before claim removes the queued payload. After
    /// claim, ticket timeout/drop never cancels Scheme execution or frees roots.
    /// # Errors
    /// Invalid short declaration, finite admission timeout or closed service.
    pub async fn submit<M: SchemeServiceRequest>(
        &self,
        message: M,
        timeout: Duration,
    ) -> Result<RoutedSchemeTicket<M::Output>, InstanceError> {
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or(InstanceError::InvalidConfig("short admission deadline"))?;
        if timeout.is_zero() {
            return Err(InstanceError::Timeout);
        }
        let message = Checked::new(message)?;
        if message.execution != SchemeServiceExecution::Short {
            return Err(InstanceError::InvalidConfig(
                "short pool requires short service",
            ));
        }
        let pending = tokio::time::timeout_at(
            tokio::time::Instant::from_std(deadline),
            Arc::clone(&self.source.pending).acquire_owned(),
        )
        .await
        .map_err(|_| InstanceError::Timeout)?
        .map_err(|_| InstanceError::Closed)?;
        let id = self.source.next.fetch_add(1, Ordering::Relaxed);
        let prepared = prepare(message, deadline, pending, id);
        self.source.publish(prepared.queued)?;
        let _guard = PendingGuard {
            source: Arc::clone(&self.source),
            id,
        };
        let owner =
            tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), prepared.placement)
                .await
                .map_err(|_| InstanceError::Timeout)?
                .map_err(|_| InstanceError::Closed)??;
        Ok(RoutedSchemeTicket {
            owner,
            ticket: prepared.ticket,
            route_class: SchemeRouteClass::Short,
            execution_class: SchemeServiceExecution::Short,
        })
    }
}
impl Source {
    pub(super) fn publish(&self, queued: Queued) -> Result<(), InstanceError> {
        let admission = self.admission.upgrade().ok_or(InstanceError::Closed)?;
        let senders = admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !admission.accepting.load(Ordering::Acquire) || self.pending.is_closed() {
            drop(senders);
            return Err(InstanceError::Closed);
        }
        let mut queue = self
            .queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Owner failure can close this source without the pool table fence.
        // Recheck under the queue lock: close marks pending closed before drain.
        if self.pending.is_closed() {
            drop(queue);
            drop(senders);
            return Err(InstanceError::Closed);
        }
        queue.requests.push_back(queued);
        self.length.store(queue.requests.len(), Ordering::Release);
        drop(queue);
        drop(senders);
        self.wake();
        Ok(())
    }
}

struct Prepared<R> {
    queued: Queued,
    placement: oneshot::Receiver<Result<usize, InstanceError>>,
    ticket: AsyncInstanceTicket<R>,
}
fn prepare<M: SchemeServiceRequest>(
    message: M,
    deadline: Instant,
    pending: tokio::sync::OwnedSemaphorePermit,
    id: usize,
) -> Prepared<M::Output> {
    let trace = message.entry_trace();
    let (assigned, placement) = oneshot::channel();
    let (send, response) = oneshot::channel();
    let queued = Queued {
        id,
        deadline,
        assigned,
        _pending: pending,
        request: Request {
            started: Arc::new(AtomicBool::new(false)),
            capacity: None,
            work_units: 1,
            work: Work::Service(Box::new(TypedJob {
                message,
                send,
                submitted: false,
                lane: None,
                short: true,
                trace,
            })),
        },
    };
    Prepared {
        queued,
        placement,
        ticket: AsyncInstanceTicket { response },
    }
}

fn install(
    admission: &crate::instances::Admission,
    source: &Arc<Source>,
    wakes: Vec<(usize, Arc<crate::instances::service_wake::Wake>)>,
) -> Result<(), InstanceError> {
    let invalid = || InstanceError::InvalidConfig("short pool registration");
    admission
        .short_pool
        .set(Arc::clone(source))
        .map_err(|_| invalid())?;
    admission
        .capacity_changed
        .get_or_init(crate::instances::capacity_event::CapacityEvents::new)
        .short_pool
        .set(Arc::downgrade(source))
        .map_err(|_| invalid())?;
    for (index, (owner, wake)) in wakes.into_iter().enumerate() {
        wake.short_pool
            .set(Port {
                owner,
                index,
                source: Arc::clone(source),
            })
            .map_err(|_| invalid())?;
    }
    Ok(())
}
