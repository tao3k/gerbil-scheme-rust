#[path = "../../src/generated_scm.rs"]
mod implementation;

use std::fs;

use implementation::{stamp_generated_scm, validate_generated_scm, workspace_input_fingerprint};

use super::support::unique_temp_dir;

const INPUT_FINGERPRINT: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn generated_scm_input_fingerprint_tracks_all_build_inputs() {
    let workspace = unique_temp_dir("gerbil-native-build-generated-scm");
    fs::create_dir_all(workspace.join("scheme")).expect("create Scheme input root");
    fs::write(workspace.join("build.ss"), "build-v1\n").expect("write build.ss");
    fs::write(workspace.join("gerbil.pkg"), "package-v1\n").expect("write gerbil.pkg");
    fs::write(workspace.join("scheme/native.ss"), "native-v1\n").expect("write native.ss");
    fs::write(workspace.join("scheme/native.ssi"), "ffi-v1\n").expect("write native.ssi");
    let original = workspace_input_fingerprint(&workspace);

    fs::write(workspace.join("scheme/native.ss"), "native-v2\n").expect("change native.ss");
    let changed = workspace_input_fingerprint(&workspace);

    assert_ne!(original, changed);
    fs::remove_dir_all(workspace).expect("remove Scheme input root");
}

#[test]
fn generated_scm_provenance_accepts_untouched_body_and_inputs() {
    let tracked = stamp_generated_scm("(define answer 42)\n", INPUT_FINGERPRINT);

    assert_eq!(validate_generated_scm(&tracked, INPUT_FINGERPRINT), Ok(()));
}

#[test]
fn generated_scm_provenance_rejects_changed_inputs() {
    let tracked = stamp_generated_scm("(define answer 42)\n", INPUT_FINGERPRINT);
    let changed_input = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

    assert_eq!(
        validate_generated_scm(&tracked, changed_input),
        Err("input fingerprint does not match current Scheme build inputs")
    );
}

#[test]
fn generated_scm_provenance_rejects_changed_body() {
    let mut tracked = stamp_generated_scm("(define answer 42)\n", INPUT_FINGERPRINT);
    tracked.push_str("(define drift #t)\n");

    assert_eq!(
        validate_generated_scm(&tracked, INPUT_FINGERPRINT),
        Err("body fingerprint does not match committed SCM contents")
    );
}
