//! Exhaustive strategy mapping plus independent native readback before timing.
use super::{GATE, budget, measurement::Comparison};
use gerbil_scheme::instances::{InstancePool, SchemeServiceSettings, SchemeServiceWake};
fn requested(comparison: Comparison, arm: bool) -> SchemeServiceSettings {
    let (enabled, direct) = match comparison {
        Comparison::Service
        | Comparison::Combined
        | Comparison::ShortLayout
        | Comparison::ClassRouting
        | Comparison::ShortService
        | Comparison::Feed => (false, false),
        Comparison::Budget => (arm, false),
        Comparison::Ingress => (true, arm),
        Comparison::Notification
        | Comparison::Reselection
        | Comparison::OwnerQos
        | Comparison::Resident
        | Comparison::Pooled
        | Comparison::ObserverCost
        | Comparison::ShortRouteLoad
        | Comparison::SupplyAdmission
        | Comparison::PoolProtocol
        | Comparison::SelectiveWake
        | Comparison::RetirementRefill
        | Comparison::ProgressFair => (true, true),
        Comparison::PooledWake => (true, !arm),
    };
    SchemeServiceSettings {
        cooperative_budget: usize::from(enabled),
        wake: if direct {
            SchemeServiceWake::NativePort
        } else {
            SchemeServiceWake::Watcher
        },
    }
}
pub(super) fn configure(
    pool: &InstancePool,
    comparison: Comparison,
    arm: bool,
) -> Vec<SchemeServiceSettings> {
    let expected = requested(comparison, arm);
    budget::configure(pool, expected.cooperative_budget > 0);
    budget::wake(pool, expected.wake == SchemeServiceWake::NativePort);
    budget::notification(pool, comparison == Comparison::Notification && arm);
    if comparison == Comparison::Feed {
        // Native delivery precedes permit retirement; feed changes require both.
        super::measurement::idle(pool);
        for owner in 0..4 {
            pool.dispatcher()
                .configure_scheme_service_feed_on(owner, if arm { 4 } else { 1 })
                .unwrap();
        }
    }
    if completion_ab() {
        assert!(comparison == Comparison::ProgressFair && fixed_short_selective());
        super::measurement::idle(pool);
        for owner in 0..4 {
            use gerbil_scheme::instances::SchemeServiceCompletion;
            let policy = if coupled_refill_ab() || native_entry_ab() {
                SchemeServiceCompletion::PipelineCycle
            } else if pipeline_cycle_ab() {
                if arm {
                    SchemeServiceCompletion::PipelineCycle
                } else {
                    SchemeServiceCompletion::PreparedCycle
                }
            } else if prepared_cycle_ab() {
                if arm {
                    SchemeServiceCompletion::PreparedCycle
                } else {
                    SchemeServiceCompletion::NativeCycle
                }
            } else if arm && native_cycle_ab() {
                SchemeServiceCompletion::NativeCycle
            } else if arm && completion_coalesced_ab() {
                SchemeServiceCompletion::CoalescedHandoff
            } else if arm {
                SchemeServiceCompletion::FairHandoff
            } else {
                SchemeServiceCompletion::Snapshot
            };
            pool.dispatcher()
                .configure_scheme_service_completion_on(owner, policy)
                .unwrap()
                .wait(GATE)
                .unwrap();
            assert_eq!(
                pool.dispatcher()
                    .scheme_service_completion_on(owner)
                    .unwrap()
                    .wait(GATE)
                    .unwrap()
                    .policy,
                policy
            );
            if native_entry_ab() {
                assert_eq!(
                    pool.dispatcher()
                        .configure_scheme_service_native_entry_on(
                            owner,
                            arm || joint_progress_ab() || work_budget_ab()
                        )
                        .unwrap()
                        .wait(GATE)
                        .unwrap(),
                    arm || joint_progress_ab() || work_budget_ab()
                );
            }
            if joint_progress_ab() || work_budget_ab() {
                assert_eq!(
                    pool.dispatcher()
                        .configure_scheme_service_joint_progress_on(owner, joint_enabled(arm))
                        .unwrap()
                        .wait(GATE)
                        .unwrap(),
                    joint_enabled(arm)
                );
            }
            configure_work(pool, owner, arm);
            configure_publication(pool, owner, arm);
            if coupled_refill_ab() {
                assert_eq!(
                    pool.dispatcher()
                        .configure_scheme_service_refill_on(owner, arm)
                        .unwrap()
                        .wait(GATE)
                        .unwrap(),
                    arm
                );
            }
        }
    }
    let actual = read(pool);
    assert!(
        actual.iter().all(|settings| *settings == expected),
        "native policy mismatch: expected {expected:?}, actual {actual:?}"
    );
    actual
}
fn read(pool: &InstancePool) -> Vec<SchemeServiceSettings> {
    let tickets: Vec<_> = (0..4)
        .map(|owner| pool.dispatcher().scheme_service_settings_on(owner).unwrap())
        .collect();
    tickets
        .into_iter()
        .map(|ticket| ticket.wait(GATE).unwrap())
        .collect()
}

