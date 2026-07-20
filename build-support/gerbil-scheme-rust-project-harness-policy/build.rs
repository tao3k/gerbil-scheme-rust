fn main() {
    let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut config = rust_lang_project_harness::rust_harness_config_for_project(project_root);
    config.verification_policy.profile_hints.push(
        rust_lang_project_harness::RustVerificationProfileHint::new(
            std::path::PathBuf::from("src/lib.rs"),
            [rust_lang_project_harness::RustOwnerResponsibility::LatencySensitive],
        )
        .with_task_kinds([rust_lang_project_harness::RustVerificationTaskKind::Performance])
        .with_rationale(
            "gerbil-scheme-rust-project-harness-policy self-gate owns policy construction latency evidence",
        ),
    );
    let policy = rust_lang_project_harness::RustProjectHarnessDownstreamPolicy::new(
        "gerbil-scheme-rust-project-harness-policy",
        config,
    );
    rust_lang_project_harness::assert_rust_project_harness_downstream_policy_from_env(&policy);
}
