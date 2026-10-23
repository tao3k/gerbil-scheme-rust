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
                        .configure_scheme_service_native_entry_on(owner, arm)
                        .unwrap()
                        .wait(GATE)
                        .unwrap(),
                    arm
                );
            }
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
    std::env::args().any(|arg| arg == "--native-entry-ab")
}