pub(super) fn short_pool(pool: &InstancePool, comparison: Comparison) {
    if matches!(
        comparison,
        Comparison::PoolProtocol
            | Comparison::SelectiveWake
            | Comparison::RetirementRefill
            | Comparison::ProgressFair
    ) {
        pool.dispatcher()
            .configure_scheme_short_pool(gerbil_scheme::instances::SchemeShortPoolConfig {
                owners: vec![0, 1, 2, 3],
                maximum_pending: 128,
                maximum_active_per_owner: 1,
            })
            .unwrap();
    }
}

pub(super) fn short_wake(pool: &InstancePool, comparison: Comparison, arm: bool) {
    use gerbil_scheme::instances::SchemeShortWake;
    let policy = match comparison {
        Comparison::ProgressFair if arm && !fixed_short_selective() => {
            SchemeShortWake::ProgressFair
        }
        Comparison::ProgressFair | Comparison::RetirementRefill => SchemeShortWake::Selective,
        Comparison::SelectiveWake => {
            if arm {
                SchemeShortWake::Selective
            } else {
                SchemeShortWake::Broadcast
            }
        }
        _ => return,
    };
    let facility = pool.dispatcher().scheme_short_pool().unwrap();
    facility.configure_wake(policy).unwrap();
    assert_eq!(facility.wake_policy(), policy);
    // Same native transition boundary outside timing in both arms.
    read(pool);
}

pub(super) fn progress(pool: &InstancePool, comparison: Comparison) {
    if comparison == Comparison::ProgressFair {
        for owner in 0..4 {
            pool.dispatcher()
                .enable_scheme_service_progress_on(owner)
                .unwrap()
                .wait(GATE)
                .unwrap();
            assert!(
                pool.dispatcher()
                    .scheme_service_progress_on(owner)
                    .unwrap()
                    .enabled
            );
        }
        super::measurement::idle(pool);
    }
}

/// Isolate bulk offer ownership while keeping both short-routing arms identical.
pub(super) fn fixed_short_selective() -> bool {
    std::env::args().any(|arg| arg == "--fixed-short-selective")
}

/// Isolate completion handoff with identical `ProgressFair` supply in both arms.
pub(super) fn completion_fair_ab() -> bool {
    std::env::args().any(|arg| arg == "--completion-fair-ab")
}

pub(super) fn completion_coalesced_ab() -> bool {
    std::env::args().any(|arg| arg == "--completion-coalesced-ab")
}
pub(super) fn completion_ab() -> bool {
    completion_fair_ab()
        || completion_coalesced_ab()
        || native_cycle_ab()
        || prepared_cycle_ab()
        || pipeline_cycle_ab()
        || coupled_refill_ab()
        || native_entry_ab()
}
/// Outside the timing interval; actual native counters and quiescence checks.
pub(super) fn completions(pool: &InstancePool) -> [u64; 3] {
    let mut result = [0; 3];
    for owner in 0..4 {
        let actual = pool
            .dispatcher()
            .scheme_service_completion_on(owner)
            .unwrap()
            .wait(GATE)
            .unwrap();
        assert!(
            !actual.token_owned
                && !actual.opportunity_pending
                && actual.claimed == 0
                && actual.ready_queued == 0
        );
        for (sum, value) in result
            .iter_mut()
            .zip([actual.epoch, actual.batches, actual.results])
        {
            *sum += value;
        }
    }
    result
}

pub(super) fn native_cycle_ab() -> bool {
    std::env::args().any(|arg| arg == "--native-cycle-ab")
}

