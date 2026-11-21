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
        self.submit_control_on(owner, move |session| {
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
    /// Coalesce ready graphs behind a consumed owner token; retain per-result credit return.
    CoalescedHandoff,
    /// Batch native take, independently clear result slots, and refill between projections.
    NativeCycle,
    /// Prepare one root-free Rust input while retiring ready results; construct afterward.
    PreparedCycle,
    /// Retire ready slots, then refill from returned credits using fused native inputs.
    PipelineCycle,
}
impl InstanceDispatcher {
    /// Configure an idle fixed owner (v7 handoff, v8 coalesced, v9 cycle, v10 preparation, v11 pipeline).
    /// Scheme retains result graphs until projection; Rust never moves native tasks.
    /// # Errors
    /// Closed/unsupported service, non-idle work, or invalid native receipt.
    pub fn configure_scheme_service_completion_on(
        &self,
        owner: usize,
        policy: SchemeServiceCompletion,
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
                .and_then(|o| o.wake.as_ref())
                .ok_or(InstanceError::Closed)?;
            if wake.closed.load(Ordering::Acquire) {
                return Err(InstanceError::Closed);
            }
            if wake.service_version.load(Ordering::Acquire)
                < match policy {
                    SchemeServiceCompletion::PipelineCycle => 11,
                    SchemeServiceCompletion::PreparedCycle => 10,
                    SchemeServiceCompletion::NativeCycle => 9,
                    SchemeServiceCompletion::CoalescedHandoff => 8,
                    _ => 7,
                }
            {
                return Err(InstanceError::InvalidConfig(
                    "Scheme service lacks completion handoff",
                ));
            }
            std::sync::Arc::clone(wake)
        };
        let mode = match policy {
            SchemeServiceCompletion::Snapshot => 0,
            SchemeServiceCompletion::FairHandoff => 1,
            SchemeServiceCompletion::CoalescedHandoff => 2,
            SchemeServiceCompletion::NativeCycle => 3,
            SchemeServiceCompletion::PreparedCycle => 4,
            SchemeServiceCompletion::PipelineCycle => 5,
        };
        self.submit_control_on(owner, move |session| {
            if mode >= 3 && session.image.native.result_slot_take.is_none() {
                return Err(InstanceError::InvalidConfig(
                    "native result slot ABI unavailable",
                ));
            }
            if mode == 5 && session.image.native.numeric_service_create.is_none() {
                return Err(InstanceError::InvalidConfig(
                    "native service constructor unavailable",
                ));
            }
            let value = super::service::command(session, &[13, mode], None)?;
            if session.copy_numeric::<i64>(&value)? != [mode] {
                return Err(InstanceError::InvalidConfig(
                    "native completion handoff receipt",
                ));
            }
            wake.native_cycle.store(mode >= 3, Ordering::Release);
            wake.prepared_cycle.store(mode >= 4, Ordering::Release);
            wake.pipeline_cycle.store(mode == 5, Ordering::Release);
            wake.coupled_refill.store(false, Ordering::Release);
            session.clear_entry()?;
            Ok(())
        })
    }
}

/// Native completion custody/counters read on a fixed owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeCompletionProgress {
    pub policy: SchemeServiceCompletion,
    pub epoch: u64,
    pub token_owned: bool,
    pub claimed: usize,
    pub opportunity_pending: bool,
    pub batches: u64,
    pub results: u64,
    pub ready_queued: usize,
}
impl InstanceDispatcher {
    /// Read actual completion policy and coalescence counters (service ABI v8).
    /// # Errors
    /// Unsupported/closed endpoint, capacity exhaustion or invalid metadata.
    pub fn scheme_service_completion_on(
        &self,
        owner: usize,
    ) -> Result<InstanceTicket<SchemeCompletionProgress>, InstanceError> {
        self.submit_control_on(owner, |session| {
            let value = super::service::command(session, &[15], None)?;
            let values = session.copy_numeric::<i64>(&value)?;
            SchemeCompletionProgress::parse(&values)
        })
    }
}

impl SchemeCompletionProgress {
    fn parse(values: &[i64]) -> Result<Self, InstanceError> {
        let [
            mode @ 0..=5,
            epoch @ 0..=i64::MAX,
            token @ -1..=i64::MAX,
            claimed @ 0..=65536,
            due @ 0..=1,
            batches @ 0..=i64::MAX,
            results @ 0..=i64::MAX,
            ready @ 0..=65536,
        ] = values
        else {
            return Err(InstanceError::InvalidConfig("native completion metadata"));
        };
        if token > epoch
            || claimed + ready > 65536
            || (*claimed > 0 && *token < 0)
            || (*token >= 0 && *due != 1)
            || (*mode < 2 && (*token >= 0 || *claimed > 0 || *due != 0))
            || (*mode >= 2 && *ready > 0 && *due != 1)
        {
            return Err(InstanceError::InvalidConfig(
                "inconsistent completion custody",
            ));
        }
        Ok(SchemeCompletionProgress {
            policy: match mode {
                0 => SchemeServiceCompletion::Snapshot,
                1 => SchemeServiceCompletion::FairHandoff,
                2 => SchemeServiceCompletion::CoalescedHandoff,
                3 => SchemeServiceCompletion::NativeCycle,
                4 => SchemeServiceCompletion::PreparedCycle,
                _ => SchemeServiceCompletion::PipelineCycle,
            },
            epoch: u64::try_from(*epoch)
                .map_err(|_| InstanceError::InvalidConfig("negative completion epoch"))?,
            token_owned: *token >= 0,
            claimed: usize::try_from(*claimed)
                .map_err(|_| InstanceError::InvalidConfig("completion count overflow"))?,
            opportunity_pending: *due == 1,
            batches: u64::try_from(*batches)
                .map_err(|_| InstanceError::InvalidConfig("negative completion batches"))?,
            results: u64::try_from(*results)
                .map_err(|_| InstanceError::InvalidConfig("negative completion results"))?,
            ready_queued: usize::try_from(*ready)
                .map_err(|_| InstanceError::InvalidConfig("ready count overflow"))?,
        })
    }
}
#[cfg(test)]
#[path = "../../tests/unit/service_completion.rs"]
mod completion_tests;
