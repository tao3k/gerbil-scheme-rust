//! Qualification through the actual gerbil-scheme `InstancePool` API.
#[path = "instance_library/contracts.rs"]
mod contracts;
#[path = "instance_library/workload.rs"]
mod workload;
use gerbil_scheme::instances::{InstanceConfig, InstancePool, IsolatedProgram, ShutdownReceipt};
use std::{sync::mpsc, thread, time::Duration};
type Progress = mpsc::Sender<&'static str>;
const GATE: Duration = Duration::from_secs(5);

fn program() -> IsolatedProgram {
    // SAFETY: build.rs links the production native lifecycle and strong-root
    // ABI into a private image. Scheme catches all application exceptions.
    unsafe {
        IsolatedProgram::from_image(
            gerbil_scheme_qualification::owner_instance_image(),
            c"gerbil_owner_json_call",
        )
    }
    .expect("build-owned image")
}
fn config(owners: usize) -> InstanceConfig {
    InstanceConfig {
        owners,
        queue_capacity: 16,
        maximum_request_bytes: 16 * 1024 * 1024,
        maximum_reply_bytes: 1024 * 1024,
        startup_timeout: GATE,
    }
}
fn observed(progress: &Progress, event: &'static str) {
    progress.send(event).expect("watchdog");
}
fn drained(pool: &mut InstancePool, progress: &Progress) -> ShutdownReceipt {
    let receipt = pool.shutdown(GATE);
    assert!(receipt.is_clean());
    assert_eq!(receipt.pending, 0);
    assert_eq!(receipt.panicked, 0);
    assert_eq!(receipt.joined, pool.owners().len());
    assert_eq!(receipt.owners.len(), pool.owners().len());
    for owner in &receipt.owners {
        assert_eq!(owner.roots_before_cleanup, 0);
        assert_eq!(owner.status, 0);
        eprintln!(
            "LIBRARY-CLEANUP owner={} roots={} collections={} cleanup-ns={}",
            owner.owner,
            owner.roots_before_cleanup,
            owner.gc_collections,
            owner.cleanup_time.as_nanos()
        );
    }
    observed(progress, "observed clean owner joins");
    receipt
}
fn main() {
    let arguments: Vec<_> = std::env::args().collect();
    let (progress, observations) = mpsc::channel();
    let watchdog = thread::spawn(move || {
        let mut last = "start";
        loop {
            match observations.recv_timeout(GATE) {
                Ok(event) => last = event,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    eprintln!("LIBRARY-STALL last={last} cleanup=UNVERIFIED");
                    std::process::exit(70);
                }
            }
        }
    });
    if let Some(index) = arguments.iter().position(|argument| argument == "--cold") {
        workload::cold(arguments[index + 1].parse().expect("owners"), &progress);
    } else {
        contracts::run(&progress);
        workload::matched(
            arguments.iter().any(|argument| argument == "--test"),
            &progress,
        );
    }
    drop(progress);
    watchdog.join().expect("watchdog join");
}
