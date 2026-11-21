#[test]
fn progress_does_not_enable_optimizer_dumps_or_change_optimization_and_abi() {
    let codegen = "-O2 -Dmain=mrr_grammar_gambit_main -fno-gcse-lm";
    assert_eq!(
        super::compiler_diagnostic_options(codegen, false, false),
        codegen
    );
    let progress = super::compiler_diagnostic_options(codegen, true, false);
    assert_eq!(progress, format!("{codegen} -save-temps=obj -Q"));
    assert!(!progress.contains("fopt-info"));
    assert_eq!(
        super::compiler_diagnostic_options(codegen, false, true),
        format!("{codegen} -fopt-info-all")
    );
}
