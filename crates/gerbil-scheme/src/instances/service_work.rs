//! Scheme-owned declared executed work. Admission and item progress are independent.
use super::{InstanceDispatcher, InstanceError, InstanceTicket};
use std::sync::atomic::Ordering;
/// Logical work reported at application safe spans; never CPU time or a wall bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeExecutedWork {
    pub quantum: u64,
    pub short_work: u64,
    pub bulk_work: u64,
    pub handoffs: u64,
    /// Outstanding work-start receipts. ABI23 settles at first grant; earlier
    /// policies settle at dispatch. Use execution progress for explicit phases.
    pub pending_start: u64,
    pub ready_generation: u64,
    pub offered_generation: u64,
}
impl InstanceDispatcher {
    /// Configure an idle ABI 14 owner. Zero disables (default). A nonzero bound
    /// requires cooperative class arbitration. Scheme handlers report safe spans
    /// no larger than this bound; arbitrary handlers remain fully capable.
    /// Admission charges, item completion and root retirement do not change.
    /// # Errors
    /// Unsupported/closed owner, invalid span bound, or non-idle native service.
    pub fn configure_scheme_service_work_on(
        &self,
        owner: usize,
        quantum: usize,
    ) -> Result<InstanceTicket<u64>, InstanceError> {
        self.work_version(owner)?;
        let quantum = i64::try_from(quantum)
            .ok()
            .filter(|q| *q <= 256)
            .ok_or(InstanceError::InvalidConfig("executed work span bound"))?;
        self.submit_control_on(owner, move |session| {
            let value = super::service::command(session, &[22, quantum], None)?;
            if session.copy_numeric::<i64>(&value)? != [quantum] {
                return Err(InstanceError::InvalidConfig("executed work receipt"));
            }
            Ok(quantum.unsigned_abs())
        })
    }
    /// Read actual native counters outside the measurement window.
    /// # Errors
    /// Unsupported/closed owner or malformed native metadata.
    pub fn scheme_service_work_on(
        &self,
        owner: usize,
    ) -> Result<InstanceTicket<SchemeExecutedWork>, InstanceError> {
        self.work_version(owner)?;
        self.submit_control_on(owner, |session| {
            let value = super::service::command(session, &[23], None)?;
            let counts: Vec<u64> = session
                .copy_numeric::<i64>(&value)?
                .into_iter()
                .map(u64::try_from)
                .collect::<Result<_, _>>()
                .map_err(|_| InstanceError::InvalidConfig("executed work negative count"))?;
            let [
                quantum,
                short_work,
                bulk_work,
                handoffs,
                pending_start,
                ready_generation,
                offered_generation,
            ] = counts.as_slice()
            else {
                return Err(InstanceError::InvalidConfig("executed work shape"));
            };
            if *quantum > 256 || offered_generation > ready_generation {
                return Err(InstanceError::InvalidConfig("executed work state"));
            }
            Ok(SchemeExecutedWork {
                quantum: *quantum,
                short_work: *short_work,
                bulk_work: *bulk_work,
                handoffs: *handoffs,
                pending_start: *pending_start,
                ready_generation: *ready_generation,
                offered_generation: *offered_generation,
            })
        })
    }
    fn work_version(&self, owner: usize) -> Result<(), InstanceError> {
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
        if wake.service_version.load(Ordering::Acquire) < 14 {
            return Err(InstanceError::InvalidConfig(
                "executed work requires ABI 14",
            ));
        }
        Ok(())
    }
}
