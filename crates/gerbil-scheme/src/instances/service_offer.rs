//! Mailbox notifications are revocable; only the native entry fence binds work.
use super::{InstanceError, PoolSource, Request, pool::Work, service_wake::Wake};
use std::{
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
pub(super) struct Offer {
    request: Mutex<Option<Request>>,
    source: Weak<PoolSource>,
    target: Weak<Wake>,
    accepting: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
    deadline: Instant,
    pub(super) units: usize,
}
/// Owner receipt conveys a chance to enter, not ownership of Scheme state.
pub(super) struct Claim<'a> {
    offer: &'a Offer,
    wake: &'a Wake,
}
impl Claim<'_> {
    pub(super) fn enter(self) -> Option<Request> {
        self.offer.enter(self.wake)
    }
}
impl Offer {
    pub(super) fn claim<'a>(&'a self, wake: &'a Wake) -> Claim<'a> {
        Claim { offer: self, wake }
    }
    pub(super) fn new(
        request: Request,
        source: &Arc<PoolSource>,
        target: &Arc<Wake>,
        accepting: Arc<AtomicBool>,
        cancelled: Arc<AtomicBool>,
        deadline: Instant,
    ) -> Arc<Self> {
        Arc::new(Self {
            units: request.work_units,
            request: Mutex::new(Some(request)),
            source: Arc::downgrade(source),
            target: Arc::downgrade(target),
            accepting,
            cancelled,
            deadline,
        })
    }
    pub(super) fn pending(&self) -> bool {
        self.request
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    }
    /// Withdrawal and native entry compete for the single Rust payload. A stale
    /// mailbox marker owns neither the request nor any capacity credit.
    pub(super) fn withdraw(&self) -> Option<Request> {
        self.request
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
    pub(super) fn reclaim(&self) -> Option<Request> {
        let request = self.withdraw();
        if request.is_some() {
            if let Some(target) = self.target.upgrade() {
                target
                    .progress
                    .peer_withdrawals
                    .fetch_add(1, Ordering::AcqRel);
            }
        }
        request
    }
    fn enter(&self, wake: &Wake) -> Option<Request> {
        let mut guard = self
            .request
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let error = if !crate::instance_backend::images_ready()
            || !self.accepting.load(Ordering::Acquire)
            || wake.closed.load(Ordering::Acquire)
            || wake.failed.load(Ordering::Acquire)
            || self
                .target
                .upgrade()
                .is_none_or(|w| !std::ptr::eq(w.as_ref(), wake))
        {
            Some(InstanceError::Closed)
        } else if self.cancelled.load(Ordering::Acquire) {
            Some(InstanceError::Cancelled)
        } else if Instant::now() >= self.deadline {
            Some(InstanceError::Timeout)
        } else {
            None
        };
        // This take is the irrevocable entry fence, before M::construct can
        // allocate native roots or call arbitrary Scheme. No lock crosses FFI.
        let request = guard.take();
        drop(guard);
        if let Some(error) = error {
            self.wake_source();
            if let Some(request) = request {
                Self::reject(request, error);
            }
            None
        } else {
            if request.is_some() {
                wake.progress.peer_entries.fetch_add(1, Ordering::AcqRel);
            }
            request
        }
    }
    pub(super) fn wake_source(&self) {
        if let Some(source) = self.source.upgrade() {
            source.wake();
        }
    }
    pub(super) fn reject(mut request: Request, error: InstanceError) {
        // Return credit before invoking application-owned destructors.
        drop(request.capacity.take());
        if let Work::Service(job) = request.work {
            job.fail(error);
        }
    }
}
