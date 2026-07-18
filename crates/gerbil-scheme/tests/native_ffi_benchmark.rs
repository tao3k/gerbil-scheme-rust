use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

use gerbil_scheme::GerbilRuntime;
use rust_lang_project_harness::{RustScenarioBenchmarkStatus, validate_rust_scenario_benchmark};

const SAMPLE_COUNT: usize = 100;
const ITERATIONS_PER_SAMPLE: usize = 250;

#[test]
fn steady_state_native_ffi_reports_statistical_receipt() {
    let scenario = validate_rust_scenario_benchmark(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/unit/scenarios/native-ffi-steady-state"),
    )
    .expect("validate the steady-state native FFI scenario benchmark contract");
    assert_eq!(scenario.status, RustScenarioBenchmarkStatus::Pass);

    let runtime = GerbilRuntime::initialize().expect("initialize the live Gerbil runtime");
    let total_started = Instant::now();
    let mut sample_ns_per_triplet = [0_u128; SAMPLE_COUNT];

    for sample in &mut sample_ns_per_triplet {
        let started = Instant::now();
        for _ in 0..ITERATIONS_PER_SAMPLE {
            let value = black_box(41_i64);
            let sum = runtime
                .add_i64(value, black_box(1))
                .expect("benchmark Gerbil addition");
            let even = runtime
                .is_even_i64(value)
                .expect("benchmark Gerbil predicate");
            let ordering = runtime
                .compare_i64(value, black_box(42))
                .expect("benchmark Gerbil comparison");
            black_box((sum, even, ordering));
        }
        *sample = started.elapsed().as_nanos() / ITERATIONS_PER_SAMPLE as u128;
    }

    sample_ns_per_triplet.sort_unstable();
    let median_ns = u128::midpoint(sample_ns_per_triplet[49], sample_ns_per_triplet[50]);
    let p95_ns = sample_ns_per_triplet[94];
    let total = total_started.elapsed();
    let median_budget = scenario
        .benchmark
        .observed_timings
        .get("median_scalar_triplet_budget")
        .expect("scenario declares a median triplet budget")
        .as_duration();

    eprintln!(
        "scenario benchmark receipt: id=native-ffi-steady-state samples={SAMPLE_COUNT} iterations_per_sample={ITERATIONS_PER_SAMPLE} median_ns={median_ns} p95_ns={p95_ns} elapsed_ns={}",
        total.as_nanos(),
    );
    assert!(
        median_ns <= median_budget.as_nanos(),
        "median scalar triplet exceeded {median_budget:?}: {median_ns}ns",
    );
    assert!(
        total <= scenario.benchmark.max_total.as_duration(),
        "steady-state scenario exceeded {:?}: {:?}",
        scenario.benchmark.max_total.as_duration(),
        total,
    );
}
