//! One revocable Rust offer per owner, separate from request and short inboxes.
use super::{InstanceError, Offer, service_wake::Wake};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
#[derive(Default)]
pub(super) struct OfferPort {
    slot: Mutex<Option<Arc<Offer>>>,
    full: AtomicBool,
}
impl std::fmt::Debug for OfferPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OfferPort")
            .field("pending", &self.pending())
            .finish()
    }
}
impl OfferPort {
    pub(super) fn pending(&self) -> bool {
        self.full.load(Ordering::Acquire)
    }
    pub(super) fn peek(&self) -> Option<Arc<Offer>> {
        // Ordinary/default owners never lock an empty experimental offer port.
        if !self.pending() {
            return None;
        }
        self.slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
    pub(super) fn publish(&self, offer: Arc<Offer>, wake: &Wake) -> bool {
        let mut slot = self
            .slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if slot.is_some()
            || wake.closed.load(Ordering::Acquire)
            || wake.failed.load(Ordering::Acquire)
        {
            return false;
        }
        *slot = Some(offer);
        self.full.store(true, Ordering::Release);
        true
    }
    pub(super) fn remove(&self, offer: &Offer) {
        let removed = {
            let mut slot = self
                .slot
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if slot
                .as_ref()
                .is_some_and(|old| std::ptr::eq(old.as_ref(), offer))
            {
                let removed = slot.take();
                self.full.store(false, Ordering::Release);
                removed
            } else {
                None
            }
        };
        drop(removed);
    }
    pub(super) fn close(&self) {
        let offer = {
            let mut slot = self
                .slot
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let offer = slot.take();
            self.full.store(false, Ordering::Release);
            offer
        };
        if let Some(offer) = offer {
            offer.cancel(InstanceError::Closed);
        }
    }
}
