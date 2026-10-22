//! Optional diagnostic controls; phase deltas do not identify OS time alone.
use super::GATE;
use gerbil_scheme::instances::{InstancePool, SchemeServiceObservation};
use serde_json::{Value, json};
pub(super) fn snapshot(pool: &InstancePool) -> Vec<SchemeServiceObservation> {
    let tickets: Vec<_> = (0..4)
        .map(|owner| {
            pool.dispatcher()
                .scheme_service_observation_on(owner)
                .unwrap()
        })
        .collect();
    tickets
        .into_iter()
        .map(|ticket| ticket.wait(GATE).unwrap())
        .collect()
}
pub(super) fn delta(
    before: &[SchemeServiceObservation],
    after: &[SchemeServiceObservation],
) -> Value {
    json!(
        before
            .iter()
            .zip(after)
            .map(|(a, b)| {
                a.phases
                    .iter()
                    .zip(b.phases)
                    .map(|(start, end)| {
                        json!({
                            "calls":end.calls-start.calls, "wall_ns":end.wall_ns-start.wall_ns,
                            "cpu_ns":end.cpu_ns.zip(start.cpu_ns).map(|(x,y)|x-y),
                            "gc_real_ns":end.gc_real_ns.zip(start.gc_real_ns).map(|(x,y)|x-y),
                            "gc_cpu_ns":end.gc_cpu_ns.zip(start.gc_cpu_ns).map(|(x,y)|x-y),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    )
}

pub(super) fn append(
    value: &mut Value,
    pool: &InstancePool,
    before: Option<Vec<SchemeServiceObservation>>,
) {
    if let Some(before) = before {
        value["owner_phases"] = delta(&before, &snapshot(pool));
    }
}
