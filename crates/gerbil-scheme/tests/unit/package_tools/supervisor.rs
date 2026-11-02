//! Reuse the existing stdio-child isolation convention, with fixed real-output gates.
use std::io::Write;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;

#[cfg(unix)]
unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}

pub(super) fn run() -> bool {
    const CHILD: &str = "GERBIL_SCHEME_RUST_STDIO_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return false;
    }
    let workers = std::thread::available_parallelism().unwrap().get();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", "package_tools::build_test_and_failures_share_the_existing_owner", "--nocapture"])
            .env(CHILD, "1").stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
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
        while status.is_none() || out_open || err_open {
            tokio::select! {
                result = child.wait(), if status.is_none() => { status = Some(result.unwrap()); }
                read = stdout.read(&mut out), if out_open => {
                    let count = read.unwrap(); out_open = count != 0;
                    if count != 0 { std::io::stdout().write_all(&out[..count]).unwrap(); idle = tokio::time::Instant::now() + Duration::from_secs(5); }
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
    });
    drop(runtime);
    true
}
