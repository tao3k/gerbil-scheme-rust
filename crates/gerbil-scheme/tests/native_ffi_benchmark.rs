use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

use gerbil_scheme::GerbilRuntime;
use rust_lang_project_harness::{RustScenarioBenchmarkStatus, validate_rust_scenario_benchmark};

const SAMPLE_COUNT: usize = 100;
const ITERATIONS_PER_SAMPLE: usize = 250;
type Samples = [u128; SAMPLE_COUNT];

fn sample_operation(mut operation: impl FnMut()) -> Samples {
    let mut samples = [0_u128; SAMPLE_COUNT];
    for sample in &mut samples {
        let started = Instant::now();
        for _ in 0..ITERATIONS_PER_SAMPLE {
            operation();
        }
        *sample = started.elapsed().as_nanos() / ITERATIONS_PER_SAMPLE as u128;
    }
    samples
}

fn summarize(samples: &mut Samples) -> (u128, u128) {
    samples.sort_unstable();
    (u128::midpoint(samples[49], samples[50]), samples[94])
}

fn sample_raw_aot() -> (Samples, Samples, Samples) {
    let add = sample_operation(|| {
        black_box(unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_add_i64(black_box(41), black_box(1))
        });
    });
    let even = sample_operation(|| {
        black_box(unsafe { gerbil_scheme_sys::gerbil_scheme_rust_is_even_i64(black_box(41)) });
    });
    let compare = sample_operation(|| {
        black_box(unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_compare_i64(black_box(41), black_box(42))
        });
    });
    (add, even, compare)
}

#[test]
fn steady_state_native_ffi_reports_statistical_receipt() {
    let scenario = validate_rust_scenario_benchmark(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/unit/scenarios/native-ffi-steady-state"),
    )
    .expect("validate the steady-state native FFI scenario benchmark contract");
    assert_eq!(scenario.status, RustScenarioBenchmarkStatus::Pass);

    let runtime = GerbilRuntime::initialize().expect("initialize the live Gerbil runtime");
    let total_started = Instant::now();
    let mut c_identity_ns = sample_operation(|| {
        let value = unsafe { gerbil_scheme_sys::gerbil_scheme_rust_identity_i64(black_box(41)) };
        black_box(value);
    });
    let (mut raw_add_ns, mut raw_even_ns, mut raw_compare_ns) = sample_raw_aot();
    let mut add_ns = sample_operation(|| {
        let sum = runtime
            .add_i64(black_box(41), black_box(1))
            .expect("benchmark Gerbil addition");
        black_box(sum);
    });
    let mut even_ns = sample_operation(|| {
        let even = runtime
            .is_even_i64(black_box(41))
            .expect("benchmark Gerbil predicate");
        black_box(even);
    });
    let mut compare_ns = sample_operation(|| {
        let ordering = runtime
            .compare_i64(black_box(41), black_box(42))
            .expect("benchmark Gerbil comparison");
        black_box(ordering);
    });

    let (c_identity_median_ns, c_identity_p95_ns) = summarize(&mut c_identity_ns);
    let (raw_add_median_ns, raw_add_p95_ns) = summarize(&mut raw_add_ns);
    let (raw_even_median_ns, raw_even_p95_ns) = summarize(&mut raw_even_ns);
    let (raw_compare_median_ns, raw_compare_p95_ns) = summarize(&mut raw_compare_ns);
    let (add_median_ns, add_p95_ns) = summarize(&mut add_ns);
    let (even_median_ns, even_p95_ns) = summarize(&mut even_ns);
    let (compare_median_ns, compare_p95_ns) = summarize(&mut compare_ns);
    let add_vs_c_x100 = add_median_ns * 100 / c_identity_median_ns;
    let even_vs_c_x100 = even_median_ns * 100 / c_identity_median_ns;
    let compare_vs_c_x100 = compare_median_ns * 100 / c_identity_median_ns;
    let add_safe_vs_raw_x100 = add_median_ns * 100 / raw_add_median_ns;
    let even_safe_vs_raw_x100 = even_median_ns * 100 / raw_even_median_ns;
    let compare_safe_vs_raw_x100 = compare_median_ns * 100 / raw_compare_median_ns;
    let total = total_started.elapsed();
    let budget = |name: &str| {
        scenario
            .benchmark
            .observed_timings
            .get(name)
            .unwrap_or_else(|| panic!("scenario declares {name}"))
            .as_duration()
    };

    eprintln!(
        "scenario benchmark receipt: id=native-ffi-steady-state samples={SAMPLE_COUNT} iterations_per_sample={ITERATIONS_PER_SAMPLE} c_identity_median_ns={c_identity_median_ns} c_identity_p95_ns={c_identity_p95_ns} raw_add_median_ns={raw_add_median_ns} raw_add_p95_ns={raw_add_p95_ns} add_median_ns={add_median_ns} add_p95_ns={add_p95_ns} add_vs_c_x100={add_vs_c_x100} add_safe_vs_raw_x100={add_safe_vs_raw_x100} raw_even_median_ns={raw_even_median_ns} raw_even_p95_ns={raw_even_p95_ns} even_median_ns={even_median_ns} even_p95_ns={even_p95_ns} even_vs_c_x100={even_vs_c_x100} even_safe_vs_raw_x100={even_safe_vs_raw_x100} raw_compare_median_ns={raw_compare_median_ns} raw_compare_p95_ns={raw_compare_p95_ns} compare_median_ns={compare_median_ns} compare_p95_ns={compare_p95_ns} compare_vs_c_x100={compare_vs_c_x100} compare_safe_vs_raw_x100={compare_safe_vs_raw_x100} elapsed_ns={}",
        total.as_nanos(),
    );
    for (name, observed_ns, budget_name) in [
        (
            "C identity median",
            c_identity_median_ns,
            "median_c_identity_i64_budget",
        ),
        (
            "C identity p95",
            c_identity_p95_ns,
            "p95_c_identity_i64_budget",
        ),
        ("add median", add_median_ns, "median_add_i64_budget"),
        ("add p95", add_p95_ns, "p95_add_i64_budget"),
        ("even median", even_median_ns, "median_is_even_i64_budget"),
        ("even p95", even_p95_ns, "p95_is_even_i64_budget"),
        (
            "compare median",
            compare_median_ns,
            "median_compare_i64_budget",
        ),
        ("compare p95", compare_p95_ns, "p95_compare_i64_budget"),
    ] {
        let operation_budget = budget(budget_name);
        assert!(
            observed_ns <= operation_budget.as_nanos(),
            "{name} exceeded {operation_budget:?}: {observed_ns}ns",
        );
    }
    assert!(
        total <= scenario.benchmark.max_total.as_duration(),
        "steady-state scenario exceeded {:?}: {:?}",
        scenario.benchmark.max_total.as_duration(),
        total,
    );
}
