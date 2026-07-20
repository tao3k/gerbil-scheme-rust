// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use rust_lang_project_harness::{
    RustScenarioBenchmarkStatus, validate_required_rust_scenario_benchmarks,
};

#[test]
fn all_required_compiler_scenarios_have_valid_benchmark_contracts() {
    let receipt = validate_required_rust_scenario_benchmarks(env!("CARGO_MANIFEST_DIR"))
        .expect("validate all required Gerbil compiler scenarios");

    assert_eq!(
        receipt.status,
        RustScenarioBenchmarkStatus::Pass,
        "scenario receipts: {:#?}; suite violations: {:#?}",
        receipt.receipts,
        receipt.violations,
    );
    assert_eq!(receipt.requirements.len(), 1);
    assert_eq!(receipt.receipts.len(), 1);
    assert_eq!(
        receipt
            .receipts
            .iter()
            .filter(|scenario| scenario.status == RustScenarioBenchmarkStatus::Pass)
            .count(),
        1
    );
    assert_eq!(
        receipt
            .receipts
            .iter()
            .filter(|scenario| scenario.status == RustScenarioBenchmarkStatus::Fail)
            .count(),
        0
    );
    assert_eq!(
        receipt
            .receipts
            .iter()
            .filter(|scenario| scenario.status == RustScenarioBenchmarkStatus::Invalid)
            .count(),
        0
    );
    assert!(
        receipt
            .receipts
            .iter()
            .all(|scenario| !scenario.benchmark.observed_timings.is_empty()),
        "each Gerbil compiler scenario should expose observed timing contracts: {:#?}",
        receipt.receipts
    );
    assert!(receipt.violations.is_empty());
}
