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
    assert_eq!(receipt.requirements.len(), 9);
    assert_eq!(receipt.receipts.len(), 9);
    assert_eq!(receipt.summary.requirement_count, 9);
    assert_eq!(receipt.summary.receipt_count, 9);
    assert_eq!(receipt.summary.pass_count, 9);
    assert_eq!(receipt.summary.fail_count, 0);
    assert_eq!(receipt.summary.invalid_count, 0);
    assert_eq!(receipt.summary.violation_count, 0);
    let expected_requirement_roots = [
        "tests/unit/scenarios/backed-value-family-surface",
        "tests/unit/scenarios/invalid-comparison-fail-closed",
        "tests/unit/scenarios/native-ffi-steady-state",
        "tests/unit/scenarios/native-identity-round-trip",
        "tests/unit/scenarios/native-result-contract",
        "tests/unit/scenarios/native-runtime-round-trip",
        "tests/unit/scenarios/native-value-surface",
        "tests/unit/scenarios/source-surface-sync",
        "tests/unit/scenarios/status-contract",
    ];
    let requirement_roots = receipt
        .requirements
        .iter()
        .map(|requirement| {
            requirement
                .root
                .strip_prefix(&receipt.root)
                .unwrap_or_else(|err| {
                    panic!(
                        "scenario requirement root must be under crate root: {}; err={err}",
                        requirement.root.display()
                    )
                })
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        requirement_roots, expected_requirement_roots,
        "required runtime scenarios drifted; update the explicit scenario contract deliberately",
    );
    assert!(
        receipt
            .requirements
            .iter()
            .all(|requirement| format!("{:?}", requirement.manifest_kind) == "ScenarioToml"),
        "all gerbil-scheme runtime scenario requirements should currently be scenario.toml-backed: {:#?}",
        receipt.requirements
    );
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