pub(super) fn prepared_cycle_ab() -> bool {
    std::env::args().any(|arg| arg == "--prepared-cycle-ab")
}

pub(super) fn batched_completion_ab() -> bool {
    completion_coalesced_ab()
        || native_cycle_ab()
        || prepared_cycle_ab()
        || pipeline_cycle_ab()
        || coupled_refill_ab()
        || native_entry_ab()
}

pub(super) fn pipeline_cycle_ab() -> bool {
    std::env::args().any(|arg| arg == "--pipeline-cycle-ab")
}

/// Same native constructor and completion ABI; only owner refill arbitration differs.
pub(super) fn coupled_refill_ab() -> bool {
    std::env::args().any(|arg| arg == "--coupled-refill-ab")
}

pub(super) fn native_entry_ab() -> bool {
    std::env::args().any(|arg| {
        arg == "--native-entry-ab"
            || arg == "--joint-progress-ab"
            || arg == "--work-budget-ab"
            || arg == "--work-plan-ab"
            || arg == "--lease-plan-ab"
            || arg == "--activation-plan-ab"
            || arg == "--cycle-accounting-ab"
            || arg == "--actor-turn-ab"
            || arg == "--specialized-plan-ab"
            || arg == "--reclamation-plan-ab"
            || arg == "--receiver-plan-ab"
            || arg == "--registered-plan-ab"
            || arg == "--directed-plan-ab"
            || arg == "--execution-checkpoint-ab"
            || arg == "--service-cycle-plan-ab"
            || arg == "--progress-cycle-ab"
            || arg == "--runtime-joint-ab"
            || arg == "--runtime-cost-ab"
            || arg == "--runtime-event-cost-ab"
            || arg == "--native-budget-cost-ab"
            || arg == "--client-arrival-cost-ab"
            || arg == "--retire-publication-ab"
            || arg == "--completion-await-ab"
            || arg == "--claim-progress-ab"
            || arg == "--ready-owner-ab"
            || arg == "--client-turn-ab"
            || arg == "--arrival-clock-ab"
            || arg == "--completion-service-ab"
            || arg == "--activation-protocol-ab"
            || arg == "--lease-plan-protocol-ab"
            || arg == "--work-plan-protocol-ab"
    })
}

pub(super) fn joint_progress_ab() -> bool {
    std::env::args().any(|arg| arg == "--joint-progress-ab")
}
/// After the complete workload, outside its timing window.
pub(super) fn joint_counts(pool: &InstancePool, arm: bool) -> [[u64; 3]; 4] {
    std::array::from_fn(|owner| {
        let value = pool
            .dispatcher()
            .scheme_service_joint_progress_on(owner)
            .unwrap()
            .wait(GATE)
            .unwrap();
        assert_eq!(value.enabled, arm);
        assert_eq!(value.pending_dispatch, 0);
        [
            value.pending_dispatch,
            value.checkpoints,
            value.worker_starts,
        ]
    })
}

pub(super) fn work_budget_ab() -> bool {
    std::env::args().any(|arg| {
        arg == "--work-budget-ab"
            || arg == "--work-plan-ab"
            || arg == "--lease-plan-ab"
            || arg == "--activation-plan-ab"
            || arg == "--cycle-accounting-ab"
            || arg == "--actor-turn-ab"
            || arg == "--specialized-plan-ab"
            || arg == "--reclamation-plan-ab"
            || arg == "--receiver-plan-ab"
            || arg == "--registered-plan-ab"
            || arg == "--directed-plan-ab"
            || arg == "--execution-checkpoint-ab"
            || arg == "--service-cycle-plan-ab"
            || arg == "--progress-cycle-ab"
            || arg == "--runtime-joint-ab"
            || arg == "--runtime-cost-ab"
            || arg == "--runtime-event-cost-ab"
            || arg == "--native-budget-cost-ab"
            || arg == "--client-arrival-cost-ab"
            || arg == "--retire-publication-ab"
            || arg == "--completion-await-ab"
            || arg == "--claim-progress-ab"
            || arg == "--ready-owner-ab"
            || arg == "--client-turn-ab"
            || arg == "--arrival-clock-ab"
            || arg == "--completion-service-ab"
            || arg == "--activation-protocol-ab"
            || arg == "--lease-plan-protocol-ab"
            || arg == "--work-plan-protocol-ab"
    })
}
pub(super) fn work_counts(pool: &InstancePool, arm: bool) -> [[u64; 7]; 4] {
    std::array::from_fn(|owner| {
        let w = pool
            .dispatcher()
            .scheme_service_work_on(owner)
            .unwrap()
            .wait(GATE)
            .unwrap();
        assert_eq!(w.quantum, if work_enabled(arm) { 4 } else { 0 });
        assert_eq!(w.pending_start, 0);
        [
            w.quantum,
            w.short_work,
            w.bulk_work,
            w.handoffs,
            w.pending_start,
            w.ready_generation,
            w.offered_generation,
        ]
    })
}

