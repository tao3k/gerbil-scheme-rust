//! Reuse the existing stdio-child isolation convention, with fixed real-output gates.
use std::io::Write;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;

#[cfg(unix)]
unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}

static PACKAGE_CHILD: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(super) fn run() -> bool {
    const CHILD: &str = "GERBIL_SCHEME_RUST_STDIO_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return false;
    }
    // Each existing native gate keeps its own fixed budget and host capacity.
    // Serialize independent one-shot VM cases instead of making them compete.
    let _serial = PACKAGE_CHILD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let test_name = std::thread::current()
        .name()
        .expect("named test thread")
        .to_owned();
    let workers = std::thread::available_parallelism().unwrap().get();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let poisoned = std::env::temp_dir().join(format!("package-toolchain-poison-{}", std::process::id()));
        std::fs::create_dir_all(&poisoned).unwrap();
        std::fs::write(poisoned.join("stdio.h"), "#error unrelated host compiler headers leaked into Gerbil\n").unwrap();
        let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", &test_name, "--nocapture"])
            .env(CHILD, "1").env_remove("GERBIL_PATH").env_remove("GERBIL_LOADPATH").stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
        command.env("CPATH", &poisoned).env("SDKROOT", "/nix/store/invalid-host-sdk");
        #[cfg(unix)]
        { command.process_group(0); }
        let mut child = command.spawn().unwrap();
        let pid = child.id().unwrap();
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let start = tokio::time::Instant::now();
        let wall = start + Duration::from_secs(15);
        let mut idle = start + Duration::from_secs(5);
        let mut out = [0; 4096];
        let mut err = [0; 4096];
        let (mut out_open, mut err_open) = (true, true);
        let mut status = None;
        let mut progress = Vec::new();
        while status.is_none() || out_open || err_open {
            tokio::select! {
                result = child.wait(), if status.is_none() => { status = Some(result.unwrap()); }
                read = stdout.read(&mut out), if out_open => {
                    let count = read.unwrap(); out_open = count != 0;
                    if count != 0 { std::io::stdout().write_all(&out[..count]).unwrap(); acknowledge_progress(&mut progress, &out[..count], pid); idle = tokio::time::Instant::now() + Duration::from_secs(5); }
                }
                read = stderr.read(&mut err), if err_open => {
                    let count = read.unwrap(); err_open = count != 0;
                    if count != 0 { std::io::stderr().write_all(&err[..count]).unwrap(); idle = tokio::time::Instant::now() + Duration::from_secs(5); }
                }
                () = tokio::time::sleep_until(wall.min(idle)) => {
                    #[cfg(unix)]
                    // SAFETY: the child owns this new process group; this is failure cleanup only.
                    unsafe { kill(-(i32::try_from(pid).unwrap()), 9); }
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    panic!("package tools exceeded the unchanged 15s wall / 5s output gate; elapsed={:?}", start.elapsed());
                }
            }
        }
        let status = status.unwrap();
        assert!(status.success(), "C ABI owner failed: {status}; output retained above");
        #[cfg(unix)]
        {
            // SAFETY: signal zero observes only this newly owned group; no signal is delivered.
            // Linux and Darwin report ESRCH (3); Rust classifies it as Uncategorized.
            assert_eq!(unsafe { kill(-(i32::try_from(pid).unwrap()), 0) }, -1);
            assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(3));
        }
        println!("PACKAGE-TOOLS-NATURAL-EXIT-OK workers={workers} elapsed={:?}", start.elapsed());
        std::fs::remove_dir_all(poisoned).unwrap();
    });
    drop(runtime);
    true
}

// The foreign call waits for this receipt. Buffered-until-return output must
// fail the existing deadline; an entered event must reach the actual pipe.
fn acknowledge_progress(pending: &mut Vec<u8>, bytes: &[u8], pid: u32) {
    pending.extend_from_slice(bytes);
    while let Some(end) = pending.iter().position(|byte| *byte == b'\n') {
        let line: Vec<_> = pending.drain(..=end).collect();
        let line = std::str::from_utf8(&line).unwrap();
        if let Some(path) = line.trim_end().strip_prefix("PACKAGE-CANCEL-READY=") {
            let path = std::path::Path::new(path);
            assert_eq!(path.file_name().unwrap(), "output-observed");
            assert!(
                path.parent()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with(&format!("gerbil-package-{pid}-"))
            );
            assert!(!path.exists(), "foreign output receipt was replayed");
            std::fs::write(path, "received before foreign return").unwrap();
        }
    }
}
