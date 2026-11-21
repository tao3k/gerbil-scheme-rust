//! Exact request percentiles, independent of the untimed-throughput controls.
use std::{fmt::Write, time::Instant};

use super::{GerbilRuntime, Root, SCENARIOS, buffer, convert, texts_for_scenario};

const BATCHES: usize = 20;
#[path = "spikes.rs"]
mod spikes;

pub(super) fn percentile(sorted: &[u64], percent: usize) -> u64 {
    assert!(!sorted.is_empty() && (1..=100).contains(&percent));
    sorted[sorted.len().checked_mul(percent).unwrap().div_ceil(100) - 1]
}

pub(super) fn contracts() {
    let values: Vec<_> = (1..=100).collect();
    assert_eq!(percentile(&values, 50), 50);
    assert_eq!(percentile(&values, 95), 95);
    assert_eq!(percentile(&values, 99), 99);
    assert_eq!(percentile(&[7], 99), 7);
    assert_eq!(percentile(&[1, 2, 3], 95), 3);
    assert_eq!(percentile(&[1, 2, 3], 50), 2);
}

pub(super) fn run() {
    contracts();
    println!(
        "UTF8-LATENCY-MEASUREMENT scope=bridge boundary=time_to_owned_validated_result scheme_owners=1 concurrency=1 workload=closed_loop queue_latency=unmeasured org_parsing=false percentile=exact_nearest_rank timer_correction=none"
    );
    let runtime = GerbilRuntime::initialize_program(gerbil_scheme_qualification::linked_program())
        .expect("one owner-local AOT runtime");
    buffer::contracts(&runtime);
    let scenario = std::env::args().nth(2).filter(|arg| arg != "--bench");
    let scenario = scenario.as_deref().unwrap_or("mixed");
    let scenarios = if scenario == "all" {
        &SCENARIOS[..]
    } else {
        std::slice::from_ref(&scenario)
    };
    for corpus in scenarios {
        let texts = texts_for_scenario(corpus);
        let roots: Vec<_> = texts
            .iter()
            .map(|text| buffer::root_text(&runtime, text))
            .collect();
        for jobs in [1_000, 10_000, 100_000] {
            measure(corpus, &roots, &texts, jobs);
        }
    }
    assert_eq!(runtime.add_i64(40, 2).unwrap(), 42);
    println!(
        "UTF8-LATENCY-COMPLETE corpora={} loads=3 batches_per_load=20 concurrent_latency_admission=false",
        scenarios.len()
    );
}

fn measure(corpus: &str, roots: &[Root], texts: &[String], jobs: usize) {
    let diagnostic = std::env::args().nth(1).as_deref() == Some("--spikes");
    if diagnostic {
        println!(
            "UTF8-SPIKE-METHOD snapshot=owner_local_allocation_free_native gc_fields=official_vmstate diagnostic_only=true reporting_threshold_ns=1000000"
        );
    }
    // Diagnostic storage is allocated before timing. It is not production
    // reserve/RSS. No histogram insertion, sorting or output occurs per call.
    let mut all = Vec::with_capacity(jobs * BATCHES);
    let mut observations = Vec::with_capacity(jobs);
    let mut clocks = Vec::with_capacity(jobs);
    for _ in 0..jobs {
        let start = Instant::now();
        clocks.push(u64::try_from(start.elapsed().as_nanos()).unwrap());
    }
    clocks.sort_unstable();
    println!(
        "UTF8-LATENCY-CLOCK corpus={corpus} jobs={jobs} observations={jobs} p50_ns={} p95_ns={} p99_ns={} max_ns={} diagnostic_capacity_bytes={}",
        percentile(&clocks, 50),
        percentile(&clocks, 95),
        percentile(&clocks, 99),
        clocks[jobs - 1],
        (all.capacity() + observations.capacity() + clocks.capacity()) * 8
    );
    for index in 0..1_000 {
        let slot = index % roots.len();
        assert_eq!(convert(&roots[slot], 5), texts[slot], "warmup current text");
    }
    let mut total_wall = 0.0;
    for batch in 0..BATCHES {
        observations.clear();
        let batch_start = Instant::now();
        for index in 0..jobs {
            let slot = index % roots.len();
            if diagnostic {
                spikes::before();
            }
            let start = Instant::now();
            let text = std::hint::black_box(convert(&roots[slot], 5));
            let elapsed = u64::try_from(start.elapsed().as_nanos()).unwrap();
            if diagnostic {
                spikes::after(corpus, jobs, batch, index, elapsed);
            }
            // Content checks, sample recording and consumer destruction occur
            // after the service clock. Conversion includes full UTF-8 validation.
            assert_eq!(text, texts[slot], "changing input exact content");
            observations.push(elapsed);
        }
        let wall = batch_start.elapsed().as_secs_f64();
        total_wall += wall;
        assert_eq!(observations.len(), jobs);
        observations.sort_unstable();
        println!(
            "UTF8-LATENCY-BATCH corpus={corpus} jobs={jobs} batch={} requests={jobs} errors=0 batch_wall_ms={:.3} observed_batch_rps={:.3} p50_ns={} p95_ns={} p99_ns={} max_ns={}",
            batch + 1,
            wall * 1000.0,
            f64::from(u32::try_from(jobs).unwrap()) / wall,
            percentile(&observations, 50),
            percentile(&observations, 95),
            percentile(&observations, 99),
            observations[jobs - 1]
        );
        all.extend_from_slice(&observations);
    }
    assert_eq!(all.len(), jobs * BATCHES);
    all.sort_unstable();
    println!(
        "UTF8-LATENCY-SUMMARY corpus={corpus} jobs={jobs} batches=20 requests={} errors=0 observed_batch_rps={:.3} p50_ns={} p95_ns={} p99_ns={} max_ns={}",
        all.len(),
        f64::from(u32::try_from(all.len()).unwrap()) / total_wall,
        percentile(&all, 50),
        percentile(&all, 95),
        percentile(&all, 99),
        all[all.len() - 1]
    );
    histogram(corpus, jobs, &all);
}

pub(super) fn histogram(corpus: &str, jobs: usize, sorted: &[u64]) {
    // Lossless run-length encoding preserves every observation's multiplicity
    // for independent percentile recomputation without one log line per request.
    let mut encoded = String::new();
    let mut first = 0;
    while first < sorted.len() {
        let value = sorted[first];
        let count = sorted[first..].partition_point(|sample| *sample == value);
        write!(encoded, "{value}:{count},").unwrap();
        first += count;
    }
    println!("UTF8-LATENCY-HISTOGRAM corpus={corpus} jobs={jobs} ns_counts={encoded}");
}