fn runtime_boundary_policy(arm: bool) -> gerbil_scheme::instances::SchemeServiceCyclePolicy {
    if retire_publication_ab() || client_arrival_cost_ab() || arm && native_budget_cost_ab() {
        gerbil_scheme::instances::SchemeServiceCyclePolicy::NativeBudget
    } else if arm && runtime_event_cost_ab() {
        gerbil_scheme::instances::SchemeServiceCyclePolicy::RuntimeEvent
    } else if arm {
        gerbil_scheme::instances::SchemeServiceCyclePolicy::RuntimeFair
    } else if runtime_cost_ab() || runtime_event_cost_ab() || native_budget_cost_ab() {
        gerbil_scheme::instances::SchemeServiceCyclePolicy::ReadyFirst
    } else {
        gerbil_scheme::instances::SchemeServiceCyclePolicy::RuntimeReadyFirst
    }
}

fn configure_work(pool: &InstancePool, owner: usize, arm: bool) {
    if work_budget_ab() {
        let quantum = if work_enabled(arm) { 4 } else { 0 };
        if work_plan_ab() {
            let policy = gerbil_scheme::instances::SchemeWorkExecutionPolicy {
                contended_work: quantum,
                maximum_uninterrupted_work: if work_enabled(arm) { 48 } else { 0 },
            };
            let ticket = if runtime_joint_ab()
                || (runtime_cost_ab() || runtime_event_cost_ab() || native_budget_cost_ab())
            {
                pool.dispatcher().configure_scheme_service_boundary_plan_on(
                    owner,
                    policy,
                    runtime_boundary_policy(arm),
                )
            } else if progress_cycle_ab() {
                pool.dispatcher().configure_scheme_service_boundary_plan_on(
                    owner,
                    policy,
                    if arm {
                        gerbil_scheme::instances::SchemeServiceCyclePolicy::ProgressFair
                    } else {
                        gerbil_scheme::instances::SchemeServiceCyclePolicy::ReadyFirst
                    },
                )
            } else if service_cycle_plan_ab() {
                pool.dispatcher().configure_scheme_service_boundary_plan_on(
                    owner,
                    policy,
                    if arm {
                        gerbil_scheme::instances::SchemeServiceCyclePolicy::ReadyFirst
                    } else {
                        gerbil_scheme::instances::SchemeServiceCyclePolicy::CheckpointFirst
                    },
                )
            } else if execution_checkpoint_ab() && arm {
                pool.dispatcher()
                    .configure_scheme_service_execution_checkpoint_plan_on(owner, policy)
            } else if directed_plan_ab() && arm {
                pool.dispatcher()
                    .configure_scheme_service_directed_plan_on(owner, policy)
            } else if execution_checkpoint_ab()
                || directed_plan_ab()
                || (registered_plan_ab() && arm)
            {
                pool.dispatcher()
                    .configure_scheme_service_registered_plan_on(owner, policy)
            } else if receiver_plan_ab() && arm {
                pool.dispatcher()
                    .configure_scheme_service_receiver_plan_on(owner, policy)
            } else if reclamation_plan_ab() && arm {
                pool.dispatcher()
                    .configure_scheme_service_reclamation_plan_on(owner, policy)
            } else if registered_plan_ab()
                || receiver_plan_ab()
                || reclamation_plan_ab()
                || (specialized_plan_ab() && arm)
            {
                pool.dispatcher()
                    .configure_scheme_service_specialized_plan_on(owner, policy)
            } else if specialized_plan_ab() {
                pool.dispatcher()
                    .configure_scheme_service_cycle_plan_on(owner, policy)
            } else if actor_turn_ab() && arm {
                pool.dispatcher()
                    .configure_scheme_service_actor_turn_on(owner, policy)
            } else if actor_turn_ab() || (cycle_accounting_ab() && arm) {
                pool.dispatcher()
                    .configure_scheme_service_cycle_plan_on(owner, policy)
            } else if cycle_accounting_ab() || (activation_plan_ab() && arm) {
                pool.dispatcher()
                    .configure_scheme_service_activation_plan_on(owner, policy)
            } else if lease_plan_ab() {
                pool.dispatcher()
                    .configure_scheme_service_lease_plan_on(owner, policy)
            } else {
                pool.dispatcher()
                    .configure_scheme_service_work_plan_on(owner, policy)
            };
            assert_eq!(ticket.unwrap().wait(GATE).unwrap(), policy);
            return;
        }
        assert_eq!(
            pool.dispatcher()
                .configure_scheme_service_work_on(owner, quantum)
                .unwrap()
                .wait(GATE)
                .unwrap(),
            quantum as u64
        );
    }
}

