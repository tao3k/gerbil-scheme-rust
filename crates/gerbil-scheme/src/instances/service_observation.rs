//! Explicit opt-in observation on the existing fixed-owner API.
use super::{InstanceDispatcher, InstanceError, InstanceTicket, SchemeServiceObservation};
use std::sync::Arc;
impl InstanceDispatcher {
    /// Enable phase clocks for this owner. Call before the measured workload.
    /// Disabled owners read no additional clocks or native diagnostic counters.
    /// # Errors
    /// Rejects a missing owner, disabled Scheme service or closed pool.
    pub fn observe_scheme_service_on(&self, owner: usize) -> Result<(), InstanceError> {
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
        if wake.closed.load(std::sync::atomic::Ordering::Acquire) {
            return Err(InstanceError::Closed);
        }
        wake.observation
            .get_or_init(|| Arc::new(super::service_metrics::Monitor::default()));
        Ok(())
    }
    /// Snapshot completed intervals on the owner after its prior work. A current
    /// blocking wait is counted when the snapshot request wakes and exits it.
    /// Observer wake/idle time is included: compare deltas, not startup totals.
    /// # Errors
    /// Rejects a disabled observer or closed/missing owner.
    pub fn scheme_service_observation_on(
        &self,
        owner: usize,
    ) -> Result<InstanceTicket<SchemeServiceObservation>, InstanceError> {
        let monitor = {
            let senders = self
                .admission
                .senders
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            Arc::clone(
                senders
                    .owners
                    .get(owner)
                    .and_then(|o| o.wake.as_ref())
                    .and_then(|wake| wake.observation.get())
                    .ok_or(InstanceError::InvalidConfig(
                        "Scheme service observation not enabled",
                    ))?,
            )
        };
        self.submit_native_on(owner, move |_| Ok(monitor.snapshot()))
    }
}
