//! Rust offer lifetime is independent of native credit and Scheme graph lifetime.
use super::{
    Capacity, EntryCredits, InstanceError, PoolSource, Request, RouteTarget, StagedWork,
    pool::Work, service_wake::Wake,
};
use std::{
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
struct Payload {
    request: Request,
    staged: StagedWork,
}
pub(super) struct Offer {
    payload: Mutex<Option<Payload>>,
    source: Weak<PoolSource>,
    target: Weak<Wake>,
    capacity: Arc<Capacity>,
    active: Weak<Semaphore>,
    accepting: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
    deadline: Instant,
    pub(super) units: usize,
}
/// An actual owner receipt is still revocable until the entry fence succeeds.
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
        target: &RouteTarget,
        accepting: Arc<AtomicBool>,
        cancelled: Arc<AtomicBool>,
        deadline: Instant,
    ) -> Arc<Self> {
        let units = request.work_units;
        debug_assert!(
            request.capacity.is_none(),
            "offer cannot hold native credits"
        );
        Arc::new(Self {
            units,
            payload: Mutex::new(Some(Payload {
                request,
                staged: StagedWork::new(Arc::clone(&target.staged), units),
            })),
            source: Arc::downgrade(source),
            target: Arc::downgrade(&target.wake),
            capacity: Arc::clone(&target.capacity),
            active: Arc::downgrade(&target.active),
            accepting,
            cancelled,
            deadline,
        })
    }
    pub(super) fn pending(&self) -> bool {
        self.payload
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    }
    fn detach(&self) {
        if let Some(wake) = self.target.upgrade() {
            wake.offers.remove(self);
        }
    }
    pub(super) fn withdraw(&self) -> Option<Request> {
        let payload = self
            .payload
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        self.detach(); // no offer lock is held while touching its notification slot
        payload.map(|p| p.request)
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
    fn error(&self, wake: &Wake) -> Option<InstanceError> {
        if !crate::instance_backend::images_ready()
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
        }
    }
    fn reserve_active(&self) -> Result<OwnedSemaphorePermit, InstanceError> {
        let active = self.active.upgrade().ok_or(InstanceError::Closed)?;
        active
            .try_acquire_owned()
            .map_err(|e| super::capacity::immediate_error(&e))
    }
    fn enter(&self, wake: &Wake) -> Option<Request> {
        let mut guard = self
            .payload
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.as_ref()?;
        if let Some(error) = self.error(wake) {
            drop(guard);
            self.cancel(error);
            return None;
        }
        let active = match self.reserve_active() {
            Ok(active) => active,
            Err(InstanceError::QueueFull) => return None,
            Err(error) => {
                drop(guard);
                self.cancel(error);
                return None;
            }
        };
        // Immediate semaphore attempts do not invoke application callbacks.
        // Retain every acquired part until after the payload fence is released.
        // Withdrawal cannot win between native acquisition and payload take.
        let mut credits = EntryCredits::new(Arc::clone(&self.capacity), active);
        let error = credits
            .acquire(self.units)
            .err()
            .or_else(|| self.error(wake));
        let payload = if matches!(error, Some(InstanceError::QueueFull)) {
            None
        } else {
            guard.take()
        };
        drop(guard);
        let Payload {
            mut request,
            staged,
        } = payload?;
        self.detach();
        if let Some(error) = error {
            drop((credits, staged));
            Self::reject(request, error);
            self.wake_source();
            return None;
        }
        // Unique entry fence precedes M::construct and fixes native ownership.
        request.capacity = Some(credits.finish(staged));
        wake.progress.peer_entries.fetch_add(1, Ordering::AcqRel);
        Some(request)
    }
    #[cfg(test)]
    pub(super) fn with_entry_fence<T>(&self, action: impl FnOnce() -> T) -> T {
        let _guard = self.payload.lock().unwrap();
        action()
    }
    #[cfg(test)]
    pub(super) fn entry_fence_is_free(&self) -> bool {
        self.payload.try_lock().is_ok()
    }
    pub(super) fn cancel(&self, error: InstanceError) {
        if let Some(request) = self.withdraw() {
            Self::reject(request, error);
            self.wake_source();
        }
    }
    pub(super) fn wake_source(&self) {
        if let Some(source) = self.source.upgrade() {
            source.wake_if_changed(self.units);
        }
    }
    pub(super) fn reject(mut request: Request, error: InstanceError) {
        drop(request.capacity.take());
        if let Work::Service(job) = request.work {
            job.fail(error);
        }
    }
}