pub(super) fn work_plan_ab() -> bool {
    std::env::args().any(|arg| {
        arg == "--work-plan-ab"
            || arg == "--lease-plan-ab"
            || arg == "--activation-plan-ab"
            || arg == "--cycle-accounting-ab"
            || arg == "--actor-turn-ab"
            || arg == "--specialized-plan-ab"
            || arg == "--reclamation-plan-ab"
            || arg == "--receiver-plan-ab"
            || arg == "--registered-plan-ab"
            || arg == "--directed-plan-ab"
            || arg == "--execution-checkpoint-ab"
            || arg == "--service-cycle-plan-ab"
            || arg == "--progress-cycle-ab"
            || arg == "--runtime-joint-ab"
            || arg == "--runtime-cost-ab"
            || arg == "--runtime-event-cost-ab"
            || arg == "--native-budget-cost-ab"
            || arg == "--client-arrival-cost-ab"
            || arg == "--retire-publication-ab"
            || arg == "--completion-await-ab"
            || arg == "--claim-progress-ab"
            || arg == "--ready-owner-ab"
            || arg == "--client-turn-ab"
            || arg == "--arrival-clock-ab"
            || arg == "--completion-service-ab"
            || arg == "--activation-protocol-ab"
            || arg == "--lease-plan-protocol-ab"
            || arg == "--work-plan-protocol-ab"
    })
}
pub(super) fn plan_counts(pool: &InstancePool, arm: bool) -> [[u64; 5]; 4] {
    std::array::from_fn(|owner| {
        let p = pool
            .dispatcher()
            .scheme_service_work_plan_on(owner)
            .unwrap()
            .wait(GATE)
            .unwrap();
        if lease_plan_ab() {
            let cached = pool
                .dispatcher()
                .scheme_service_lease_plan_on(owner)
                .unwrap()
                .wait(GATE)
                .unwrap();
            assert_eq!(cached.enabled, work_enabled(arm));
            assert_eq!(
                (cached.short_span, cached.bulk_span),
                if work_enabled(arm) { (48, 48) } else { (0, 0) }
            );
        }
        assert_eq!(p.contended_work, if work_enabled(arm) { 4 } else { 0 });
        assert_eq!(
            p.maximum_uninterrupted_work,
            if work_enabled(arm) { 48 } else { 0 }
        );
        assert!(p.maximum_selected_work <= p.maximum_uninterrupted_work);
        [
            p.contended_work,
            p.maximum_uninterrupted_work,
            p.whole_plans,
            p.sliced_plans,
            p.maximum_selected_work,
        ]
    })
}

pub(super) fn work_plan_protocol_ab() -> bool {
    std::env::args().any(|arg| {
        matches!(
            arg.as_str(),
            "--work-plan-protocol-ab" | "--lease-plan-protocol-ab" | "--activation-protocol-ab"
        )
    })
}
pub(super) fn joint_enabled(arm: bool) -> bool {
    if work_plan_protocol_ab() {
        arm
    } else {
        (joint_progress_ab() && arm) || work_budget_ab()
    }
}

