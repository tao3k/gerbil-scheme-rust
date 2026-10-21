//! Scheduled probe tails and finite cross-producer progress use native clocks.
use super::{GATE, corpus, workload};
use gerbil_scheme::instances::InstancePool;
use serde_json::{Value, json};
use std::{sync::Arc, time::Instant};
#[derive(Clone, Copy)]
pub(super) struct Case {
    pub capacity: usize,
    pub batch: usize,
}
pub(super) fn idle(pool: &InstancePool) {
    let started = Instant::now();
    loop {
        let snapshots = pool.dispatcher().owner_capacity();
        if snapshots.len() == 4 && snapshots.iter().all(|s| s.available == s.limit) {
            return;
        }
        assert!(started.elapsed() < GATE, "pool credits did not return");
        std::thread::yield_now();
    }
}
pub(super) fn run(
    pool: &InstancePool,
    runtime: &tokio::runtime::Runtime,
    inputs: &Arc<Vec<Vec<i64>>>,
    probes: usize,
    case: Case,
) -> Value {
    idle(pool);
    pool.dispatcher()
        .configure_owner_capacity(case.capacity)
        .unwrap();
    let released = Instant::now();
    let bulk_inputs = Arc::clone(inputs);
    let dispatch = pool.dispatcher();
    let probe_dispatch = pool.dispatcher();
    let (bulk, probe) = runtime.block_on(async {
        // One task per producer, never one per native request. Scheme stays on owners.
        let bulk = runtime.spawn(workload::bulk(dispatch, bulk_inputs, case.batch, released));
        let probe = runtime.spawn(workload::probes(probe_dispatch, probes, released));
        let bulk = bulk.await.unwrap();
        let probe = probe.await.unwrap().unwrap();
        (bulk, probe)
    });
    let elapsed = released.elapsed().as_nanos();
    idle(pool);
    let last_bulk = *bulk.clocks.iter().flatten().max().unwrap();
    let last_probe = *probe.clocks.iter().max().unwrap();
    let bulk_before_last_probe = bulk
        .clocks
        .iter()
        .flatten()
        .filter(|&&clock| clock <= last_probe)
        .count();
    let probe_before_bulk_end = probe
        .clocks
        .iter()
        .filter(|&&clock| clock <= last_bulk)
        .count();
    let first_owner_done = bulk
        .clocks
        .iter()
        .map(|clocks| *clocks.last().unwrap())
        .min()
        .unwrap();
    let owner_progress = bulk
        .clocks
        .iter()
        .map(|clocks| {
            clocks
                .iter()
                .filter(|&&clock| clock <= first_owner_done)
                .count()
        })
        .min()
        .unwrap();
    let bulk_owner_ns: Vec<Vec<u128>> = bulk
        .clocks
        .iter()
        .map(|clocks| {
            clocks
                .iter()
                .map(|clock| clock.duration_since(released).as_nanos())
                .collect()
        })
        .collect();
    json!({"elapsed_ns": elapsed, "bulk_elapsed_ns": bulk.elapsed,
        "bulk_orders": inputs.len(), "probe_orders": probes,
        "bulk_owner_native_ns": bulk_owner_ns, "probe_native_ns": probe.native,
        "probe_delivery_ns": probe.delivery, "probe_admission_ns": probe.admission,
        "probe_wake_ns": probe.wake, "owner_progress_at_first_done": owner_progress,
        "bulk_completed_at_last_probe": bulk_before_last_probe, "probes_completed_during_bulk": probe_before_bulk_end,
        "actor_peer_min": bulk.actor_peer_min, "capacity": case.capacity, "batch": case.batch})
}
fn series(samples: &[Value], field: &str) -> Vec<u128> {
    samples
        .iter()
        .flat_map(|sample| {
            sample[field]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| u128::from(v.as_u64().unwrap()))
        })
        .collect()
}
fn quantile(samples: &[Value], field: &str) -> u128 {
    corpus::percentile(&mut series(samples, field), 99)
}
pub(super) fn emit(samples: &[Value], pair: usize, arm: usize, mixed: bool) {
    let mut duration: Vec<_> = samples
        .iter()
        .map(|sample| u128::from(sample["elapsed_ns"].as_u64().unwrap()))
        .collect();
    let mut bulk_duration: Vec<_> = samples
        .iter()
        .map(|sample| u128::from(sample["bulk_elapsed_ns"].as_u64().unwrap()))
        .collect();
    let elapsed = corpus::percentile(&mut duration, 50);
    let bulk_elapsed = corpus::percentile(&mut bulk_duration, 50);
    let orders = samples[0]["bulk_orders"].as_u64().unwrap();
    let probes = samples[0]["probe_orders"].as_u64().unwrap();
    let seconds = |ns| std::time::Duration::from_nanos(u64::try_from(ns).unwrap()).as_secs_f64();
    println!(
        "{}",
        json!({"event": "capacity_result", "pair": pair, "arm": arm, "mixed": mixed,
        "samples": samples.len(), "capacity": samples[0]["capacity"], "batch": samples[0]["batch"],
        "bulk_orders": orders, "probe_orders": probes, "median_ns": elapsed,
        "median_bulk_ns": bulk_elapsed, "bulk_orders_per_second": u32::try_from(orders).map(f64::from).unwrap() / seconds(bulk_elapsed),
        "complete_orders_per_second": u32::try_from(orders + probes).map(f64::from).unwrap() / seconds(elapsed),
        "probe_native_p99_ns": quantile(samples, "probe_native_ns"), "probe_delivery_p99_ns": quantile(samples, "probe_delivery_ns"),
        "probe_admission_p99_ns": quantile(samples, "probe_admission_ns"), "probe_wake_p99_ns": quantile(samples, "probe_wake_ns"),
        "owner_progress_min": samples.iter().map(|s| s["owner_progress_at_first_done"].as_u64().unwrap()).min(),
        "probe_overlap_min": samples.iter().map(|s| s["probes_completed_during_bulk"].as_u64().unwrap()).min(),
        "bulk_at_last_probe_min": samples.iter().map(|s| s["bulk_completed_at_last_probe"].as_u64().unwrap()).min(),
        "actor_peer_min": samples.iter().filter_map(|s| s["actor_peer_min"].as_u64()).min()})
    );
}
