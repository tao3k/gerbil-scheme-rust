fn main() {
    let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    gerbil_scheme_rust_project_harness_policy::assert_gerbil_scheme_rust_project_harness_gate_from_env(
        project_root,
    );
}