pub(super) fn lease_plan_ab() -> bool {
    std::env::args().any(|arg| {
        arg == "--lease-plan-ab"
            || arg == "--activation-plan-ab"
            || arg == "--cycle-accounting-ab"
            || arg == "--actor-turn-ab"
            || arg == "--specialized-plan-ab"
            || arg == "--reclamation-plan-ab"
            || arg == "--receiver-plan-ab"
            || arg == "--registered-plan-ab"
            || arg == "--directed-plan-ab"
            || arg == "--execution-checkpoint-ab"
            || arg == "--service-cycle-plan-ab"
            || arg == "--progress-cycle-ab"
            || arg == "--runtime-joint-ab"
            || arg == "--runtime-cost-ab"
            || arg == "--runtime-event-cost-ab"
            || arg == "--native-budget-cost-ab"
            || arg == "--client-arrival-cost-ab"
            || arg == "--retire-publication-ab"
            || arg == "--completion-await-ab"
            || arg == "--claim-progress-ab"
            || arg == "--ready-owner-ab"
            || arg == "--client-turn-ab"
            || arg == "--arrival-clock-ab"
            || arg == "--completion-service-ab"
            || arg == "--activation-protocol-ab"
            || arg == "--lease-plan-protocol-ab"
    })
}

pub(super) fn activation_plan_ab() -> bool {
    std::env::args().any(|arg| {
        arg == "--activation-plan-ab"
            || arg == "--cycle-accounting-ab"
            || arg == "--actor-turn-ab"
            || arg == "--specialized-plan-ab"
            || arg == "--reclamation-plan-ab"
            || arg == "--receiver-plan-ab"
            || arg == "--registered-plan-ab"
            || arg == "--directed-plan-ab"
            || arg == "--execution-checkpoint-ab"
            || arg == "--service-cycle-plan-ab"
            || arg == "--progress-cycle-ab"
            || arg == "--runtime-joint-ab"
            || arg == "--runtime-cost-ab"
            || arg == "--runtime-event-cost-ab"
            || arg == "--native-budget-cost-ab"
            || arg == "--client-arrival-cost-ab"
            || arg == "--retire-publication-ab"
            || arg == "--completion-await-ab"
            || arg == "--claim-progress-ab"
            || arg == "--ready-owner-ab"
            || arg == "--client-turn-ab"
            || arg == "--arrival-clock-ab"
            || arg == "--completion-service-ab"
            || arg == "--activation-protocol-ab"
    })
}
pub(super) fn work_enabled(arm: bool) -> bool {
    arm || std::env::args().any(|arg| {
        arg == "--activation-plan-ab"
            || arg == "--cycle-accounting-ab"
            || arg == "--actor-turn-ab"
            || arg == "--specialized-plan-ab"
            || arg == "--reclamation-plan-ab"
            || arg == "--receiver-plan-ab"
            || arg == "--registered-plan-ab"
            || arg == "--directed-plan-ab"
            || arg == "--execution-checkpoint-ab"
            || arg == "--service-cycle-plan-ab"
            || arg == "--progress-cycle-ab"
            || arg == "--runtime-joint-ab"
            || arg == "--runtime-cost-ab"
            || arg == "--runtime-event-cost-ab"
            || arg == "--native-budget-cost-ab"
            || arg == "--client-arrival-cost-ab"
            || arg == "--retire-publication-ab"
            || arg == "--completion-await-ab"
            || arg == "--claim-progress-ab"
            || arg == "--ready-owner-ab"
            || arg == "--client-turn-ab"
            || arg == "--arrival-clock-ab"
            || arg == "--completion-service-ab"
    })
}

pub(super) fn cycle_accounting_ab() -> bool {
    std::env::args().any(|arg| arg == "--cycle-accounting-ab" || arg == "--actor-turn-ab")
}

pub(super) fn actor_turn_ab() -> bool {
    std::env::args().any(|arg| arg == "--actor-turn-ab")
}

pub(super) fn actor_turn_counts(pool: &InstancePool) -> Vec<[u64; 4]> {
    if !actor_turn_ab() {
        return Vec::new();
    }
    (0..4)
        .map(|owner| {
            let v = pool
                .dispatcher()
                .scheme_service_actor_turn_on(owner)
                .unwrap()
                .wait(GATE)
                .unwrap();
            [
                u64::from(v.enabled),
                v.outer_grants,
                v.quantum_yields,
                v.budget_yields,
            ]
        })
        .collect()
}

