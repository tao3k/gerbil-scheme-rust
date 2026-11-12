//! The real C association implementation, through Cargo; no VM or load run.
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
#[cfg(target_os = "macos")]
use std::{
    process::Stdio,
    time::{Duration, Instant},
};

struct TestDirectory(PathBuf);
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn actual_private_select_binding_preserves_results_errno_and_cancellation() {
    check_private_select_binding(false);
    check_private_select_binding(true);
}

#[cfg(target_os = "macos")]
fn check_private_select_binding(sanitized: bool) {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "gerbil-owner-wait-binding-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    let directory = TestDirectory(directory);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let image = directory.0.join("wait-binding.dylib");
    let executable = directory.0.join("binding-contract");
    let extra_flags: &[&str] = if sanitized {
        &["-g", "-fsanitize=address,undefined"]
    } else {
        &[]
    };
    let compile = Command::new("cc")
        .args([
            "-std=c11",
            "-O2",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-dynamiclib",
            "-DGERBIL_OWNER_WAIT_SCOPE",
        ])
        .args(extra_flags)
        .arg(root.join("tests/owner_wait_binding_image.c"))
        .args([
            "-Wl,-exported_symbol,_gws_open",
            "-Wl,-exported_symbol,_gws_progress",
            "-Wl,-exported_symbol,_gws_finish",
            "-Wl,-exported_symbol,_gws_clock",
            "-Wl,-exported_symbol,_gws_select",
            "-Wl,-exported_symbol,_gws_original_distinct",
            "-Wl,-exported_symbol,_gws_on_enter",
            "-Wl,-exported_symbol,_gws_exhaustion_contract",
            "-Wl,-exported_symbol,_gws_legacy_recording",
        ])
        .arg("-o")
        .arg(&image)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let compile = Command::new("cc")
        .args(["-std=c11", "-O2", "-Wall", "-Wextra", "-Werror"])
        .args(extra_flags)
        .arg(root.join("tests/owner_wait_binding.c"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let mut child = Command::new(executable)
        .arg(image)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let finished = child.try_wait().unwrap().is_some();
        if Instant::now() >= deadline {
            if !finished {
                child.kill().unwrap();
            }
            let result = child.wait_with_output().unwrap();
            panic!(
                "native wait binding contract exceeded five seconds: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        if finished {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, b"OWNER-WAIT-BINDING-CONTRACT-OK\n");
    assert!(
        result.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn owner_local_wait_scope_pairs_calls_and_rejects_stale_events() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "gerbil-owner-wait-scope-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    let directory = TestDirectory(directory);
    let executable = directory.0.join("scope-contract");
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/owner_wait_scope.c");
    let compile = Command::new("cc")
        .args([
            "-std=c11",
            "-O2",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic",
        ])
        .arg(source)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let result = Command::new(executable).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, b"OWNER-WAIT-SCOPE-CONTRACT-OK\n");
}
