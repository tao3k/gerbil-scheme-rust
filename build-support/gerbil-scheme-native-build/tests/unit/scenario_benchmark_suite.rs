// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use rust_lang_project_harness::{
    RustScenarioBenchmarkStatus, validate_required_rust_scenario_benchmarks,
};

#[test]
fn all_required_native_build_scenarios_have_valid_contracts() {
    let receipt = validate_required_rust_scenario_benchmarks(env!("CARGO_MANIFEST_DIR"))
        .expect("validate all required native build scenarios");
    assert_eq!(
        receipt.status,
        RustScenarioBenchmarkStatus::Pass,
        "scenario receipts: {:#?}; violations: {:#?}",
        receipt.receipts,
        receipt.violations,
    );
    assert_eq!(receipt.requirements.len(), 2);
    assert_eq!(receipt.receipts.len(), 2);
    assert!(receipt.violations.is_empty(), "{:#?}", receipt.violations);
}
