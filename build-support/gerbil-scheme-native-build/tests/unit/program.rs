//! Reject malformed AOT plans before invoking a compiler or creating an archive.

use gerbil_scheme_native_build::{ProgramArchiveRequest, build_program_archive, source_workspace};
use serde_json::{Value, json};
use std::{fs, path::Path};

#[test]
fn source_workspace_is_owned_by_the_resolved_crate() {
    let workspace = source_workspace();
    assert!(workspace.join("build.ss").is_file());
    assert!(workspace.join("scheme/program-build.ss").is_file());
}

fn rejected_plan(plan: &Value) -> String {
    let root = super::support::unique_temp_dir("gerbil-program-rejection");
    fs::create_dir_all(&root).unwrap();
    let manifest = root.join("program.json");
    fs::write(&manifest, serde_json::to_vec(plan).unwrap()).unwrap();
    let error = build_program_archive(ProgramArchiveRequest {
        manifest: &manifest,
        gsc: Path::new("/missing-compiler-must-not-run"),
        archive_name: "test_program",
        linker_name: "test_linker",
        out_dir: &root.join("out"),
    })
    .unwrap_err();
    assert!(!root.join("out").exists());
    fs::remove_dir_all(root).unwrap();
    error
}

fn plan(modules: Value) -> Value {
    json!({"schema": "gerbil-scheme-rust.aot-program.v1", "modules": modules,
        "stub": "unused.scm", "library_dir": "unused", "link_options": []})
}

#[test]
fn program_requires_exactly_one_bridge() {
    let bridge =
        json!({"module": "gerbil-scheme-rust/scheme/native", "scm": "unused.scm", "system": false});
    let other = json!({"module": "example/application", "scm": "unused.scm", "system": false});
    assert!(rejected_plan(&plan(json!([other]))).contains("exactly one native bridge"));
    assert!(
        rejected_plan(&plan(json!([bridge.clone(), bridge]))).contains("exactly one native bridge")
    );
}

#[test]
fn program_rejects_empty_unknown_and_unversioned_plans() {
    assert_eq!(
        rejected_plan(&plan(json!([]))),
        "invalid AOT program manifest"
    );
    let mut unknown = plan(json!([]));
    unknown["caller_override"] = json!(true);
    assert!(rejected_plan(&unknown).contains("unknown field"));
    let mut unversioned = plan(
        json!([{"module": "gerbil-scheme-rust/scheme/native", "scm": "unused.scm", "system": false}]),
    );
    unversioned["schema"] = json!("other");
    assert_eq!(rejected_plan(&unversioned), "invalid AOT program manifest");
}

#[test]
fn program_rejects_linker_arguments_before_reading_input() {
    let error = build_program_archive(ProgramArchiveRequest {
        manifest: Path::new("/missing-manifest-must-not-open"),
        gsc: Path::new("/missing-compiler-must-not-run"),
        archive_name: "test_program",
        linker_name: "invalid linker",
        out_dir: Path::new("/missing-output-must-not-create"),
    })
    .unwrap_err();
    assert_eq!(error, "invalid native linker name");
}
