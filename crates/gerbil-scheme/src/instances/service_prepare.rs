//! One owned Rust input per owner; preparation has no native session or roots.
use super::{InstanceError, Request, service_offer::Offer, service_wake::Wake};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
pub(super) struct Prepared {
    request: Request,
    accepting: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
    deadline: Instant,
}
impl Prepared {
    pub(super) fn new(
        request: Request,
        accepting: Arc<AtomicBool>,
        cancelled: Arc<AtomicBool>,
        deadline: Instant,
    ) -> Self {
        Self {
            request,
            accepting,
            cancelled,
            deadline,
        }
    }
    pub(super) fn enter(self, wake: &Wake) -> Option<Request> {
        let error = if !crate::instance_backend::images_ready()
            || !self.accepting.load(Ordering::Acquire)
            || wake.closed.load(Ordering::Acquire)
            || wake.failed.load(Ordering::Acquire)
        {
            Some(InstanceError::Closed)
        } else if self.cancelled.load(Ordering::Acquire) {
            Some(InstanceError::Cancelled)
        } else if Instant::now() >= self.deadline {
            Some(InstanceError::Timeout)
        } else {
            None
        };
        if let Some(error) = error {
            self.reject(error);
            None
        } else {
            Some(self.request)
        }
    }
    pub(super) fn reject(self, error: InstanceError) {
        Offer::reject(self.request, error);
    }
}
