//! Constructs and enforces the `gerbil-scheme-rust` downstream harness policy.

use std::path::Path;

use rust_lang_project_harness::RustProjectHarnessDownstreamPolicy;

/// Build the repository policy for a Cargo project rooted at `project_root`.
#[must_use]
pub fn rust_project_harness_policy_for_project(
    project_root: &Path,
) -> RustProjectHarnessDownstreamPolicy {
    let mut config = rust_lang_project_harness::rust_harness_config_for_project(project_root);
    config.verification_policy.profile_hints.push(
        rust_lang_project_harness::RustVerificationProfileHint::new(
            std::path::PathBuf::from("src/lib.rs"),
            [
                rust_lang_project_harness::RustOwnerResponsibility::LatencySensitive,
                rust_lang_project_harness::RustOwnerResponsibility::AvailabilityCritical,
            ],
        )
        .with_task_kinds([
            rust_lang_project_harness::RustVerificationTaskKind::Performance,
            rust_lang_project_harness::RustVerificationTaskKind::Stability,
        ])
        .with_rationale(
            "Gerbil Scheme bindings require stable ABI behavior and bounded native build latency",
        ),
    );
    if config.verification_policy.stability_picture.is_none() {
        config.verification_policy.stability_picture =
            Some(rust_lang_project_harness::RustVerificationStabilityPictureConfig::default());
    }
    RustProjectHarnessDownstreamPolicy::new("gerbil-scheme-rust", config)
}

/// Enforce the project harness gate when the harness environment enables it.
pub fn assert_gerbil_scheme_rust_project_harness_gate_from_env(project_root: &Path) {
    let policy = rust_project_harness_policy_for_project(project_root);
    rust_lang_project_harness::assert_rust_project_harness_downstream_policy_from_env(&policy);
}
