//! Actual image-local C lifecycle code; real VM coverage is a separate gate.
#[cfg(target_os = "macos")]
#[test]
fn cached_provider_preserves_hot_calls_and_failure_reclamation() {
    use std::{
        fs,
        path::PathBuf,
        process::Command,
        time::{Duration, Instant},
    };
    struct Directory(PathBuf);
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let path = std::env::temp_dir().join(format!("gerbil-timer-cache-{}", std::process::id()));
    fs::create_dir(&path).unwrap();
    let directory = Directory(path);
    let executable = directory.0.join("cache-contract");
    let compiler = std::env::var_os("GERBIL_GCC").unwrap_or_else(|| "cc".into());
    let output = Command::new(compiler)
        .args(["-std=c11", "-O2", "-Wall", "-Wextra", "-Werror"])
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/owner_timer_cache.c"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for case in [0, 1, 2, 3, 4, 6, 7] {
        let mut child = Command::new(&executable)
            .arg(case.to_string())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "cache case {case}: {status}");
                break;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("cache case {case}: five-second deadline");
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