pub(super) fn specialized_plan_ab() -> bool {
    std::env::args().any(|arg| arg == "--specialized-plan-ab")
}

pub(super) fn reclamation_plan_ab() -> bool {
    std::env::args().any(|arg| arg == "--reclamation-plan-ab")
}

pub(super) fn receiver_plan_ab() -> bool {
    std::env::args().any(|arg| arg == "--receiver-plan-ab")
}

pub(super) fn registered_plan_ab() -> bool {
    std::env::args().any(|arg| arg == "--registered-plan-ab")
}
pub(super) fn execution_counts(pool: &InstancePool, arm: bool) -> Vec<[u64; 6]> {
    if !registered_plan_ab()
        && !directed_plan_ab()
        && !execution_checkpoint_ab()
        && !service_cycle_plan_ab()
        && !progress_cycle_ab()
        && !runtime_joint_ab()
        && !(runtime_cost_ab() || runtime_event_cost_ab() || native_budget_cost_ab())
    {
        return Vec::new();
    }
    (0..4)
        .map(|owner| {
            let p = pool
                .dispatcher()
                .scheme_service_execution_on(owner)
                .unwrap()
                .wait(GATE)
                .unwrap();
            assert_eq!(
                p.enabled,
                arm || directed_plan_ab()
                    || execution_checkpoint_ab()
                    || service_cycle_plan_ab()
                    || progress_cycle_ab()
                    || runtime_joint_ab()
                    || (runtime_cost_ab() || runtime_event_cost_ab() || native_budget_cost_ab())
            );
            assert_eq!(p.pending_execution, 0);
            [
                p.pending_execution,
                p.issued,
                p.runnable,
                p.registered,
                p.first_grants,
                p.finished_without_grant,
            ]
        })
        .collect()
}

pub(super) fn directed_plan_ab() -> bool {
    std::env::args().any(|arg| arg == "--directed-plan-ab")
}

pub(super) fn execution_checkpoint_ab() -> bool {
    std::env::args().any(|arg| arg == "--execution-checkpoint-ab")
}

pub(super) fn service_cycle_plan_ab() -> bool {
    std::env::args().any(|arg| arg == "--service-cycle-plan-ab")
}
pub(super) fn service_cycle_counts(pool: &InstancePool, arm: bool) -> Vec<[u64; 8]> {
    if !(service_cycle_plan_ab()
        || progress_cycle_ab()
        || runtime_joint_ab()
        || runtime_cost_ab()
        || runtime_event_cost_ab()
        || native_budget_cost_ab())
    {
        return Vec::new();
    }
    (0..4)
        .map(|owner| {
            let p = pool
                .dispatcher()
                .scheme_service_cycle_progress_on(owner)
                .unwrap()
                .wait(GATE)
                .unwrap();
            assert!(p.enabled);
            assert_eq!(
                p.ready_first,
                arm || progress_cycle_ab()
                    || runtime_joint_ab()
                    || (runtime_cost_ab() || runtime_event_cost_ab() || native_budget_cost_ab())
            );
            assert_eq!(p.progress_fair, arm && progress_cycle_ab());
            [
                u64::from(p.ready_first),
                p.direct_collections,
                p.scheduler_passes,
                p.short_collected,
                p.bulk_collected,
                p.active_scheduler_passes,
                u64::from(p.progress_fair),
                p.stalled_ready_passes,
            ]
        })
        .collect()
}

pub(super) fn progress_cycle_ab() -> bool {
    std::env::args().any(|arg| arg == "--progress-cycle-ab")
}

pub(super) fn runtime_joint_ab() -> bool {
    std::env::args().any(|arg| arg == "--runtime-joint-ab")
}
pub(super) fn runtime_counts(pool: &InstancePool, arm: bool) -> Vec<[u64; 4]> {
    if !(runtime_joint_ab() || runtime_cost_ab() || runtime_event_cost_ab()) {
        return Vec::new();
    }
    (0..4)
        .map(|owner| {
            let p = pool
                .dispatcher()
                .scheme_runtime_progress_on(owner)
                .unwrap()
                .wait(GATE)
                .unwrap();
            assert_eq!(
                p.enabled,
                arm || !(runtime_cost_ab() || runtime_event_cost_ab() || native_budget_cost_ab())
            );
            assert_eq!(p.fair, arm);
            assert_eq!(
                [
                    p.runnable,
                    p.lease_wait,
                    p.external_wait,
                    p.actor_barrier,
                    p.running
                ],
                [[0, 0]; 5]
            );
            assert_eq!(p.entered, p.retired);
            assert_eq!(p.ready_results, 0);
            [
                p.entered,
                p.retired,
                p.scheduler_passes,
                p.native_wait_decisions,
            ]
        })
        .collect()
}

