//! Matched bursts: native completion timestamps and distinct owner/Actor fairness.
use super::{corpus, executor::Executor, observed};
use gerbil_scheme::instances::InstancePool;
use serde_json::json;
use std::{sync::mpsc::Sender, time::Instant};
pub(super) struct Sample {
    pub elapsed: u128,
    pub latency: Vec<u128>,
    pub delivery: Vec<u128>,
    pub after_native: Vec<u128>,
    pub owner_latency: Vec<Vec<u128>>,
    pub actor_peer_min: Option<usize>,
    pub owner_progress_min: usize,
}
pub(super) fn run(
    pool: &InstancePool,
    executor: &Executor,
    inputs: &[Vec<i64>],
    batch: usize,
    progress: &Sender<&'static str>,
) -> Sample {
    let owners = pool.owners().len();
    let mut sample = Sample {
        elapsed: 0,
        latency: Vec::new(),
        delivery: Vec::new(),
        after_native: Vec::new(),
        owner_latency: vec![Vec::new(); owners],
        actor_peer_min: None,
        owner_progress_min: 128 / owners,
    };
    let started = Instant::now();
    for window in inputs.chunks(128) {
        let released = Instant::now();
        let jobs = window
            .chunks(batch)
            .enumerate()
            .map(|(chunk, items)| (chunk * batch / 32 % owners, items.to_vec()))
            .collect();
        let output = executor.window(&pool.dispatcher(), jobs, released);
        let delivered = released.elapsed().as_nanos();
        assert_eq!(output.len(), window.len() / batch);
        let mut clocks: Vec<Vec<Instant>> = vec![Vec::new(); owners];
        for (chunk, result) in output.into_iter().enumerate() {
            let (values, completed) = result.unwrap();
            let offset = chunk * batch;
            let owner = offset / 32 % owners;
            let latency = completed.duration_since(released).as_nanos();
            assert_eq!(values.len(), batch);
            for (index, row) in values.iter().enumerate() {
                corpus::verify(&window[offset + index], row);
                sample.latency.push(latency);
                sample.owner_latency[owner].push(latency);
                clocks[owner].push(completed);
            }
            if batch >= 4 {
                let peers = corpus::peers(&values);
                sample.actor_peer_min =
                    Some(sample.actor_peer_min.map_or(peers, |old| old.min(peers)));
            }
            observed(progress, "validated dispatcher native completion");
        }
        // Equal per-owner offered work. Measure the least owner progress at the
        // earliest instant any owner finishes its complete share of this burst.
        let first_done = clocks
            .iter()
            .map(|times| *times.iter().max().unwrap())
            .min()
            .unwrap();
        let minimum = clocks
            .iter()
            .map(|times| times.iter().filter(|&&t| t <= first_done).count())
            .min()
            .unwrap();
        sample.owner_progress_min = sample.owner_progress_min.min(minimum);
        let last_native = clocks
            .iter()
            .flatten()
            .max()
            .unwrap()
            .duration_since(released)
            .as_nanos();
        assert!(last_native <= delivered);
        sample.after_native.push(delivered - last_native);
        sample.delivery.push(delivered);
    }
    sample.elapsed = started.elapsed().as_nanos();
    sample
}
pub(super) fn emit(
    samples: &[Sample],
    owners: usize,
    (mode, workers, diagnostic, grouped): (&str, usize, bool, bool),
    mixed: bool,
    batch: usize,
    smoke: bool,
    jobs: usize,
) {
    let mut elapsed: Vec<_> = samples.iter().map(|s| s.elapsed).collect();
    let mut latency: Vec<_> = samples
        .iter()
        .flat_map(|s| s.latency.iter().copied())
        .collect();
    let mut delivery: Vec<_> = samples
        .iter()
        .flat_map(|s| s.delivery.iter().copied())
        .collect();
    let mut recovery: Vec<_> = samples
        .iter()
        .flat_map(|s| s.after_native.iter().copied())
        .collect();
    let owner_p99: Vec<_> = (0..owners)
        .map(|owner| {
            let mut values: Vec<_> = samples
                .iter()
                .flat_map(|s| s.owner_latency[owner].iter().copied())
                .collect();
            corpus::percentile(&mut values, 99)
        })
        .collect();
    let median = corpus::percentile(&mut elapsed, 50);
    let seconds = std::time::Duration::from_nanos(u64::try_from(median).unwrap()).as_secs_f64();
    let rate = f64::from(u32::try_from(jobs).unwrap()) / seconds;
    println!(
        "{}",
        json!({
            "event": "dispatch_result", "diagnostic": diagnostic, "grouped": grouped, "owners": owners, "mode": mode, "workers": workers,
            "mixed": mixed, "batch": batch, "smoke": smoke, "samples": samples.len(),
            "orders_per_sample": jobs, "median_ns": median, "orders_per_second": rate,
            "native_response_p99_ns": corpus::percentile(&mut latency, 99),
            "window_delivery_p99_ns": corpus::percentile(&mut delivery, 99),
            "window_recovery_p99_ns": corpus::percentile(&mut recovery, 99), "owner_p99_ns": owner_p99,
            "actor_peer_min": samples.iter().filter_map(|s| s.actor_peer_min).min(),
            "owner_progress_min_orders_at_first_done": samples.iter().map(|s| s.owner_progress_min).min().unwrap(),
            "owner_offered_orders_per_window": 128 / owners
        })
    );
}
