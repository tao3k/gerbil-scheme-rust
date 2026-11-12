// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Wait for real host children concurrently with the actual VM lifecycle.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
};

struct Stop<'a>(&'a AtomicBool);
impl Drop for Stop<'_> {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

pub(super) fn during<T>(operation: impl FnOnce() -> T) -> T {
    let stop = AtomicBool::new(false);
    let (ready, receiver) = mpsc::sync_channel(1);
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| -> Result<usize, String> {
            let mut waited = 0;
            loop {
                let status = std::process::Command::new("/bin/sh")
                    .args(["-c", "exit 37"])
                    .status()
                    .map_err(|error| format!("host child wait failed: {error}"))?;
                if status.code() != Some(37) {
                    return Err(format!("host child status changed: {status}"));
                }
                waited += 1;
                if waited == 1 {
                    ready.send(()).map_err(|error| error.to_string())?;
                }
                if stop.load(Ordering::Acquire) {
                    return Ok(waited);
                }
            }
        });
        let stop_on_unwind = Stop(&stop);
        receiver
            .recv_timeout(std::time::Duration::from_secs(15))
            .expect("host child supervisor did not start");
        let result = operation();
        drop(stop_on_unwind);
        let waited = worker
            .join()
            .expect("host child supervisor panicked")
            .expect("VM lifecycle stole or changed a host child status");
        assert!(waited > 0);
        result
    })
}

#[cfg(unix)]
pub(super) fn assert_host_broken_pipe() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("true")
        .stdin(Stdio::piped())
        .spawn()
        .expect("create a real host pipe");
    let mut input = child.stdin.take().expect("parent pipe writer");
    assert!(child.wait().expect("close the reader").success());
    assert_eq!(
        input
            .write_all(b"closed")
            .expect_err("reader already exited")
            .kind(),
        std::io::ErrorKind::BrokenPipe,
    );
}

#[cfg(target_os = "linux")]
pub(super) fn host_allowed_cpus() -> String {
    std::fs::read_to_string("/proc/thread-self/status")
        .expect("read calling thread CPU affinity")
        .lines()
        .find_map(|line| line.strip_prefix("Cpus_allowed_list:").map(str::trim))
        .expect("calling thread has an allowed CPU list")
        .to_owned()
}
