//! Explicit Scheme scheduling cost contract, independent of admission charges.
use super::{InstanceDispatcher, InstanceError, InstanceTicket};
use std::sync::atomic::Ordering;
/// Scheme owns both collection and green-thread scheduling at native boundaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemeServiceCyclePolicy {
    CheckpointFirst,
    ReadyFirst,
    /// Repay stalled execution at the existing single service boundary.
    ProgressFair,
    /// Same lifecycle observations as `RuntimeFair`, with ready-first selection.
    RuntimeReadyFirst,
    /// Class-aware runnable/lease/external-wait service-cycle selection.
    RuntimeFair,
    /// Event-only contexts; existing budget ownership remains authoritative.
    RuntimeEvent,
    /// Native Scheme readiness and existing budget/Actor ownership; no shadow contexts.
    NativeBudget,
}
/// Idle-boundary cumulative events, not execution time or ownership authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeServiceCycleProgress {
    pub enabled: bool,
    pub ready_first: bool,
    pub progress_fair: bool,
    pub stalled_ready_passes: u64,
    pub direct_collections: u64,
    pub scheduler_passes: u64,
    pub active_scheduler_passes: u64,
    pub short_collected: u64,
    pub bulk_collected: u64,
}
/// Scheme execution contexts, separate from jobs and completion roots.
/// Each phase pair is `[short, bulk]`; Actors own one context per active turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeRuntimeProgress {
    pub enabled: bool,
    pub fair: bool,
    pub runnable: [u64; 2],
    pub lease_wait: [u64; 2],
    pub external_wait: [u64; 2],
    pub actor_barrier: [u64; 2],
    pub running: [u64; 2],
    pub entered: u64,
    pub retired: u64,
    pub scheduler_passes: u64,
    pub native_wait_decisions: u64,
    pub ready_results: u64,
}
/// Cold inventory of native Scheme scheduling sources. This is neither a VM
/// presence flag nor a root, lease, thread-state or admission counter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeNativeProgress {
    pub enabled: bool,
    pub actor_sources: u64,
}
/// Cooperative logical work, never a wall-clock or arbitrary-code preemption bound.
/// A selected uninterrupted span may finish before new arrivals get a turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeWorkExecutionPolicy {
    pub contended_work: usize,
    pub maximum_uninterrupted_work: usize,
}
/// Scheme-selected plans; observations do not settle roots or capacity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeWorkPlanObservation {
    pub contended_work: u64,
    pub maximum_uninterrupted_work: u64,
    pub whole_plans: u64,
    pub sliced_plans: u64,
    pub maximum_selected_work: u64,
}
/// Future Scheme class permissions, independent of an already-selected span.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeLeasePlanObservation {
    pub enabled: bool,
    pub short_span: u64,
    pub bulk_span: u64,
}
impl InstanceDispatcher {
    /// Select an idle ABI15 policy. `(0,0)` disables; positive policies require
    /// existing cooperative arbitration. New arrivals can wait for one already
    /// selected span. Continuation reentry retains that declared span bound.
    /// Selecting any completion policy disables the work policy.
    /// # Errors
    /// Invalid bounds, unsupported/closed owner, or non-idle Scheme service.
    pub fn configure_scheme_service_work_plan_on(
        &self,
        owner: usize,
        policy: SchemeWorkExecutionPolicy,
    ) -> Result<InstanceTicket<SchemeWorkExecutionPolicy>, InstanceError> {
        self.configure_work_plan(owner, policy, 0)
    }
    /// Use ABI16 Scheme lease-owned plans with cold layout validation and
    /// event-driven invalidation. Whole terminal declarations are accounted at release.
    /// Each successful continuation return charges its retained declaration; this
    /// is conservative logical work, not an instruction or partial-replay count.
    /// Already-selected spans and stale-policy continuation fences are retained.
    /// # Errors
    /// Invalid bounds, unsupported/closed owner, or non-idle Scheme service.
    pub fn configure_scheme_service_lease_plan_on(
        &self,
        owner: usize,
        policy: SchemeWorkExecutionPolicy,
    ) -> Result<InstanceTicket<SchemeWorkExecutionPolicy>, InstanceError> {
        self.configure_work_plan(owner, policy, 1)
    }
    /// ABI17 immutable plan metadata with independent continuation activation state.
    /// Bounds, class arbitration and independent root/capacity retirement are retained.
    /// # Errors
    /// Invalid bounds, unsupported/closed owner, or non-idle service.
    pub fn configure_scheme_service_activation_plan_on(
        &self,
        owner: usize,
        policy: SchemeWorkExecutionPolicy,
    ) -> Result<InstanceTicket<SchemeWorkExecutionPolicy>, InstanceError> {
        self.configure_work_plan(owner, policy, 2)
    }
    /// ABI18 successful Scheme returns accrue work to the owning lease; outer
    /// release/suspension settles it. Selection reports once at first entry.
    /// Multishot returns each credit their retained declaration; exceptions do not.
    /// # Errors
    /// Invalid bounds, unsupported/closed owner, or non-idle service.
    pub fn configure_scheme_service_cycle_plan_on(
        &self,
        owner: usize,
        policy: SchemeWorkExecutionPolicy,
    ) -> Result<InstanceTicket<SchemeWorkExecutionPolicy>, InstanceError> {
        self.configure_work_plan(owner, policy, 3)
    }
    /// ABI19 producer-declared Scheme Actor turns own an outer lease; item winds borrow it.
    /// Only the declared-work Actor factory opts in; generic Actors retain their unit protocol.
    /// Existing Actor yields release the lease. Outstanding invocation spans
    /// survive suspension; old continuation bounds remain cooperative.
    /// # Errors
    /// Invalid bounds, unsupported/closed owner, or non-idle service.
    pub fn configure_scheme_service_actor_turn_on(
        &self,
        owner: usize,
        policy: SchemeWorkExecutionPolicy,
    ) -> Result<InstanceTicket<SchemeWorkExecutionPolicy>, InstanceError> {
        self.configure_work_plan(owner, policy, 4)
    }
    /// ABI20 shares immutable layout entry/exit hooks. The Scheme continuation
    /// retains the selected plan; every successful replay credits work, while
    /// exceptions release ownership without terminal credit. No outer turn lease.
    /// # Errors
    /// Invalid bounds, unsupported/closed owner, or non-idle service.
    pub fn configure_scheme_service_specialized_plan_on(
        &self,
        owner: usize,
        policy: SchemeWorkExecutionPolicy,
    ) -> Result<InstanceTicket<SchemeWorkExecutionPolicy>, InstanceError> {
        self.configure_work_plan(owner, policy, 5)
    }
    /// ABI21 gives unharvested short results the next Scheme service opportunity.
    /// Worker grants resume after the actual ready harvest, before projection;
    /// roots and admission permits retain their independent retirement rules.
    /// Native shutdown revokes the fence so draining needs no host round trip.
    /// Previously-issued grants, selected spans and multishot bounds remain.
    /// # Errors
    /// Invalid bounds, unsupported/closed owner, or non-idle service.
    pub fn configure_scheme_service_reclamation_plan_on(
        &self,
        owner: usize,
        policy: SchemeWorkExecutionPolicy,
    ) -> Result<InstanceTicket<SchemeWorkExecutionPolicy>, InstanceError> {
        self.configure_work_plan(owner, policy, 6)
    }
    /// ABI22 reserves completion priority only during an active Scheme harvest
    /// turn. Receiver departure without harvest preserves ready custody while
    /// restoring worker grants; no future Rust call is an execution prerequisite.
    /// The service-cycle receiver is continuation-safe and revoked by close.
    /// # Errors
    /// Invalid bounds, unsupported/closed owner, or non-idle service.
    pub fn configure_scheme_service_receiver_plan_on(
        &self,
        owner: usize,
        policy: SchemeWorkExecutionPolicy,
    ) -> Result<InstanceTicket<SchemeWorkExecutionPolicy>, InstanceError> {
        self.configure_work_plan(owner, policy, 7)
    }
    /// ABI23 retains short execution debt through runnable and registered states.
    /// Only the receiving Scheme thread's first actual grant settles execution;
    /// early terminal failure has a separate receipt. Dispatch remains independent.
    /// # Errors
    /// Invalid bounds, unsupported/closed owner, or non-idle service.
    pub fn configure_scheme_service_registered_plan_on(
        &self,
        owner: usize,
        policy: SchemeWorkExecutionPolicy,
    ) -> Result<InstanceTicket<SchemeWorkExecutionPolicy>, InstanceError> {
        self.configure_work_plan(owner, policy, 8)
    }
    /// ABI24 gives each contended Scheme acquisition a private wake token.
    /// Registration, class arbitration and first-grant settlement retain ABI23
    /// semantics. Uncontended acquisition allocates no notification token.
    /// # Errors
    /// Invalid bounds, unsupported/closed owner, or non-idle service.
    pub fn configure_scheme_service_directed_plan_on(
        &self,
        owner: usize,
        policy: SchemeWorkExecutionPolicy,
    ) -> Result<InstanceTicket<SchemeWorkExecutionPolicy>, InstanceError> {
        self.configure_work_plan(owner, policy, 9)
    }
    /// ABI25 keeps bounded Scheme service checkpoints eligible until a short
    /// worker receives its first execution grant, independently of dispatch.
    /// It neither waits for handler completion nor adds per-handler polling.
    /// # Errors
    /// Invalid bounds, unsupported/closed owner, or non-idle service.
    pub fn configure_scheme_service_execution_checkpoint_plan_on(
        &self,
        owner: usize,
        policy: SchemeWorkExecutionPolicy,
    ) -> Result<InstanceTicket<SchemeWorkExecutionPolicy>, InstanceError> {
        self.configure_work_plan(owner, policy, 10)
    }
    /// Explicit matched service-cycle policies. `ReadyFirst` collects actionable
    /// results before scheduling, and permits at most one explicit scheduler
    /// pass in an empty harvest. `ProgressFair` also offers that single pass when
    /// ready results coexist with unfinished work but no logical execution
    /// advanced since the last harvest. No policy acknowledges execution by yield.
    /// # Errors
    /// Invalid bounds, unsupported/closed owner, or non-idle service.
    pub fn configure_scheme_service_boundary_plan_on(
        &self,
        owner: usize,
        work: SchemeWorkExecutionPolicy,
        policy: SchemeServiceCyclePolicy,
    ) -> Result<InstanceTicket<SchemeWorkExecutionPolicy>, InstanceError> {
        self.configure_work_plan(
            owner,
            work,
            match policy {
                SchemeServiceCyclePolicy::CheckpointFirst => 11,
                SchemeServiceCyclePolicy::ReadyFirst => 12,
                SchemeServiceCyclePolicy::ProgressFair => 13,
                SchemeServiceCyclePolicy::RuntimeReadyFirst => 14,
                SchemeServiceCyclePolicy::RuntimeFair => 15,
                SchemeServiceCyclePolicy::RuntimeEvent => 16,
                SchemeServiceCyclePolicy::NativeBudget => 17,
            },
        )
    }
    /// Read native boundary decisions; query at idle for conservation checks.
    /// # Errors
    /// Unsupported/closed owner or malformed native counters.
    pub fn scheme_service_cycle_progress_on(
        &self,
        owner: usize,
    ) -> Result<InstanceTicket<SchemeServiceCycleProgress>, InstanceError> {
        self.work_plan_version(owner, 12)?;
        self.submit_control_on(owner, |session| {
            let value = super::service::command(session, &[41], None)?;
            let values = session
                .copy_numeric::<i64>(&value)?
                .into_iter()
                .map(u64::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| InstanceError::InvalidConfig("cycle progress range"))?;
            let [
                enabled,
                ready_first,
                direct,
                passes,
                short,
                bulk,
                active,
                progress_fair,
                stalled,
            ] = values.as_slice()
            else {
                return Err(InstanceError::InvalidConfig("cycle progress shape"));
            };
            if *progress_fair > 1
                || (*progress_fair == 1 && *ready_first != 1)
                || *stalled > *active
                || *active > *passes
                || *enabled > 1
                || *ready_first > 1
                || (*enabled == 0 && values.iter().any(|v| *v != 0))
                || *direct
                    > short
                        .checked_add(*bulk)
                        .ok_or(InstanceError::InvalidConfig("cycle progress overflow"))?
            {
                return Err(InstanceError::InvalidConfig("cycle progress conservation"));
            }
            Ok(SchemeServiceCycleProgress {
                enabled: *enabled == 1,
                ready_first: *ready_first == 1,
                progress_fair: *progress_fair == 1,
                stalled_ready_passes: *stalled,
                direct_collections: *direct,
                scheduler_passes: *passes,
                active_scheduler_passes: *active,
                short_collected: *short,
                bulk_collected: *bulk,
            })
        })
    }
    /// Observe Scheme lifecycle states and native completion custody. This
    /// receipt neither transfers roots nor proves CPU execution or OS readiness.
    /// # Errors
    /// Unsupported/closed owner or malformed/nonconserving native receipt.
    pub fn scheme_runtime_progress_on(
        &self,
        owner: usize,
    ) -> Result<InstanceTicket<SchemeRuntimeProgress>, InstanceError> {
        self.work_plan_version(owner, 14)?;
        self.submit_control_on(owner, |session| {
            let value = super::service::command(session, &[45], None)?;
            let v = session
                .copy_numeric::<i64>(&value)?
                .into_iter()
                .map(u64::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| InstanceError::InvalidConfig("runtime progress range"))?;
            let x: [u64; 17] = v
                .try_into()
                .map_err(|_| InstanceError::InvalidConfig("runtime progress shape"))?;
            let active = x[2..12].iter().try_fold(0_u64, |n, v| n.checked_add(*v));
            if x[0] > 1
                || x[1] > x[0]
                || active.and_then(|n| n.checked_add(x[13])) != Some(x[12])
                || (x[0] == 0 && x[..16].iter().any(|v| *v != 0))
            {
                return Err(InstanceError::InvalidConfig(
                    "runtime progress conservation",
                ));
            }
            Ok(SchemeRuntimeProgress {
                enabled: x[0] == 1,
                fair: x[1] == 1,
                runnable: [x[2], x[3]],
                lease_wait: [x[4], x[5]],
                external_wait: [x[6], x[7]],
                actor_barrier: [x[8], x[9]],
                running: [x[10], x[11]],
                entered: x[12],
                retired: x[13],
                scheduler_passes: x[14],
                native_wait_decisions: x[15],
                ready_results: x[16],
            })
        })
    }
    /// Observe the ABI32 native scheduling policy without enabling context tracking.
    /// # Errors
    /// Unsupported/closed owner or malformed native inventory.
    pub fn scheme_native_progress_on(
        &self,
        owner: usize,
    ) -> Result<InstanceTicket<SchemeNativeProgress>, InstanceError> {
        self.work_plan_version(owner, 17)?;
        self.submit_control_on(owner, |session| {
            let value = super::service::command(session, &[48], None)?;
            let v = session.copy_numeric::<i64>(&value)?;
            match v.as_slice() {
                [enabled @ 0..=1, count] if *count >= 0 && (*enabled == 1 || *count == 0) => {
                    Ok(SchemeNativeProgress {
                        enabled: *enabled == 1,
                        actor_sources: u64::try_from(*count)
                            .map_err(|_| InstanceError::InvalidConfig("native inventory range"))?,
                    })
                }
                _ => Err(InstanceError::InvalidConfig("native inventory shape")),
            }
        })
    }
    /// Read Scheme-owned first-execution states for short service requests.
    /// Cumulative transitions describe protocol events, not OS runnable time.
    /// # Errors
    /// Unsupported/closed owner or a malformed/inconsistent native receipt.
    pub fn scheme_service_execution_on(
        &self,
        owner: usize,
    ) -> Result<InstanceTicket<super::SchemeExecutionProgress>, InstanceError> {
        self.work_plan_version(owner, 8)?;
        self.submit_control_on(owner, |session| {
            let value = super::service::command(session, &[36], None)?;
            let v = session
                .copy_numeric::<i64>(&value)?
                .into_iter()
                .map(u64::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| InstanceError::InvalidConfig("execution progress count"))?;
            let [
                enabled,
                pending,
                issued,
                runnable,
                registered,
                granted,
                ungranted,
            ] = v.as_slice()
            else {
                return Err(InstanceError::InvalidConfig("execution progress shape"));
            };
            if *enabled > 1
                || registered > runnable
                || runnable > issued
                || granted > registered
                || pending
                    .checked_add(*granted)
                    .and_then(|n| n.checked_add(*ungranted))
                    != Some(*issued)
                || (*enabled == 0 && v.iter().any(|n| *n != 0))
            {
                return Err(InstanceError::InvalidConfig(
                    "execution progress conservation",
                ));
            }
            Ok(super::SchemeExecutionProgress {
                enabled: *enabled == 1,
                pending_execution: *pending,
                issued: *issued,
                runnable: *runnable,
                registered: *registered,
                first_grants: *granted,
                finished_without_grant: *ungranted,
            })
        })
    }
    fn configure_work_plan(
        &self,
        owner: usize,
        policy: SchemeWorkExecutionPolicy,
        mode: u8,
    ) -> Result<InstanceTicket<SchemeWorkExecutionPolicy>, InstanceError> {
        self.work_plan_version(owner, mode)?;
        let q = policy.contended_work;
        let m = policy.maximum_uninterrupted_work;
        if !((q == 0 && m == 0) || (q > 0 && q <= 256 && q <= m && m <= 65536)) {
            return Err(InstanceError::InvalidConfig("Scheme work plan cost bounds"));
        }
        let q = i64::try_from(q).map_err(|_| InstanceError::InvalidConfig("work quantum"))?;
        let m = i64::try_from(m).map_err(|_| InstanceError::InvalidConfig("work maximum"))?;
        self.submit_control_on(owner, move |session| {
            let v = super::service::command(
                session,
                &[
                    match mode {
                        0 => 24,
                        1 => 26,
                        2 => 28,
                        3 => 29,
                        4 => 30,
                        5 => 32,
                        6 => 33,
                        7 => 34,
                        8 => 35,
                        9 => 37,
                        10 => 38,
                        11 => 39,
                        12 => 40,
                        13 => 42,
                        14 => 43,
                        15 => 44,
                        16 => 46,
                        _ => 47,
                    },
                    q,
                    m,
                ],
                None,
            )?;
            let expected = if mode > 0 {
                vec![q, m, if q > 0 { i64::from(mode) } else { 0 }]
            } else {
                vec![q, m]
            };
            if session.copy_numeric::<i64>(&v)? != expected {
                return Err(InstanceError::InvalidConfig("work plan receipt"));
            }
            Ok(policy)
        })
    }
    /// Observe selected work plans through the owner. Use idle boundaries for
    /// exact conservation; this is not an OS scheduler or collector CPU sample.
    /// # Errors
    /// Unsupported/closed owner or malformed native metadata.
    pub fn scheme_service_work_plan_on(
        &self,
        owner: usize,
    ) -> Result<InstanceTicket<SchemeWorkPlanObservation>, InstanceError> {
        self.work_plan_version(owner, 0)?;
        self.submit_control_on(owner, |session| {
            let v = super::service::command(session, &[25], None)?;
            let v: Vec<u64> = session
                .copy_numeric::<i64>(&v)?
                .into_iter()
                .map(u64::try_from)
                .collect::<Result<_, _>>()
                .map_err(|_| InstanceError::InvalidConfig("work plan count"))?;
            let [q, m, fast, contended, selected] = v.as_slice() else {
                return Err(InstanceError::InvalidConfig("work plan shape"));
            };
            if !((*q == 0 && *m == 0 && *selected == 0)
                || (*q > 0 && *q <= 256 && q <= m && *m <= 65536 && selected <= m))
            {
                return Err(InstanceError::InvalidConfig("work plan state"));
            }
            Ok(SchemeWorkPlanObservation {
                contended_work: *q,
                maximum_uninterrupted_work: *m,
                whole_plans: *fast,
                sliced_plans: *contended,
                maximum_selected_work: *selected,
            })
        })
    }
    /// ABI16 cached class permissions: enabled flag, short span, bulk span.
    /// These are future permissions; an already-selected span does not change.
    /// # Errors
    /// Unsupported/closed owner or malformed native metadata.
    pub fn scheme_service_lease_plan_on(
        &self,
        owner: usize,
    ) -> Result<InstanceTicket<SchemeLeasePlanObservation>, InstanceError> {
        self.work_plan_version(owner, 1)?;
        self.submit_control_on(owner, |session| {
            let v = super::service::command(session, &[27], None)?;
            let v = session.copy_numeric::<i64>(&v)?;
            match v.as_slice() {
                [0, 0, 0] => Ok(SchemeLeasePlanObservation {
                    enabled: false,
                    short_span: 0,
                    bulk_span: 0,
                }),
                [1, short, bulk] if (1..=65536).contains(short) && (1..=65536).contains(bulk) => {
                    Ok(SchemeLeasePlanObservation {
                        enabled: true,
                        short_span: short.unsigned_abs(),
                        bulk_span: bulk.unsigned_abs(),
                    })
                }
                _ => Err(InstanceError::InvalidConfig("lease cache shape/range")),
            }
        })
    }
    pub(super) fn work_plan_version(&self, owner: usize, mode: u8) -> Result<(), InstanceError> {
        let s = self
            .admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let wake = s
            .owners
            .get(owner)
            .and_then(|o| o.wake.as_ref())
            .ok_or(InstanceError::Closed)?;
        if wake.service_version.load(Ordering::Acquire) < 15 + usize::from(mode) {
            return Err(InstanceError::InvalidConfig(
                "work plans require ABI15; lease ABI16; shared metadata ABI17; lease settlement ABI18; Actor turn ABI19; specialized ABI20; reclamation ABI21; receiver ABI22; registered ABI23; directed ABI24; execution checkpoint ABI25; cycle ABI26/27; progress fair ABI28; observed Runtime ABI29; joint Runtime ABI30; event Runtime ABI31; native budget ABI32",
            ));
        }
        Ok(())
    }
}
