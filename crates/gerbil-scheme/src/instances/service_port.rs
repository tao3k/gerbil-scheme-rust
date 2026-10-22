//! Explicit Scheme-owned input/completion wake transport; no Rust green scheduler.
use super::{InstanceDispatcher, InstanceError, InstanceTicket};
use std::sync::{Arc, atomic::Ordering};
/// Existing watcher is the default. `NativePort` uses the same Scheme IO/timer loop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemeServiceWake {
    Watcher,
    NativePort,
}
impl InstanceDispatcher {
    /// Change the native wake bridge only when Scheme work/leases have drained.
    /// The returned ticket confirms old-reader join and exclusive port ownership.
    /// # Errors
    /// Rejects unsupported/closed services, non-idle work or native IO failure.
    pub fn configure_scheme_service_wake_on(
        &self,
        owner: usize,
        mode: SchemeServiceWake,
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
            if wake.service_version.load(Ordering::Acquire) < 4 {
                return Err(InstanceError::InvalidConfig(
                    "Scheme service lacks native port wake",
                ));
            }
            Arc::clone(wake)
        };
        let direct = matches!(mode, SchemeServiceWake::NativePort);
        self.submit_native_on(owner, move |session| {
            let value = super::service::command(session, &[10, i64::from(direct)], None)?;
            if session.copy_numeric::<i64>(&value)? != [i64::from(direct)] {
                return Err(InstanceError::InvalidConfig("native port wake receipt"));
            }
            wake.direct.store(direct, Ordering::Release);
            Ok(())
        })
    }
}

/// Scheme-owned completion handoff. Snapshot preserves the existing default.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemeServiceCompletion {
    Snapshot,
    /// One result per class per owner turn; native workers yield at job handoff.
    FairHandoff,
}
impl InstanceDispatcher {
    /// Configure completion handoff on an idle fixed owner (service ABI v7).
    /// Scheme retains result graphs until projection; Rust never moves native tasks.
    /// # Errors
    /// Closed/unsupported service, non-idle work, or invalid native receipt.
    pub fn configure_scheme_service_completion_on(
        &self,
        owner: usize,
        policy: SchemeServiceCompletion,
    ) -> Result<InstanceTicket<()>, InstanceError> {
        {
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
            if wake.service_version.load(Ordering::Acquire) < 7 {
                return Err(InstanceError::InvalidConfig(
                    "Scheme service lacks completion handoff",
                ));
            }
        }
        let mode = i64::from(policy == SchemeServiceCompletion::FairHandoff);
        self.submit_native_on(owner, move |session| {
            let value = super::service::command(session, &[13, mode], None)?;
            if session.copy_numeric::<i64>(&value)? != [mode] {
                return Err(InstanceError::InvalidConfig(
                    "native completion handoff receipt",
                ));
            }
            Ok(())
        })
    }
}
