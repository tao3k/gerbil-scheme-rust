//! Scheme-owned cooperative unit/class policy; Rust only configures and observes.
use super::{InstanceDispatcher, InstanceError, InstanceTicket};
use std::sync::atomic::Ordering;
/// Granted Scheme units and contention, not wall-clock latency bounds.
#[derive(Clone, Copy, Debug, Default)]
pub struct SchemeCooperativeBudgetObservation {
    pub short_units: u64,
    pub bulk_units: u64,
    pub suspensions: u64,
    pub maximum_short_bypasses: u64,
    pub maximum_bulk_bypasses: u64,
    pub contended_grants: u64,
    pub queued_short: u64,
    pub queued_bulk: u64,
    pub active: u64,
}
impl InstanceDispatcher {
    fn require_service_budget(&self, owner: usize) -> Result<(), InstanceError> {
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
        if wake.service_version.load(Ordering::Acquire) < 3 {
            return Err(InstanceError::InvalidConfig(
                "Scheme service lacks cooperative budget capability",
            ));
        }
        Ok(())
    }
    /// Configure Scheme class arbitration at a native idle boundary. Zero
    /// disables it (the default); 1..=256 bounds consecutive cooperative units
    /// when both classes wait. Applications define units and release on waits.
    /// Arbitrary Scheme code is never preempted or restricted by Rust.
    /// # Errors
    /// Rejects unsupported/closed endpoints, invalid bounds and non-idle Scheme
    /// jobs. The returned ticket confirms actual native configuration.
    pub fn configure_scheme_service_budget_on(
        &self,
        owner: usize,
        maximum_consecutive_units: usize,
    ) -> Result<InstanceTicket<()>, InstanceError> {
        self.require_service_budget(owner)?;
        let maximum = i64::try_from(maximum_consecutive_units)
            .ok()
            .filter(|value| *value <= 256)
            .ok_or(InstanceError::InvalidConfig(
                "cooperative unit budget bound",
            ))?;
        self.submit_native_on(owner, move |session| {
            let value = super::service::command(session, &[8, maximum], None)?;
            if session.copy_numeric::<i64>(&value)? != [0] {
                return Err(InstanceError::InvalidConfig("cooperative budget receipt"));
            }
            Ok(())
        })
    }
    /// Snapshot Scheme's native counters through the existing fixed owner.
    /// # Errors
    /// Rejects unsupported/closed endpoints or invalid native metadata.
    pub fn scheme_service_budget_on(
        &self,
        owner: usize,
    ) -> Result<InstanceTicket<SchemeCooperativeBudgetObservation>, InstanceError> {
        self.require_service_budget(owner)?;
        self.submit_native_on(owner, |session| {
            let value = super::service::command(session, &[9], None)?;
            let values = session.copy_numeric::<i64>(&value)?;
            let row: Vec<_> = values.into_iter().map(u64::try_from).collect();
            let [
                Ok(short_units),
                Ok(bulk_units),
                Ok(suspensions),
                Ok(maximum_short_bypasses),
                Ok(maximum_bulk_bypasses),
                Ok(contended_grants),
                Ok(queued_short),
                Ok(queued_bulk),
                Ok(active),
            ] = row.as_slice()
            else {
                return Err(InstanceError::InvalidConfig("cooperative budget counters"));
            };
            Ok(SchemeCooperativeBudgetObservation {
                short_units: *short_units,
                bulk_units: *bulk_units,
                suspensions: *suspensions,
                maximum_short_bypasses: *maximum_short_bypasses,
                maximum_bulk_bypasses: *maximum_bulk_bypasses,
                contended_grants: *contended_grants,
                queued_short: *queued_short,
                queued_bulk: *queued_bulk,
                active: *active,
            })
        })
    }
}
