use crate::gerbil_command;
use std::ffi::OsStr;

#[test]
fn selected_sdk_command_removes_ambient_apple_tool_redirects() {
    let command = gerbil_command("selected-gsc");
    assert_eq!(command.get_program(), OsStr::new("selected-gsc"));
    for name in ["SDKROOT", "DEVELOPER_DIR"] {
        assert!(
            command
                .get_envs()
                .any(|(key, value)| { key == OsStr::new(name) && value.is_none() })
        );
    }
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
