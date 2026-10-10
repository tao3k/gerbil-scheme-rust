// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use gerbil_scheme::NativeError;
use gerbil_scheme_sys::GerbilStatus;

#[test]
fn invalid_comparison_result_fails_closed_as_an_invalid_value() {
    let scenario = rust_lang_project_harness::validate_rust_scenario_benchmark(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/unit/scenarios/invalid-comparison-fail-closed"),
    )
    .expect("validate the invalid comparison scenario benchmark contract");
    assert_eq!(
        scenario.status,
        rust_lang_project_harness::RustScenarioBenchmarkStatus::Pass,
    );

    let started = std::time::Instant::now();
    for _ in 0..10_000 {
        let error = NativeError::InvalidComparisonResult { code: 7 };
        assert_eq!(error.status(), Some(GerbilStatus::InvalidValue));
        assert_eq!(error.to_string(), "invalid Gerbil i64 comparison result 7");
    }
    let elapsed = started.elapsed();
    eprintln!(
        "scenario benchmark receipt: id=invalid-comparison-fail-closed projections=10000 elapsed_ns={}",
        elapsed.as_nanos(),
    );
    assert!(
        elapsed <= scenario.benchmark.max_total.as_duration(),
        "invalid comparison scenario exceeded {:?}: {:?}",
        scenario.benchmark.max_total.as_duration(),
        elapsed,
    );
}
