//! Bounded unassigned Rust payloads; only a fixed owner can claim native work.
#[path = "admission.rs"]
mod admission;
#[path = "claim.rs"]
mod claim;
#[path = "notification.rs"]
mod notification;
use crate::instances::{Admission, Capacity, InstanceError, Request, service_wake::Wake};
use std::{
    collections::{BTreeSet, VecDeque},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Instant,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, oneshot};
/// Explicit short-request supply bounds. Configuration freezes native budgets
/// until pool shutdown. Clones share one queue and per-owner active quota.
#[derive(Clone, Debug)]
pub struct SchemeShortPoolConfig {
    pub owners: Vec<usize>,
    pub maximum_pending: usize,
    pub maximum_active_per_owner: usize,
}
/// Explicit shared-short notification policy; existing broadcast is unchanged.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SchemeShortWake {
    #[default]
    Broadcast,
    /// FIFO wait registration and bounded, nonexclusive wake obligations.
    Selective,
}
#[derive(Debug, Default)]
pub(super) struct Queue {
    requests: VecDeque<Queued>,
    waiters: VecDeque<usize>,
    notified: BTreeSet<usize>,
}
/// Owner binding occurs when a running owner claims a request, before construct.
/// This queue accepts only stateless, one-unit Scheme short-service requests.
#[derive(Clone, Debug)]
pub struct SchemeShortPool {
    source: Arc<Source>,
}
#[derive(Debug)]
pub(super) struct Destination {
    owner: usize,
    capacity: Arc<Capacity>,
    active: Arc<Semaphore>,
    wake: Weak<Wake>,
    waiting: AtomicBool,
}
pub(super) struct Queued {
    id: usize,
    request: Request,
    assigned: oneshot::Sender<Result<usize, InstanceError>>,
    deadline: Instant,
    _pending: OwnedSemaphorePermit,
}
impl std::fmt::Debug for Queued {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Queued")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}
#[derive(Debug)]
pub(crate) struct Source {
    admission: Weak<Admission>,
    destinations: Vec<Destination>,
    queue: Mutex<Queue>,
    selective: AtomicBool,
    pending_limit: usize,
    pending: Arc<Semaphore>,
    length: AtomicUsize,
    next: AtomicUsize,
}
#[derive(Debug)]
pub(crate) struct Port {
    pub owner: usize,
    index: usize,
    pub source: Arc<Source>,
}
impl Source {
    pub(crate) fn capacity_available(&self) {
        if self.length.load(Ordering::Acquire) != 0 {
            self.wake();
        }
    }

    fn broadcast(&self) {
        for target in &self.destinations {
            if let Some(wake) = target
                .wake
                .upgrade()
                .filter(|w| !w.closed.load(Ordering::Acquire) && !w.failed.load(Ordering::Acquire))
            {
                wake.notify_for(true);
            }
        }
    }
    fn cancel(&self, id: usize) {
        let removed = {
            let mut queue = self
                .queue
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let removed = queue
                .requests
                .iter()
                .position(|r| r.id == id)
                .and_then(|i| queue.requests.remove(i));
            self.length.store(queue.requests.len(), Ordering::Release);
            if queue.requests.is_empty() {
                queue.notified.clear();
            }
            removed
        };
        drop(removed);
        self.capacity_available();
    }
    pub(crate) fn close(&self) {
        self.pending.close();
        let queued = {
            let mut queue = self
                .queue
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            self.length.store(0, Ordering::Release);
            queue.waiters.clear();
            queue.notified.clear();
            for target in &self.destinations {
                target.waiting.store(false, Ordering::Release);
            }
            std::mem::take(&mut queue.requests)
        };
        for request in queued {
            reject(request, InstanceError::Closed);
        }
    }
    pub(crate) fn owner_closed(&self) {
        self.remove_closed_waiters();
        if self.destinations.iter().all(|t| {
            t.wake.upgrade().is_none_or(|w| {
                w.closed.load(Ordering::Acquire) || w.failed.load(Ordering::Acquire)
            })
        }) {
            self.close();
        } else {
            self.capacity_available();
        }
    }
}
fn reject(queued: Queued, error: InstanceError) {
    let _ = queued.assigned.send(Err(error.clone()));
    if let crate::instances::pool::Work::Service(job) = queued.request.work {
        job.fail(error);
    }
}
struct PendingGuard {
    source: Arc<Source>,
    id: usize,
}
impl Drop for PendingGuard {
    fn drop(&mut self) {
        self.source.cancel(self.id);
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/short_pool_close.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/unit/short_pool_notification.rs"]
mod notification_tests;
