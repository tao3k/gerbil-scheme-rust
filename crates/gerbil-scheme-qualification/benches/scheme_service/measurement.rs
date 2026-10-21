//! Finite same-owner sample; phase labels do not claim isolated CPU execution.
use super::{GATE, corpus, workload};
use gerbil_scheme::instances::InstancePool;
use serde_json::{Value, json};
use std::{sync::Arc, time::Instant};
pub(super) fn idle(pool: &InstancePool) {
    let start = Instant::now();
    while !pool
        .dispatcher()
        .owner_capacity()
        .iter()
        .all(|c| c.available == c.limit && c.work_available == c.work_limit)
    {
        assert!(start.elapsed() < GATE, "service credits did not return");
        std::thread::yield_now();
    }
}
pub(super) fn run(
    pool: &InstancePool,
    runtime: &tokio::runtime::Runtime,
    inputs: &Arc<Vec<Vec<i64>>>,
    probes: usize,
    asynchronous: bool,
) -> Value {
    idle(pool);
    let released = Instant::now();
    let count = inputs.len();
    let bulk_inputs = Arc::clone(inputs);
    let dispatch = pool.dispatcher();
    let probe_dispatch = pool.dispatcher();
    let (rows, probe) = runtime.block_on(async {
        let bulk = runtime.spawn(workload::bulk(
            dispatch,
            bulk_inputs,
            asynchronous,
            released,
        ));
        let probes = runtime.spawn(workload::probes(
            probe_dispatch,
            probes,
            asynchronous,
            released,
        ));
        (bulk.await.unwrap(), probes.await.unwrap())
    });
    let elapsed = released.elapsed().as_nanos();
    idle(pool);
    let mut clocks = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut service = Vec::new();
    let mut construct = Vec::new();
    let mut pending = Vec::new();
    let mut projection = Vec::new();
    let mut peer = usize::MAX;
    for (index, row) in rows.into_iter().enumerate() {
        let (reply, completed) = row.unwrap();
        assert_eq!(reply.values.len(), 8);
        for (offset, value) in reply.values.iter().enumerate() {
            corpus::verify(&inputs[index * 8 + offset], value);
        }
        peer = peer.min(corpus::peers(&reply.values));
        service.push(completed.duration_since(reply.entered).as_nanos());
        construct.push(reply.constructed.duration_since(reply.entered).as_nanos());
        pending.push(
            reply
                .projection
                .duration_since(reply.constructed)
                .as_nanos(),
        );
        projection.push(completed.duration_since(reply.projection).as_nanos());
        clocks[index / 4 % 4].extend([completed; 8]);
    }
    let end = *clocks.iter().flatten().max().unwrap();
    let first_done = *clocks
        .iter()
        .map(|c| c.iter().max().unwrap())
        .min()
        .unwrap();
    let minimum = clocks
        .iter()
        .map(|c| c.iter().filter(|&&at| at <= first_done).count())
        .min()
        .unwrap();
    let finish_spread = clocks
        .iter()
        .map(|c| {
            c.iter()
                .max()
                .unwrap()
                .duration_since(first_done)
                .as_nanos()
        })
        .max()
        .unwrap();
    let owner_native_ns: Vec<Vec<u128>> = clocks
        .iter()
        .map(|c| {
            c.iter()
                .map(|at| at.duration_since(released).as_nanos())
                .collect()
        })
        .collect();
    json!({"orders":count,"asynchronous":asynchronous,"elapsed_ns":elapsed,"bulk_elapsed_ns":end.duration_since(released).as_nanos(),
        "bulk_owner_native_ns":owner_native_ns,"bulk_service_ns":service,"bulk_construct_ns":construct,
        "bulk_pending_before_projection_ns":pending,"bulk_projection_retirement_ns":projection,
        "probe_actual_ns":probe.actual,"probe_scheduled_ns":probe.scheduled,"probe_admission_ns":probe.admission,"probe_service_ns":probe.service,
        "probe_overlap":probe.completed.iter().filter(|&&at|at<=end).count(),"owner_progress_at_first_done":minimum,
        "owner_finish_spread_ns":finish_spread,"actor_peer_min":peer})
}
fn quantile(samples: &[Value], field: &str, percent: usize) -> u128 {
    let mut data: Vec<_> = samples
        .iter()
        .flat_map(|s| s[field].as_array().unwrap())
        .map(|v| u128::from(v.as_u64().unwrap()))
        .collect();
    corpus::percentile(&mut data, percent)
}
pub(super) fn emit(samples: &[Value], arm: usize, mixed: bool) {
    let mut duration: Vec<_> = samples
        .iter()
        .map(|s| u128::from(s["bulk_elapsed_ns"].as_u64().unwrap()))
        .collect();
    let median = corpus::percentile(&mut duration, 50);
    let seconds = std::time::Duration::from_nanos(u64::try_from(median).unwrap()).as_secs_f64();
    println!(
        "{}",
        json!({"event":"service_result","arm":arm,"mixed":mixed,"samples":samples.len(),
        "bulk_orders_per_second":samples[0]["orders"].as_f64().unwrap()/seconds,
        "bulk_service_p99_ns":quantile(samples,"bulk_service_ns",99),"bulk_pending_p99_ns":quantile(samples,"bulk_pending_before_projection_ns",99),
        "probe_actual_p99_ns":quantile(samples,"probe_actual_ns",99),"probe_scheduled_p99_ns":quantile(samples,"probe_scheduled_ns",99),
        "probe_admission_p99_ns":quantile(samples,"probe_admission_ns",99),
        "owner_progress_min":samples.iter().map(|s|s["owner_progress_at_first_done"].as_u64().unwrap()).min(),
        "owner_finish_spread_max_ns":samples.iter().map(|s|s["owner_finish_spread_ns"].as_u64().unwrap()).max(),
        "actor_peer_min":samples.iter().map(|s|s["actor_peer_min"].as_u64().unwrap()).min(),
        "probe_overlap_min":samples.iter().map(|s|s["probe_overlap"].as_u64().unwrap()).min()})
    );
}
