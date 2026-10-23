//! Bounded owner admission turns after complete native result retirement.
use super::{InstanceDispatcher, InstanceError, InstanceTicket};
use std::sync::{Arc, atomic::Ordering};
impl InstanceDispatcher {
    /// Opt into one FIFO/short/supply admission turn per independently retired
    /// result. Requires `PipelineCycle`; selecting a completion policy resets this
    /// option. Pure wakes consume a turn without ending the whole refill cycle.
    /// Controls and arbitrary native callbacks remain FIFO barriers. No turn is
    /// retained across cycles, and slot/work permits authorize every new input.
    /// This bounds dispatch turns, not arbitrary Scheme execution time.
    /// # Errors
    /// Closed owner or a completion policy without pipeline support.
    pub fn configure_scheme_service_refill_on(
        &self,
        owner: usize,
        enabled: bool,
    ) -> Result<InstanceTicket<bool>, InstanceError> {
        let wake = {
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
            Arc::clone(wake)
        };
        self.submit_control_on(owner, move |_| {
            if !wake.pipeline_cycle.load(Ordering::Acquire) {
                return Err(InstanceError::InvalidConfig(
                    "refill requires PipelineCycle",
                ));
            }
            wake.coupled_refill.store(enabled, Ordering::Release);
            Ok(wake.coupled_refill.load(Ordering::Acquire))
        })
    }
}

impl InstanceDispatcher {
    /// Opt into Scheme-owned numeric construction and admission in one native
    /// entry. Requires an idle ABI 12 `PipelineCycle` endpoint explicitly enabling
    /// native entry. The owner retains
    /// one revocable submission capability, never an application input buffer.
    /// Completion policy changes revoke it; close drains before arena release.
    /// # Errors
    /// Closed/unsupported endpoint, non-idle native service or retained-root failure.
    pub fn configure_scheme_service_native_entry_on(
        &self,
        owner: usize,
        enabled: bool,
    ) -> Result<InstanceTicket<bool>, InstanceError> {
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
        if enabled && wake.service_version.load(Ordering::Acquire) < 12 {
            return Err(InstanceError::InvalidConfig(
                "native service entry requires ABI 12",
            ));
        }
        drop(senders);
        self.submit_control_on(owner, move |session| {
            if enabled {
                if session.image.native.numeric_service_enter.is_none()
                    || session.image.native.numeric_batch_transfer.is_none()
                {
                    return Err(InstanceError::InvalidConfig(
                        "native service entry unavailable",
                    ));
                }
                if session.entry_id().is_none() {
                    let capability = super::service::command(session, &[19], None)?;
                    session.retain_entry(capability)?;
                }
            } else {
                session.clear_entry()?;
            }
            Ok(session.entry_id().is_some())
        })
    }
}

impl super::NativeSession<'_> {
    /// Whether this owner currently retains the explicit native submission
    /// capability. This exposes no root identity or cross-owner authority.
    #[must_use]
    pub fn scheme_service_native_entry_enabled(&self) -> bool {
        self.entry_id().is_some()
    }
}
