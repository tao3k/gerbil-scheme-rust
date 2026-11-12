use crate::{configure_gerbil_runtime_diagnostics, gerbil_command};
use std::ffi::OsStr;

#[test]
fn build_children_and_object_workers_use_the_executing_host_capacity() {
    const CHILD: &str = "GERBIL_CORE_INJECTION_RECEIPT";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "compiler::environment_tests::build_children_and_object_workers_use_the_executing_host_capacity", "--nocapture"])
            .env(CHILD, "1")
            .env_remove("NUM_JOBS")
            .env("GERBIL_BUILD_CORES", "stale-untrusted-count")
            .output().unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let host = std::thread::available_parallelism().unwrap().get();
    let output = gerbil_command("printenv")
        .arg("GERBIL_BUILD_CORES")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        host.to_string()
    );
    assert_eq!(crate::program::native_build_parallelism(usize::MAX), host);
    assert_eq!(crate::program::native_build_parallelism(1), 1);
    println!("BUILD-CORE-RECEIPT actual-host={host} child={host} object-workers={host}");
}

#[test]
fn selected_sdk_command_removes_ambient_apple_tool_redirects() {
    let command = gerbil_command("selected-gsc");
    assert_eq!(command.get_program(), OsStr::new("selected-gsc"));
    for name in [
        "CC",
        "CFLAGS",
        "CPPFLAGS",
        "LDFLAGS",
        "CPATH",
        "C_INCLUDE_PATH",
        "CPLUS_INCLUDE_PATH",
        "LIBRARY_PATH",
        "NIX_CFLAGS_COMPILE",
        "NIX_LDFLAGS",
        "SDKROOT",
        "DEVELOPER_DIR",
    ] {
        assert!(
            command
                .get_envs()
                .any(|(key, value)| { key == OsStr::new(name) && value.is_none() })
        );
    }
}

#[test]
fn runtime_diagnostics_keep_selected_sdk_mappings() {
    let mut command = gerbil_command("selected-gxi");
    command.env("GAMBOPT", "~~=/sdk,~~lib=/sdk/lib");
    configure_gerbil_runtime_diagnostics(&mut command, true);
    assert!(command.get_envs().any(|(key, value)| {
        key == OsStr::new("GAMBOPT")
            && value == Some(OsStr::new("~~=/sdk,~~lib=/sdk/lib,1n,2n,d5qQ"))
    }));

    let mut quiet = gerbil_command("selected-gxi");
    quiet.env("GAMBOPT", "~~=/sdk");
    configure_gerbil_runtime_diagnostics(&mut quiet, false);
    assert!(quiet.get_envs().any(|(key, value)| {
        key == OsStr::new("GAMBOPT") && value == Some(OsStr::new("~~=/sdk"))
    }));
}

#[cfg(target_os = "macos")]
#[test]
fn darwin_system_linker_resolves_through_clean_selected_sdk_command() {
    let output = gerbil_command("/usr/bin/xcrun")
        .args(["--find", "ld"])
        .output()
        .expect("resolve actual system linker");
    assert!(output.status.success(), "system linker resolution failed");
    let path = String::from_utf8(output.stdout).expect("linker path UTF-8");
    let path = std::path::Path::new(path.trim());
    assert!(path.is_file(), "resolved linker is missing");
    assert!(
        !path.starts_with("/nix/store"),
        "foreign SDK linker selected"
    );
}

#[test]
fn native_object_workers_obey_cargo_budget_without_changing_host_discovery() {
    const CHILD: &str = "GERBIL_NATIVE_BUDGET_RECEIPT";
    if std::env::var_os(CHILD).is_none() {
        let host = std::thread::available_parallelism().unwrap().get();
        for budget in [
            "1".to_owned(),
            (host + 1).to_string(),
            "0".to_owned(),
            "invalid".to_owned(),
        ] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "compiler::environment_tests::native_object_workers_obey_cargo_budget_without_changing_host_discovery", "--nocapture"])
                .env(CHILD, "1").env("NUM_JOBS", &budget)
                .env("GERBIL_BUILD_CORES", "stale-untrusted-count")
                .output().unwrap();
            assert!(
                output.status.success(),
                "budget={budget}: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        return;
    }
    let budget = std::env::var("NUM_JOBS")
        .unwrap()
        .parse::<std::num::NonZeroUsize>();
    if budget.is_err() {
        let outcome =
            std::panic::catch_unwind(|| crate::program::native_build_parallelism(usize::MAX));
        assert!(
            outcome.is_err(),
            "actual compiler pool accepted an invalid Cargo budget"
        );
        return;
    }
    let actual = crate::program::native_build_parallelism(usize::MAX);
    let budget = budget.unwrap().get();
    let host = std::thread::available_parallelism().unwrap().get();
    assert_eq!(actual, host.min(budget));
    assert_eq!(crate::program::native_build_parallelism(1), 1);
    let output = gerbil_command("printenv")
        .arg("GERBIL_BUILD_CORES")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        host.to_string()
    );
}
