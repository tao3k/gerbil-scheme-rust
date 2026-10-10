use crate::{configure_gerbil_runtime_diagnostics, gerbil_command};
use std::ffi::OsStr;

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
