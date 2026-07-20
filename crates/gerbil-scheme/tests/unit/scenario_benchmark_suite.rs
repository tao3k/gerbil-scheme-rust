use rust_lang_project_harness::{
    RustScenarioBenchmarkStatus, validate_required_rust_scenario_benchmarks,
};

#[test]
fn all_required_safe_value_scenarios_have_valid_contracts() {
    let receipt = validate_required_rust_scenario_benchmarks(env!("CARGO_MANIFEST_DIR"))
        .expect("validate all required gerbil-scheme safe value scenarios");
    assert_eq!(
        receipt.status,
        RustScenarioBenchmarkStatus::Pass,
        "scenario receipts: {:#?}; violations: {:#?}",
        receipt.receipts,
        receipt.violations,
    );
    let requirements = format!("{:#?}", receipt.requirements);
    let receipts = format!("{:#?}", receipt.receipts);
    for scenario_id in [
        "safe-scalar-projection",
        "safe-bytevector-borrow",
        "safe-vector-borrow",
        "safe-handle-backed-views",
        "safe-pair-list-status",
    ] {
        assert!(
            requirements.contains(scenario_id),
            "missing scenario requirement {scenario_id}: {requirements}",
        );
        assert!(
            receipts.contains(scenario_id),
            "missing scenario receipt {scenario_id}: {receipts}",
        );
    }
    assert!(receipt.violations.is_empty(), "{:#?}", receipt.violations);
}
