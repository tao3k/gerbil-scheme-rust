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
        | Comparison::SupplyAdmission => (true, true),
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
