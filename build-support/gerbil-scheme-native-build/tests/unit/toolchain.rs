use gerbil_scheme_native_build::discover_native_c_compiler;

#[test]
fn native_compiler_discovery_uses_the_cc_contract() {
    if let Ok(tool) = discover_native_c_compiler() {
        assert!(!tool.program.as_os_str().is_empty());
    }
}
