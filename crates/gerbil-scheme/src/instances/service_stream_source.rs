//! Only uncommitted Rust messages are shared; native state never leaves its owner.
use super::{InstanceError, Request, pool::Work};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex, OnceLock, Weak,
        atomic::{AtomicBool, Ordering},
    },
};
pub(super) struct Shared {
    pub ready: AtomicBool,
    // Monotonic terminal input state. Mutation is serialized by requests.
    sealed: AtomicBool,
    failure: OnceLock<InstanceError>,
    pub fair: OnceLock<super::Fair>,
    pub(super) requests: Mutex<VecDeque<Request>>,
    pub(super) offers: Mutex<VecDeque<Arc<super::Offer>>>,
    pub signals: Mutex<Vec<Weak<super::service_stream_runner::Signal>>>,
}
impl Shared {
    pub(super) fn opening() -> Arc<Self> {
        let source = Self::new(VecDeque::new());
        source.sealed.store(false, Ordering::Release);
        source.ready.store(true, Ordering::Release);
        source
    }
    pub(super) fn publish(
        &self,
        batch: VecDeque<Request>,
        cancelled: &AtomicBool,
        deadline: std::time::Instant,
    ) -> Result<(), InstanceError> {
        self.publish_bounded(batch, cancelled, deadline, usize::MAX)
    }
    pub(super) fn publish_bounded(
        &self,
        mut batch: VecDeque<Request>,
        cancelled: &AtomicBool,
        deadline: std::time::Instant,
        bound: usize,
    ) -> Result<(), InstanceError> {
        let notify = {
            let mut requests = self
                .requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(error) = self.failure.get() {
                return Err(error.clone());
            }
            if self.sealed.load(Ordering::Acquire) {
                return Err(InstanceError::Closed);
            }
            if cancelled.load(Ordering::Acquire) {
                return Err(InstanceError::Cancelled);
            }
            if std::time::Instant::now() >= deadline {
                return Err(InstanceError::Timeout);
            }
            if batch.len() > bound.saturating_sub(requests.len()) {
                return Err(InstanceError::QueueFull);
            }
            let notify = requests.is_empty();
            requests.append(&mut batch);
            notify
        };
        // Publication precedes notification. Empty/open is never end-of-input.
        if notify {
            self.wake();
        }
        Ok(())
    }
    pub(super) fn seal(&self) {
        {
            let _requests = self
                .requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            self.sealed.store(true, Ordering::Release);
        }
        self.wake();
    }
    /// Remove only a still-staged request. `Source::take` uses this same mutex,
    /// so withdrawal and owner claim have one winner. Destructors/wakes run outside.
    pub(super) fn withdraw(&self, started: &Arc<AtomicBool>) -> bool {
        let request = {
            let mut requests = self
                .requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            requests
                .iter()
                .position(|r| Arc::ptr_eq(&r.started, started))
                .and_then(|index| requests.remove(index))
        };
        if let Some(request) = request {
            drop(request);
            self.wake();
            true
        } else {
            false
        }
    }
    pub(super) fn head_units(&self) -> Option<usize> {
        let requests = self
            .requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let offers = self
            .offers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        offers
            .iter()
            .find(|o| o.pending())
            .map(|o| o.units)
            .or_else(|| requests.front().map(|r| r.work_units))
    }
    pub(super) fn wake_if_changed(&self, previous: usize) {
        if self.head_units() != Some(previous) {
            self.wake();
        }
    }
    pub(super) fn wake(&self) {
        let signals = self
            .signals
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        for signal in signals {
            if let Some(signal) = signal.upgrade() {
                signal.wake();
            }
        }
    }
    pub(super) fn new(requests: VecDeque<Request>) -> Arc<Self> {
        Arc::new(Self {
            ready: AtomicBool::new(false),
            sealed: AtomicBool::new(true),
            failure: OnceLock::new(),
            fair: OnceLock::new(),
            requests: Mutex::new(requests),
            offers: Mutex::new(VecDeque::new()),
            signals: Mutex::new(Vec::new()),
        })
    }
}
pub(super) enum Source {
    Owner(VecDeque<Request>),
    Pool(Arc<Shared>),
}
impl Source {
    pub(super) fn is_empty(&self) -> bool {
        match self {
            Self::Owner(q) => q.is_empty(),
            Self::Pool(s) => {
                let requests = s
                    .requests
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if !s.sealed.load(Ordering::Acquire) {
                    return false;
                }
                if s.fair.get().is_none() {
                    return requests.is_empty();
                }
                let offers = s
                    .offers
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                requests.is_empty() && !offers.iter().any(|o| o.pending())
            }
        }
    }
    pub(super) fn ready(&self) -> bool {
        match self {
            Self::Owner(_) => true,
            Self::Pool(s) => s.ready.load(Ordering::Acquire),
        }
    }
    fn with<T>(&mut self, f: impl FnOnce(&mut VecDeque<Request>) -> T) -> T {
        match self {
            Self::Owner(q) => f(q),
            Self::Pool(s) => f(&mut s
                .requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)),
        }
    }
    pub(super) fn units(&mut self) -> Option<usize> {
        if let Self::Pool(s) = self {
            if s.fair.get().is_none() {
                return self.with(|q| q.front().map(|r| r.work_units));
            }
            let requests = s
                .requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let mut offers = s
                .offers
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            offers.retain(|o| o.pending());
            return offers
                .front()
                .map(|o| o.units)
                .or_else(|| requests.front().map(|r| r.work_units));
        }
        self.with(|q| q.front().map(|r| r.work_units))
    }
    pub(super) fn assist(
        &self,
        owner: usize,
        units: usize,
        accepting: &Arc<AtomicBool>,
        cancelled: &Arc<AtomicBool>,
        deadline: std::time::Instant,
    ) -> bool {
        match self {
            Self::Owner(_) => false,
            Self::Pool(s) => super::route_progress(s, owner, units, accepting, cancelled, deadline),
        }
    }
    pub(super) fn take(&mut self, units: usize) -> Option<Request> {
        // A ready owner can withdraw an offer without waiting for its mailbox.
        // Release the old target's provisional credits outside all source locks.
        let reclaimed = if let Self::Pool(s) = self {
            if s.fair.get().is_none() {
                None
            } else {
                let _requests = s
                    .requests
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let mut offers = s
                    .offers
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                offers.retain(|o| o.pending());
                if offers.front().is_some_and(|o| o.units == units) {
                    offers.pop_front().and_then(|o| o.reclaim())
                } else {
                    None
                }
            }
        } else {
            None
        };
        if let Some(mut request) = reclaimed {
            drop(request.capacity.take());
            if self.units() != Some(units) {
                self.wake();
            }
            return Some(request);
        }
        let request = self.with(|q| {
            if q.front().is_some_and(|r| r.work_units == units) {
                q.pop_front()
            } else {
                None
            }
        });
        if request.is_some() && self.units() != Some(units) {
            self.wake();
        }
        request
    }
    fn wake(&self) {
        if let Self::Pool(source) = self {
            source.wake();
        }
    }
    pub(super) fn reject(&mut self, error: &InstanceError) {
        // Fail application jobs outside the shared source lock.
        let (requests, was_open) = match self {
            Self::Owner(q) => (std::mem::take(q), false),
            Self::Pool(source) => {
                let mut requests = source
                    .requests
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let _ = source.failure.set(error.clone());
                let was_open = !source.sealed.swap(true, Ordering::AcqRel);
                (std::mem::take(&mut *requests), was_open)
            }
        };
        let offers = if let Self::Pool(s) = self {
            std::mem::take(
                &mut *s
                    .offers
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner),
            )
        } else {
            VecDeque::new()
        };
        if was_open || !requests.is_empty() || !offers.is_empty() {
            self.wake();
        }
        for request in offers.into_iter().filter_map(|o| o.withdraw()) {
            super::Offer::reject(request, error.clone());
        }
        for request in requests {
            if let Work::Service(job) = request.work {
                job.fail(error.clone());
            }
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/service_stream_source.rs"]
mod tests;
