//! Host transport policy; Scheme owns all intra-instance scheduling.
use super::{InstanceDispatcher, InstanceError};
use std::sync::atomic::Ordering;
/// Coalescing remains the default. `ShortIndependent` writes discharge each declared short wake duty.
#[derive(Clone, Copy, Debug)]
pub enum SchemeServiceNotification {
    Coalesced,
    ShortIndependent,
}
impl InstanceDispatcher {
    /// Select host notification transport for one live Scheme service.
    /// Explicit short-service one-byte writes bypass another publisher's claimed wake and
    /// writer mutex. Full pipes are already readable. This adds no OS deadline.
    /// Bulk notifications remain coalesced; close also writes independently.
    /// Changes may overlap existing publishers; redundant bytes remain safe.
    /// # Errors
    /// Rejects absent, uninitialized or closed owners/services.
    pub fn configure_scheme_service_notification_on(
        &self,
        owner: usize,
        policy: SchemeServiceNotification,
    ) -> Result<(), InstanceError> {
        let senders = self
            .admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let wake = senders
            .owners
            .get(owner)
            .and_then(|owner| owner.wake.as_ref())
            .ok_or(InstanceError::Closed)?;
        if wake.closed.load(Ordering::Acquire) {
            return Err(InstanceError::Closed);
        }
        if wake.service_version.load(Ordering::Acquire) == 0 {
            return Err(InstanceError::InvalidConfig(
                "Scheme service not initialized",
            ));
        }
        wake.independent_short.store(
            matches!(policy, SchemeServiceNotification::ShortIndependent),
            Ordering::Release,
        );
        Ok(())
    }
}
