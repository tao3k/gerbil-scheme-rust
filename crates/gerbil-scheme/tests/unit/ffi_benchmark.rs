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
        *sample = started.elapsed().as_nanos();
    }
    samples
}

fn summarize(samples: &mut Samples) -> (u128, u128) {
    samples.sort_unstable();
    (u128::midpoint(samples[49], samples[50]), samples[94])
}

fn ratio_x100(numerator: u128, denominator: u128) -> Option<u128> {
    numerator.checked_mul(100)?.checked_div(denominator)
}

#[test]
fn ratios_reject_unresolved_clock_samples_and_overflow() {
    assert_eq!(ratio_x100(15, 10), Some(150));
    // Valid subnanosecond averages retain a resolvable full-batch ratio.
    assert_eq!(ratio_x100(250, 125), Some(200));
    assert_eq!(ratio_x100(0, 10), Some(0));
    assert_eq!(ratio_x100(15, 0), None);
    assert_eq!(ratio_x100(u128::MAX, 1), None);
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

fn sample_safe(runtime: &GerbilRuntime) -> (Samples, Samples, Samples, Samples) {
    let identity_batch_ns = sample_operation(|| {
        let value = runtime
            .identity_i64(black_box(41))
            .expect("benchmark Gerbil identity");
        black_box(value);
    });
    let add_batch_ns = sample_operation(|| {
        let sum = runtime
            .add_i64(black_box(41), black_box(1))
            .expect("benchmark Gerbil addition");
        black_box(sum);
    });
    let even_batch_ns = sample_operation(|| {
        let even = runtime
            .is_even_i64(black_box(41))
            .expect("benchmark Gerbil predicate");
        black_box(even);
    });
    let compare_batch_ns = sample_operation(|| {
        let ordering = runtime
            .compare_i64(black_box(41), black_box(42))
            .expect("benchmark Gerbil comparison");
        black_box(ordering);
    });

    (
        identity_batch_ns,
        add_batch_ns,
        even_batch_ns,
        compare_batch_ns,
    )
}

#[test]
fn steady_state_native_ffi_reports_statistical_receipt() {
    let scenario = validate_rust_scenario_benchmark(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/unit/scenarios/ffi-steady-state"),
    )
    .expect("validate the steady-state native FFI scenario benchmark contract");
    assert_eq!(scenario.status, RustScenarioBenchmarkStatus::Pass);

    let runtime = GerbilRuntime::initialize().expect("initialize the live Gerbil runtime");
    let total_started = Instant::now();
    let mut c_identity_batch_ns = sample_operation(|| {
        let value = unsafe { gerbil_scheme_sys::gerbil_scheme_rust_identity_i64(black_box(41)) };
        black_box(value);
    });
    let (mut raw_add_batch_ns, mut raw_even_batch_ns, mut raw_compare_batch_ns) = sample_raw_aot();
    let (mut identity_batch_ns, mut add_batch_ns, mut even_batch_ns, mut compare_batch_ns) =
        sample_safe(&runtime);

    let (c_identity_median_batch_ns, c_identity_p95_batch_ns) = summarize(&mut c_identity_batch_ns);
    let (raw_add_median_batch_ns, raw_add_p95_batch_ns) = summarize(&mut raw_add_batch_ns);
    let (raw_even_median_batch_ns, raw_even_p95_batch_ns) = summarize(&mut raw_even_batch_ns);
    let (raw_compare_median_batch_ns, raw_compare_p95_batch_ns) =
        summarize(&mut raw_compare_batch_ns);
    let (identity_median_batch_ns, identity_p95_batch_ns) = summarize(&mut identity_batch_ns);
    let (add_median_batch_ns, add_p95_batch_ns) = summarize(&mut add_batch_ns);
    let (even_median_batch_ns, even_p95_batch_ns) = summarize(&mut even_batch_ns);
    let (compare_median_batch_ns, compare_p95_batch_ns) = summarize(&mut compare_batch_ns);
    // Preserve full batch durations for ratios: dividing first truncates valid
    // subnanosecond averages to zero. A genuinely unresolved batch still fails.
    let ratio = |numerator, denominator| {
        ratio_x100(numerator, denominator)
            .expect("FFI benchmark ratio is unresolved: zero control timing or arithmetic overflow")
    };
    let identity_vs_c_x100 = ratio(identity_median_batch_ns, c_identity_median_batch_ns);
    let add_vs_c_x100 = ratio(add_median_batch_ns, c_identity_median_batch_ns);
    let even_vs_c_x100 = ratio(even_median_batch_ns, c_identity_median_batch_ns);
    let compare_vs_c_x100 = ratio(compare_median_batch_ns, c_identity_median_batch_ns);
    let add_safe_vs_raw_x100 = ratio(add_median_batch_ns, raw_add_median_batch_ns);
    let even_safe_vs_raw_x100 = ratio(even_median_batch_ns, raw_even_median_batch_ns);
    let compare_safe_vs_raw_x100 = ratio(compare_median_batch_ns, raw_compare_median_batch_ns);
    let total = total_started.elapsed();
    eprintln!(
        "scenario benchmark receipt: id=ffi-steady-state samples={SAMPLE_COUNT} iterations_per_sample={ITERATIONS_PER_SAMPLE} c_identity_median_batch_ns={c_identity_median_batch_ns} c_identity_p95_batch_ns={c_identity_p95_batch_ns} identity_median_batch_ns={identity_median_batch_ns} identity_p95_batch_ns={identity_p95_batch_ns} identity_vs_c_x100={identity_vs_c_x100} raw_add_median_batch_ns={raw_add_median_batch_ns} raw_add_p95_batch_ns={raw_add_p95_batch_ns} add_median_batch_ns={add_median_batch_ns} add_p95_batch_ns={add_p95_batch_ns} add_vs_c_x100={add_vs_c_x100} add_safe_vs_raw_x100={add_safe_vs_raw_x100} raw_even_median_batch_ns={raw_even_median_batch_ns} raw_even_p95_batch_ns={raw_even_p95_batch_ns} even_median_batch_ns={even_median_batch_ns} even_p95_batch_ns={even_p95_batch_ns} even_vs_c_x100={even_vs_c_x100} even_safe_vs_raw_x100={even_safe_vs_raw_x100} raw_compare_median_batch_ns={raw_compare_median_batch_ns} raw_compare_p95_batch_ns={raw_compare_p95_batch_ns} compare_median_batch_ns={compare_median_batch_ns} compare_p95_batch_ns={compare_p95_batch_ns} compare_vs_c_x100={compare_vs_c_x100} compare_safe_vs_raw_x100={compare_safe_vs_raw_x100} elapsed_ns={}",
        total.as_nanos(),
    );
    assert_operation_budgets(
        &scenario,
        &[
            (
                "C identity median",
                c_identity_median_batch_ns,
                "median_c_identity_i64_budget",
            ),
            (
                "C identity p95",
                c_identity_p95_batch_ns,
                "p95_c_identity_i64_budget",
            ),
            (
                "identity median",
                identity_median_batch_ns,
                "median_identity_i64_budget",
            ),
            (
                "identity p95",
                identity_p95_batch_ns,
                "p95_identity_i64_budget",
            ),
            ("add median", add_median_batch_ns, "median_add_i64_budget"),
            ("add p95", add_p95_batch_ns, "p95_add_i64_budget"),
            (
                "even median",
                even_median_batch_ns,
                "median_is_even_i64_budget",
            ),
            ("even p95", even_p95_batch_ns, "p95_is_even_i64_budget"),
            (
                "compare median",
                compare_median_batch_ns,
                "median_compare_i64_budget",
            ),
            (
                "compare p95",
                compare_p95_batch_ns,
                "p95_compare_i64_budget",
            ),
        ],
    );
    assert!(
        total <= scenario.benchmark.max_total.as_duration(),
        "steady-state scenario exceeded {:?}: {:?}",
        scenario.benchmark.max_total.as_duration(),
        total,
    );
}

fn assert_operation_budgets(
    scenario: &rust_lang_project_harness::RustScenarioBenchmarkReceipt,
    observations: &[(&str, u128, &str)],
) {
    for (name, observed_batch_ns, budget_name) in observations {
        let operation_budget = scenario
            .benchmark
            .observed_timings
            .get(*budget_name)
            .unwrap_or_else(|| panic!("scenario declares {budget_name}"))
            .as_duration();
        assert!(
            *observed_batch_ns <= operation_budget.as_nanos() * ITERATIONS_PER_SAMPLE as u128,
            "{name} exceeded {operation_budget:?}: {observed_batch_ns}ns per {ITERATIONS_PER_SAMPLE} calls",
        );
    }
}
