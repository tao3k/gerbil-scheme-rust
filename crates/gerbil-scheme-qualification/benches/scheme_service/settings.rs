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
        | Comparison::RetirementRefill => (true, true),
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
        Comparison::PoolProtocol | Comparison::SelectiveWake | Comparison::RetirementRefill
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
    if matches!(
        comparison,
        Comparison::SelectiveWake | Comparison::RetirementRefill
    ) {
        use gerbil_scheme::instances::SchemeShortWake;
        pool.dispatcher()
            .scheme_short_pool()
            .unwrap()
            .configure_wake(if arm || comparison == Comparison::RetirementRefill {
                SchemeShortWake::Selective
            } else {
                SchemeShortWake::Broadcast
            })
            .unwrap();
        assert_eq!(
            pool.dispatcher().scheme_short_pool().unwrap().wake_policy(),
            if arm || comparison == Comparison::RetirementRefill {
                SchemeShortWake::Selective
            } else {
                SchemeShortWake::Broadcast
            }
        );
        // Same native boundary in both arms, outside timing. Configuration wakes
        // old-policy sleepers so they can register under the new protocol.
        read(pool);
    }
}
