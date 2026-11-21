//! Read applied Scheme configuration, never infer it from a host strategy name.
use super::{InstanceDispatcher, InstanceError, InstanceTicket, SchemeServiceWake};
use std::sync::atomic::Ordering;
/// Native service settings read on the fixed owner. No rooted value escapes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeServiceSettings {
    /// Zero disables cooperative class arbitration; otherwise 1..=256.
    pub cooperative_budget: usize,
    pub wake: SchemeServiceWake,
}
impl SchemeServiceSettings {
    fn parse(values: &[i64]) -> Result<Self, InstanceError> {
        let [budget @ 0..=256, direct @ 0..=1] = values else {
            return Err(InstanceError::InvalidConfig(
                "native service settings receipt",
            ));
        };
        Ok(Self {
            cooperative_budget: usize::try_from(*budget).unwrap(),
            wake: if *direct == 1 {
                SchemeServiceWake::NativePort
            } else {
                SchemeServiceWake::Watcher
            },
        })
    }
}
impl InstanceDispatcher {
    /// Read the applied Scheme budget and wake mode through the native endpoint.
    /// The query is serialized on the fixed owner. It neither configures policy
    /// nor bypasses native capacity or lifecycle checks. Requires service ABI v5.
    /// # Errors
    /// Closed/unsupported endpoint, capacity exhaustion or invalid native metadata.
    pub fn scheme_service_settings_on(
        &self,
        owner: usize,
    ) -> Result<InstanceTicket<SchemeServiceSettings>, InstanceError> {
        self.require_service_settings(owner)?;
        self.submit_control_on(owner, |session| {
            let value = super::service::command(session, &[11], None)?;
            SchemeServiceSettings::parse(&session.copy_numeric::<i64>(&value)?)
        })
    }
    fn require_service_settings(&self, owner: usize) -> Result<(), InstanceError> {
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
        if wake.closed.load(Ordering::Acquire) {
            return Err(InstanceError::Closed);
        }
        if wake.service_version.load(Ordering::Acquire) < 5 {
            return Err(InstanceError::InvalidConfig(
                "Scheme service lacks settings readback",
            ));
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "../../tests/unit/service_settings.rs"]
mod tests;
