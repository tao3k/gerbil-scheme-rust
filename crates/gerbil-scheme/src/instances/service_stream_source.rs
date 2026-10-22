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
    pub fair: OnceLock<super::Fair>,
    pub(super) requests: Mutex<VecDeque<Request>>,
    pub signals: Mutex<Vec<Weak<super::service_stream_runner::Signal>>>,
}
impl Shared {
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
            fair: OnceLock::new(),
            requests: Mutex::new(requests),
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
            Self::Pool(s) => s
                .requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_empty(),
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
        self.with(|q| q.front().map(|r| r.work_units))
    }
    pub(super) fn assist(
        &self,
        owner: usize,
        units: usize,
        admission: &Weak<super::Admission>,
        cancelled: &AtomicBool,
        deadline: std::time::Instant,
    ) -> bool {
        match self {
            Self::Owner(_) => false,
            Self::Pool(s) => super::route_progress(s, owner, units, admission, cancelled, deadline),
        }
    }
    pub(super) fn take(&mut self, units: usize) -> Option<Request> {
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
        let requests = self.with(std::mem::take);
        if !requests.is_empty() {
            self.wake();
        }
        for request in requests {
            if let Work::Service(job) = request.work {
                job.fail(error.clone());
            }
        }
    }
}
