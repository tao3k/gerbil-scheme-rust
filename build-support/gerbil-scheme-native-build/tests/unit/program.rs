//! Reject malformed AOT plans before invoking a compiler or creating an archive.

use gerbil_scheme_native_build::{
    ProgramArchiveContract, ProgramArchiveObservation, ProgramArchiveObserver,
    ProgramArchiveRequest, build_program_archive, build_program_archive_observed,
    build_program_archive_with_contract, source_workspace,
};
use serde_json::{Value, json};
use std::sync::Mutex;
use std::{fs, path::Path};
#[cfg(unix)]
use std::{
    os::unix::fs::PermissionsExt,
    sync::mpsc::{self, Sender},
    time::Duration,
};

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
    assert!(rejected_plan(&plan(json!([other]))).contains("required module"));
    assert!(rejected_plan(&plan(json!([bridge.clone(), bridge]))).contains("required module"));
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

struct Observations(Mutex<Vec<String>>);

impl ProgramArchiveObserver for Observations {
    fn observe(&self, observation: ProgramArchiveObservation<'_>) {
        self.0.lock().unwrap().push(observation.to_string());
    }
}

#[test]
fn observed_build_identifies_the_last_owned_phase_without_a_heartbeat() {
    let root = super::support::unique_temp_dir("gerbil-program-observation");
    fs::create_dir_all(&root).unwrap();
    let bridge = root.join("native.scm");
    fs::write(&bridge, "(display 'native)").unwrap();
    let manifest = root.join("program.json");
    fs::write(
        &manifest,
        serde_json::to_vec(&plan(json!([{
            "module": "gerbil-scheme-rust/scheme/native",
            "scm": bridge,
            "system": false
        }])))
        .unwrap(),
    )
    .unwrap();
    let observations = Observations(Mutex::new(Vec::new()));
    let error = build_program_archive_observed(
        ProgramArchiveRequest {
            manifest: &manifest,
            gsc: Path::new("/missing-compiler-must-not-run"),
            archive_name: "test_program",
            linker_name: "test_linker",
            out_dir: &root.join("out"),
        },
        &observations,
    )
    .unwrap_err();
    assert!(error.contains("generate program module C"));
    let rows = observations.0.into_inner().unwrap();
    assert_eq!(
        rows[0],
        "phase=program-plan state=complete operation=validate compiler-owned AOT manifest"
    );
    assert_eq!(
        rows[1],
        "phase=module-c-batch state=start operation=stage program module C sources subject=1"
    );
    assert_eq!(
        rows[2],
        "phase=module-c state=start operation=generate program module C subject=gerbil-scheme-rust/scheme/native"
    );
    assert!(rows[3].starts_with("phase=module-c state=failed operation=generate program module C subject=gerbil-scheme-rust/scheme/native elapsedMs="));
    assert!(rows[4].starts_with("phase=module-c-batch state=failed operation=stage program module C sources subject=1 elapsedMs="));
    assert_eq!(rows.len(), 5, "no timer heartbeat may manufacture rows");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn downstream_program_contract_selects_its_own_required_module_and_main_symbol() {
    let root = super::support::unique_temp_dir("gerbil-downstream-program-contract");
    fs::create_dir_all(&root).unwrap();
    let application = root.join("application.scm");
    fs::write(&application, "(display 'application)").unwrap();
    let manifest = root.join("program.json");
    fs::write(
        &manifest,
        serde_json::to_vec(&plan(json!([{
            "module": "example/application",
            "scm": application,
            "system": false
        }])))
        .unwrap(),
    )
    .unwrap();
    let observations = Observations(Mutex::new(Vec::new()));
    let error = build_program_archive_with_contract(
        ProgramArchiveRequest {
            manifest: &manifest,
            gsc: Path::new("/missing-compiler-must-not-run"),
            archive_name: "test_program",
            linker_name: "test_linker",
            out_dir: &root.join("out"),
        },
        ProgramArchiveContract {
            required_modules: &["example/application"],
            linker_main_symbol: "example_program_main",
            additional_objects: &[],
        },
        &observations,
    )
    .unwrap_err();
    assert!(error.contains("generate program module C"));
    assert!(
        observations.0.into_inner().unwrap()[2].contains("subject=example/application"),
        "the downstream module identity must survive into observation"
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
struct ChannelObservations(Sender<String>);

#[cfg(unix)]
impl ProgramArchiveObserver for ChannelObservations {
    fn observe(&self, observation: ProgramArchiveObservation<'_>) {
        self.0.send(observation.to_string()).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn slow_native_child_publishes_its_phase_before_exit_without_repeated_heartbeats() {
    let root = super::support::unique_temp_dir("gerbil-program-slow-child");
    fs::create_dir_all(&root).unwrap();
    let bridge = root.join("native.scm");
    fs::write(&bridge, "(display 'native)").unwrap();
    let manifest = root.join("program.json");
    fs::write(
        &manifest,
        serde_json::to_vec(&plan(json!([{
            "module": "gerbil-scheme-rust/scheme/native",
            "scm": bridge,
            "system": false
        }])))
        .unwrap(),
    )
    .unwrap();
    let compiler = root.join("slow-gsc");
    fs::write(&compiler, "#!/bin/sh\nsleep 1\nexit 1\n").unwrap();
    fs::set_permissions(&compiler, fs::Permissions::from_mode(0o755)).unwrap();
    let out = root.join("out");
    let (sender, receiver) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let observer = ChannelObservations(sender);
        build_program_archive_observed(
            ProgramArchiveRequest {
                manifest: &manifest,
                gsc: &compiler,
                archive_name: "test_program",
                linker_name: "test_linker",
                out_dir: &out,
            },
            &observer,
        )
    });

    assert_eq!(
        receiver.recv_timeout(Duration::from_millis(250)).unwrap(),
        "phase=program-plan state=complete operation=validate compiler-owned AOT manifest"
    );
    assert_eq!(
        receiver.recv_timeout(Duration::from_millis(250)).unwrap(),
        "phase=module-c-batch state=start operation=stage program module C sources subject=1"
    );
    assert_eq!(
        receiver.recv_timeout(Duration::from_millis(250)).unwrap(),
        "phase=module-c state=start operation=generate program module C subject=gerbil-scheme-rust/scheme/native"
    );
    assert!(
        !worker.is_finished(),
        "the native child must still own execution"
    );
    assert!(
        receiver.recv_timeout(Duration::from_millis(250)).is_err(),
        "a blocked child must not manufacture timer heartbeat rows"
    );
    assert!(worker.join().unwrap().is_err());
    assert!(receiver.recv().unwrap().starts_with(
        "phase=module-c state=failed operation=generate program module C subject=gerbil-scheme-rust/scheme/native elapsedMs="
    ));
    assert!(receiver.recv().unwrap().starts_with(
        "phase=module-c-batch state=failed operation=stage program module C sources subject=1 elapsedMs="
    ));
    assert!(receiver.try_recv().is_err());
    fs::remove_dir_all(root).unwrap();
}
