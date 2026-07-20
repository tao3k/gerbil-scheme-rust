// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use rust_lang_project_harness::{
    RustScenarioBenchmarkStatus, validate_required_rust_scenario_benchmarks,
};

#[test]
fn all_required_runtime_scenarios_have_valid_benchmark_contracts() {
    let receipt = validate_required_rust_scenario_benchmarks(env!("CARGO_MANIFEST_DIR"))
        .expect("validate all required Gerbil runtime scenarios");

    assert_eq!(
        receipt.status,
        RustScenarioBenchmarkStatus::Pass,
        "scenario receipts: {:#?}; suite violations: {:#?}",
        receipt.receipts,
        receipt.violations,
    );
    assert_eq!(receipt.requirements.len(), 5);
    assert_eq!(receipt.receipts.len(), 5);
    assert_eq!(receipt.summary.requirement_count, 5);
    assert_eq!(receipt.summary.receipt_count, 5);
    assert_eq!(receipt.summary.pass_count, 5);
    assert_eq!(receipt.summary.fail_count, 0);
    assert_eq!(receipt.summary.invalid_count, 0);
    assert_eq!(receipt.summary.violation_count, 0);
    assert!(
        receipt
            .summary
            .worst_observed_total_target_basis_points
            .is_some()
    );
    assert!(
        receipt
            .summary
            .worst_observed_total_max_basis_points
            .is_some()
    );
    assert!(
        receipt
            .summary
            .worst_observed_memory_budget_basis_points
            .is_some()
    );
    assert!(receipt.violations.is_empty());
}
