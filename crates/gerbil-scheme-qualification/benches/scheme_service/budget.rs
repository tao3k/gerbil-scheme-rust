//! Actual native Scheme configuration and counter receipts for one directed pair.
use super::GATE;
use gerbil_scheme::instances::InstancePool;
use serde_json::{Value, json};
pub(super) fn configure(pool: &InstancePool, enabled: bool) {
    let tickets: Vec<_> = (0..4)
        .map(|owner| {
            pool.dispatcher()
                .configure_scheme_service_budget_on(owner, usize::from(enabled))
                .unwrap()
        })
        .collect();
    for ticket in tickets {
        ticket.wait(GATE).unwrap();
    }
}
pub(super) fn snapshot(
    pool: &InstancePool,
    enabled: bool,
    inputs: &[Vec<i64>],
    probes: usize,
) -> Value {
    let tickets: Vec<_> = (0..4)
        .map(|owner| pool.dispatcher().scheme_service_budget_on(owner).unwrap())
        .collect();
    let rows: Vec<_> = tickets
        .into_iter()
        .map(|ticket| ticket.wait(GATE).unwrap())
        .collect();
    let short: u64 = rows.iter().map(|row| row.short_units).sum();
    let bulk: u64 = rows.iter().map(|row| row.bulk_units).sum();
    let waits = inputs.iter().filter(|row| row[1] == 1).count();
    let handoffs = if super::settings::work_budget_ab() {
        (0..4)
            .map(|owner| {
                pool.dispatcher()
                    .scheme_service_work_on(owner)
                    .unwrap()
                    .wait(GATE)
                    .unwrap()
                    .handoffs
            })
            .sum::<u64>()
    } else {
        0
    };
    let (outer_grants, turn_yields) = turn_ownership(pool);
    let entries = outer_grants.unwrap_or_else(|| u64::try_from(inputs.len()).unwrap());
    let expected_bulk = entries + u64::try_from(waits).unwrap() + handoffs + turn_yields;
    if super::settings::work_budget_ab() {
        // Short has one terminal four-record span. Every additional grant and
        // suspension must correspond to an actual bulk checkpoint handoff.
        assert_eq!(
            rows.iter().map(|row| row.suspensions).sum::<u64>(),
            u64::try_from(waits).unwrap() + handoffs + turn_yields
        );
    }
    assert_eq!(
        short,
        if enabled {
            u64::try_from(probes).unwrap()
        } else {
            0
        }
    );
    assert_eq!(bulk, if enabled { expected_bulk } else { 0 });
    json!(rows.into_iter().map(|row| {
        assert!(row.maximum_short_bypasses <= 1 && row.maximum_bulk_bypasses <= 1);
        assert_eq!((row.queued_short, row.queued_bulk, row.active), (0, 0, 0));
        json!({"short_units":row.short_units,"bulk_units":row.bulk_units,"suspensions":row.suspensions,
            "maximum_short_bypasses":row.maximum_short_bypasses,"maximum_bulk_bypasses":row.maximum_bulk_bypasses,
            "contended_grants":row.contended_grants})
    }).collect::<Vec<_>>())
}

pub(super) fn wake(pool: &InstancePool, direct: bool) {
    use gerbil_scheme::instances::SchemeServiceWake;
    let mode = if direct {
        SchemeServiceWake::NativePort
    } else {
        SchemeServiceWake::Watcher
    };
    let tickets: Vec<_> = (0..4)
        .map(|owner| {
            pool.dispatcher()
                .configure_scheme_service_wake_on(owner, mode)
                .unwrap()
        })
        .collect();
    for ticket in tickets {
        ticket.wait(GATE).unwrap();
    }
}

pub(super) fn notification(pool: &InstancePool, independent: bool) {
    use gerbil_scheme::instances::SchemeServiceNotification;
    let policy = if independent {
        SchemeServiceNotification::ShortIndependent
    } else {
        SchemeServiceNotification::Coalesced
    };
    for owner in 0..4 {
        pool.dispatcher()
            .configure_scheme_service_notification_on(owner, policy)
            .unwrap();
    }
}

pub(super) fn verify_notifications(value: &Value, probes: usize) {
    let entries = value["probe_entry"].as_array().unwrap();
    assert_eq!(entries.len(), probes);
    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry["wake"].as_str(), Some("Written" | "Full")))
    );
}

fn turn_ownership(pool: &InstancePool) -> (Option<u64>, u64) {
    if !super::settings::actor_turn_ab() {
        return (None, 0);
    }
    let turns: Vec<_> = (0..4)
        .map(|owner| {
            pool.dispatcher()
                .scheme_service_actor_turn_on(owner)
                .unwrap()
                .wait(GATE)
                .unwrap()
        })
        .collect();
    assert!(turns.iter().all(|v| v.enabled == turns[0].enabled));
    if turns[0].enabled {
        (
            Some(turns.iter().map(|v| v.outer_grants).sum()),
            turns
                .iter()
                .map(|v| v.quantum_yields + v.budget_yields)
                .sum(),
        )
    } else {
        (None, 0)
    }
}
