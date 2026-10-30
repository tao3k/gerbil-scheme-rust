//! Opt-in owner-local typed exchange; the existing Scheme workers still execute.
use super::{InstanceDispatcher, InstanceError, InstanceTicket, NativeSession};
use std::sync::{Arc, atomic::Ordering};
impl InstanceDispatcher {
    /// Enable direct typed submission and ready-slot transfer on an idle Snapshot
    /// service. All three native exports must be present; defaults are unchanged.
    /// # Errors
    /// Closed/unsupported owner, pending work or invalid capability.
    pub fn configure_scheme_service_exchange_on(
        &self,
        owner: usize,
        enabled: bool,
    ) -> Result<InstanceTicket<()>, InstanceError> {
        let wake = {
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
            if wake.service_version.load(Ordering::Acquire) < 34 {
                return Err(InstanceError::InvalidConfig(
                    "native exchange ABI unavailable",
                ));
            }
            Arc::clone(wake)
        };
        self.submit_control_on(owner, move |session| {
            if session.image.native.exchange.is_none() {
                return Err(InstanceError::InvalidConfig(
                    "native exchange exports unavailable",
                ));
            }
            let value = super::service::command(session, &[49, i64::from(enabled)], None)?;
            if enabled {
                if session.exchange_id().is_none() {
                    session.retain_exchange(value)?;
                }
            } else {
                if session.copy_numeric::<i64>(&value)? != [0] {
                    return Err(InstanceError::InvalidConfig(
                        "native exchange disable receipt",
                    ));
                }
                session.clear_exchange()?;
            }
            wake.exchange.store(enabled, Ordering::Release);
            Ok(())
        })
    }
}
impl NativeSession<'_> {
    /// Whether this owner explicitly enabled the typed exchange capability.
    pub fn scheme_service_exchange_enabled(&self) -> bool {
        self.exchange_id().is_some()
    }
    pub(super) fn exchange_failure(&self) -> InstanceError {
        match self.call_result(0) {
            Err(error) => error,
            Ok(_) => InstanceError::InvalidConfig("native exchange failed without diagnostic"),
        }
    }
}