pub(super) fn runtime_cost_ab() -> bool {
    std::env::args().any(|arg| arg == "--runtime-cost-ab")
}

pub(super) fn runtime_event_cost_ab() -> bool {
    std::env::args().any(|arg| arg == "--runtime-event-cost-ab")
}

pub(super) fn native_budget_cost_ab() -> bool {
    std::env::args().any(|arg| {
        arg == "--native-budget-cost-ab"
            || arg == "--client-arrival-cost-ab"
            || arg == "--retire-publication-ab"
            || arg == "--completion-await-ab"
            || arg == "--claim-progress-ab"
            || arg == "--ready-owner-ab"
            || arg == "--client-turn-ab"
            || arg == "--arrival-clock-ab"
            || arg == "--completion-service-ab"
    })
}
pub(super) fn native_budget_counts(pool: &InstancePool, arm: bool) -> Vec<[u64; 4]> {
    if !native_budget_cost_ab() {
        return Vec::new();
    }
    (0..4)
        .map(|owner| {
            let d = pool.dispatcher();
            let n = d
                .scheme_native_progress_on(owner)
                .unwrap()
                .wait(GATE)
                .unwrap();
            let p = d
                .scheme_runtime_progress_on(owner)
                .unwrap()
                .wait(GATE)
                .unwrap();
            assert_eq!(
                n.enabled,
                arm || client_arrival_cost_ab() || retire_publication_ab()
            );
            assert_eq!(n.actor_sources, 0);
            assert!(!p.enabled && !p.fair);
            assert_eq!(
                [
                    p.runnable,
                    p.lease_wait,
                    p.external_wait,
                    p.actor_barrier,
                    p.running
                ],
                [[0, 0]; 5]
            );
            assert_eq!((p.entered, p.retired, p.ready_results), (0, 0, 0));
            [u64::from(n.enabled), n.actor_sources, p.entered, p.retired]
        })
        .collect()
}

pub(super) fn client_arrival_cost_ab() -> bool {
    std::env::args().any(|arg| arg == "--client-arrival-cost-ab")
}

pub(super) fn retire_publication_ab() -> bool {
    std::env::args().any(|arg| {
        arg == "--retire-publication-ab"
            || arg == "--completion-await-ab"
            || arg == "--claim-progress-ab"
            || arg == "--ready-owner-ab"
            || arg == "--client-turn-ab"
            || arg == "--arrival-clock-ab"
            || arg == "--completion-service-ab"
    })
}
fn configure_publication(pool: &InstancePool, owner: usize, arm: bool) {
    if retire_publication_ab() {
        use gerbil_scheme::instances::SchemeCompletionPublication::{AfterRetirement, Immediate};
        let policy = if arm
            || completion_await_ab()
            || claim_progress_ab()
            || ready_owner_ab()
            || client_turn_ab()
            || arrival_clock_ab()
            || completion_service_ab()
        {
            AfterRetirement
        } else {
            Immediate
        };
        assert_eq!(
            pool.dispatcher()
                .configure_scheme_completion_publication_on(owner, policy)
                .unwrap(),
            policy
        );
    }
}

pub(super) fn completion_await_ab() -> bool {
    std::env::args().any(|arg| arg == "--completion-await-ab")
}

pub(super) fn claim_progress_ab() -> bool {
    std::env::args().any(|arg| arg == "--claim-progress-ab")
}

pub(super) fn ready_owner_ab() -> bool {
    std::env::args().any(|arg| arg == "--ready-owner-ab")
}

pub(super) fn client_turn_ab() -> bool {
    std::env::args().any(|arg| arg == "--client-turn-ab")
}

pub(super) fn arrival_clock_ab() -> bool {
    std::env::args().any(|arg| arg == "--arrival-clock-ab")
}

pub(super) fn completion_service_ab() -> bool {
    std::env::args().any(|arg| arg == "--completion-service-ab")
}
