//! Completion clocks belong to the owner; dequeue order is not response latency.
use super::{GATE, corpus, observed};
use gerbil_scheme::instances::{InstancePool, InstanceTicket, ProfiledBatch};
use serde_json::json;
use std::{sync::mpsc::Sender, time::Instant};

enum Pending {
    Normal(InstanceTicket<Vec<Vec<i64>>>),
    Profiled(InstanceTicket<ProfiledBatch<i64>>),
}

pub(super) struct Sample {
    pub elapsed: u128,
    pub latency: Vec<u128>,
    pub phases: Vec<[u128; 6]>,
    pub gc: [f64; 4],
    pub collections: [f64; 4],
    pub peer_min: Option<usize>,
}

pub(super) fn run(
    pool: &InstancePool,
    inputs: &[Vec<i64>],
    batch: usize,
    profiled: bool,
    progress: &Sender<&'static str>,
) -> Sample {
    let dispatch = pool.dispatcher();
    let mut sample = Sample {
        elapsed: 0,
        latency: Vec::new(),
        phases: Vec::new(),
        gc: [0.0; 4],
        collections: [0.0; 4],
        peer_min: None,
    };
    let started = Instant::now();
    for window in inputs.chunks(128) {
        let released = Instant::now();
        let mut pending = Vec::new();
        for (chunk, items) in window.chunks(batch).enumerate() {
            let owner = chunk * batch / 32 % pool.owners().len();
            let ticket = if profiled {
                Pending::Profiled(
                    dispatch
                        .submit_numeric_batch_profiled_on(owner, items.to_vec(), 1)
                        .unwrap(),
                )
            } else {
                Pending::Normal(
                    dispatch
                        .submit_numeric_batch_on(owner, items.to_vec(), 1)
                        .unwrap(),
                )
            };
            pending.push((chunk * batch, ticket));
        }
        for (offset, ticket) in pending {
            let (values, timeline, completed) = match ticket {
                Pending::Normal(ticket) => {
                    let (values, completed) = ticket.wait_completed(GATE).unwrap();
                    (values, None, completed)
                }
                Pending::Profiled(ticket) => {
                    let (output, completed) = ticket.wait_completed(GATE).unwrap();
                    (output.values, Some(output.timeline), completed)
                }
            };
            let latency = completed.duration_since(released).as_nanos();
            assert_eq!(values.len(), batch);
            for (index, value) in values.iter().enumerate() {
                corpus::verify(&window[offset + index], value);
                sample.latency.push(latency);
            }
            if batch >= 4 {
                let peers = corpus::peers(&values);
                sample.peer_min = Some(sample.peer_min.map_or(peers, |old| old.min(peers)));
            }
            if let Some(t) = timeline {
                let marks = [
                    t.dispatched,
                    t.entered,
                    t.items_constructed,
                    t.constructed,
                    t.executed,
                    t.projected,
                    completed,
                ];
                assert!(marks.windows(2).all(|pair| pair[0] <= pair[1]));
                sample.phases.push(std::array::from_fn(|i| {
                    marks[i + 1].duration_since(marks[i]).as_nanos()
                }));
                let counters = t
                    .diagnostics
                    .map(|value| value.expect("native diagnostics"));
                for (index, pair) in counters.windows(2).enumerate() {
                    assert!(pair[0][5] <= pair[1][5]);
                    sample.gc[index] += pair[1][2] - pair[0][2];
                    sample.collections[index] += pair[1][5] - pair[0][5];
                }
            }
            observed(progress, "validated native order completion");
        }
    }
    sample.elapsed = started.elapsed().as_nanos();
    sample
}

pub(super) fn emit(
    samples: &mut [Sample],
    owners: usize,
    mixed: bool,
    batch: usize,
    profiled: bool,
    smoke: bool,
    jobs: usize,
) {
    let mut elapsed: Vec<_> = samples.iter().map(|s| s.elapsed).collect();
    let mut latency: Vec<_> = samples
        .iter()
        .flat_map(|s| s.latency.iter().copied())
        .collect();
    let median = corpus::percentile(&mut elapsed, 50);
    let seconds =
        std::time::Duration::from_nanos(u64::try_from(median).expect("bounded sample duration"))
            .as_secs_f64();
    let rate = f64::from(u32::try_from(jobs).expect("bounded order count")) / seconds;
    let phase_names = [
        "admission_and_queue",
        "item_construction",
        "envelope_construction",
        "scheme_including_gc",
        "native_projection",
        "root_retirement_and_completion",
    ];
    let phases: Vec<_> = if profiled {
        phase_names.iter().enumerate().map(|(index, name)| {
            let mut values: Vec<_> = samples.iter().flat_map(|s| s.phases.iter().map(|row| row[index])).collect();
            json!({"name": name, "p50_ns": corpus::percentile(&mut values, 50), "p99_ns": corpus::percentile(&mut values, 99)})
        }).collect()
    } else {
        Vec::new()
    };
    let gc: [f64; 4] = std::array::from_fn(|i| samples.iter().map(|s| s.gc[i]).sum());
    let collections: [f64; 4] =
        std::array::from_fn(|i| samples.iter().map(|s| s.collections[i]).sum());
    println!(
        "{}",
        json!({
            "event": "owner_cost_result", "owners": owners, "mixed": mixed,
            "batch": batch, "profiled": profiled, "smoke": smoke, "samples": samples.len(),
            "orders_per_sample": jobs, "median_ns": median,
            "orders_per_second": rate,
            "response_p99_ns": corpus::percentile(&mut latency, 99),
            "peer_min": samples.iter().filter_map(|s| s.peer_min).min(),
            "phases": phases, "gc_phase_order": ["items", "envelope", "execution", "projection"],
            "gc_real_seconds": profiled.then_some(gc), "gc_collections": profiled.then_some(collections)
        })
    );
}
