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
