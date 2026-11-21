//! Completion publication order and bounded opt-in observations; no Scheme state.
use super::{InstanceDispatcher, InstanceError};
use std::{
    sync::{Mutex, atomic::Ordering},
    time::Instant,
};
/// Captured once per owner harvest; never changes Scheme execution budgets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemeCompletionPublication {
    Immediate,
    AfterRetirement,
}
/// Send intervals may overlap a client's ready poll and consumption. Missing
/// send-end means the sender has not returned; clients must not wait for it.
#[derive(Clone, Copy, Debug, Default)]
pub struct SchemeCompletionObservation {
    pub native_ready: Option<Instant>,
    pub retired: Option<Instant>,
    pub send_start: Option<Instant>,
    pub send_end: Option<Instant>,
    pub delivered: Option<bool>,
}
/// Explicit cold diagnostic metadata, not a root, permit or scheduling ledger.
#[derive(Debug, Default)]
pub struct SchemeCompletionTrace(Mutex<SchemeCompletionObservation>);
impl SchemeCompletionTrace {
    #[must_use]
    pub fn snapshot(&self) -> SchemeCompletionObservation {
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub(super) fn before_send(&self, native_ready: Instant) {
        let mut row = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        row.native_ready = Some(native_ready);
        row.send_start = Some(Instant::now());
    }
    pub(super) fn sent(&self, delivered: bool) {
        let at = Instant::now();
        let mut row = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        row.send_end = Some(at);
        row.delivered = Some(delivered);
    }
    pub(super) fn retired(&self) {
        let at = Instant::now();
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retired = Some(at);
    }
}
impl InstanceDispatcher {
    /// Select reply publication before or after that result's native/stream
    /// credit return. The default remains Immediate. A running harvest retains
    /// its captured choice; this store affects a subsequent harvest. Projection
    /// or slot-transfer failure still retains credit for owner drain. Successful
    /// retained application values keep their normal independent ownership.
    /// # Errors
    /// Invalid owner, missing service or a closed/failed endpoint.
    pub fn configure_scheme_completion_publication_on(
        &self,
        owner: usize,
        policy: SchemeCompletionPublication,
    ) -> Result<SchemeCompletionPublication, InstanceError> {
        let senders = self
            .admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let wake = senders
            .owners
            .get(owner)
            .and_then(|o| o.wake.as_ref())
            .ok_or(InstanceError::Closed)?;
        if wake.closed.load(Ordering::Acquire) || wake.failed.load(Ordering::Acquire) {
            return Err(InstanceError::Closed);
        }
        wake.retire_before_publish.store(
            policy == SchemeCompletionPublication::AfterRetirement,
            Ordering::Release,
        );
        Ok(policy)
    }
}
