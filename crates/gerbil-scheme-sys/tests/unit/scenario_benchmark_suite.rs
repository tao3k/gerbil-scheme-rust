use gerbil_scheme_rust_project_harness::{
    RustScenarioBenchmarkStatus, validate_required_rust_scenario_benchmarks,
};

#[test]
fn all_required_sys_scenarios_have_valid_contracts() {
    let receipt = validate_required_rust_scenario_benchmarks(env!("CARGO_MANIFEST_DIR"))
        .expect("validate all required gerbil-scheme-sys scenarios");
    assert_eq!(
        receipt.status,
        RustScenarioBenchmarkStatus::Pass,
        "scenario receipts: {:#?}; violations: {:#?}",
        receipt.receipts,
        receipt.violations,
    );
    assert_eq!(receipt.requirements.len(), 3);
    assert_eq!(receipt.receipts.len(), 3);
    assert!(receipt.violations.is_empty(), "{:#?}", receipt.violations);
}
