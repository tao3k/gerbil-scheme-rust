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
    let expected_bulk = inputs.len() + inputs.iter().filter(|row| row[1] == 1).count();
    assert_eq!(
        short,
        if enabled {
            u64::try_from(probes).unwrap()
        } else {
            0
        }
    );
    assert_eq!(
        bulk,
        if enabled {
            u64::try_from(expected_bulk).unwrap()
        } else {
            0
        }
    );
    json!(rows.into_iter().map(|row| {
        assert!(row.maximum_short_bypasses <= 1 && row.maximum_bulk_bypasses <= 1);
        assert_eq!((row.queued_short, row.queued_bulk, row.active), (0, 0, 0));
        json!({"short_units":row.short_units,"bulk_units":row.bulk_units,"suspensions":row.suspensions,
            "maximum_short_bypasses":row.maximum_short_bypasses,"maximum_bulk_bypasses":row.maximum_bulk_bypasses,
            "contended_grants":row.contended_grants})
    }).collect::<Vec<_>>())
}
