//! Matched native owner costs, independent of the Python/GIL client boundary.
#[path = "support/orders.rs"]
mod corpus;
#[path = "owner_costs/sample.rs"]
mod sample;
use gerbil_scheme::instances::{InstanceConfig, InstancePool, IsolatedProgram};
use serde_json::json;
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
const GATE: Duration = Duration::from_secs(5);
fn observed(progress: &mpsc::Sender<&'static str>, event: &'static str) {
    progress.send(event).expect("progress watchdog");
}
fn run(progress: &mpsc::Sender<&'static str>, smoke: bool) {
    // SAFETY: the existing Cargo build owns this Scheme image and native root ABI.
    let program = unsafe {
        IsolatedProgram::from_image(
            gerbil_scheme_qualification::owner_instance_image(),
            c"gerbil_owner_orders_call",
        )
    }
    .unwrap();
    let jobs = if smoke { 128 } else { 1024 };
    let blocks = if smoke { 1 } else { 5 };
    for owners in [1, 4] {
        let started = Instant::now();
        let mut pool = InstancePool::start(
            &program,
            InstanceConfig {
                owners,
                queue_capacity: 128,
                maximum_request_bytes: 16 * 1024 * 1024,
                maximum_reply_bytes: 16 * 1024 * 1024,
                startup_timeout: GATE,
            },
        )
        .unwrap();
        println!(
            "{}",
            json!({"event": "owner_cost_setup", "owners": owners, "startup_ns": started.elapsed().as_nanos()})
        );
        observed(progress, "native owners initialized");
        for mixed in [false, true] {
            // Fixed corpus, constructed before the timed sample; conversion of
            // Rust records to Scheme types remains inside the native timed path.
            let inputs: Vec<_> = (0..jobs).map(|id| corpus::order(id, mixed)).collect();
            for batch in [1, 32] {
                for profiled in [false, true] {
                    sample::run(&pool, &inputs, batch, profiled, progress);
                }
                let mut samples: [Vec<sample::Sample>; 2] = std::array::from_fn(|_| Vec::new());
                for block in 0..blocks {
                    for arm in [0, 1, 1, 0] {
                        let value = sample::run(&pool, &inputs, batch, arm == 1, progress);
                        println!(
                            "{}",
                            json!({"event": "owner_cost_sample", "owners": owners, "mixed": mixed, "batch": batch, "block": block, "profiled": arm == 1, "elapsed_ns": value.elapsed, "latency_ns": value.latency})
                        );
                        samples[arm].push(value);
                    }
                }
                for (arm, samples) in samples.iter_mut().enumerate() {
                    sample::emit(samples, owners, mixed, batch, arm == 1, smoke, jobs);
                }
            }
        }
        // Close Scheme Actor workers before native teardown; no Scheme state or
        // rooted object crosses an owner or escapes into a Rust scheduler.
        for owner in 0..owners {
            pool.dispatcher()
                .submit_native_on(owner, |session| {
                    let input = session.numeric_vector(&[7_i64])?;
                    let result = session.call(&input)?;
                    assert_eq!(session.copy_numeric::<i64>(&result)?, [0]);
                    drop(result);
                    drop(input);
                    session.collect()?;
                    assert_eq!(session.usage()?[3].to_bits(), 0.0_f64.to_bits());
                    Ok(())
                })
                .unwrap()
                .wait(GATE)
                .unwrap();
            observed(progress, "Scheme actors closed and roots balanced");
        }
        let started = Instant::now();
        let receipt = pool.shutdown(GATE);
        assert!(receipt.is_clean(), "{receipt:?}");
        assert_eq!(receipt.joined, owners);
        assert!(
            receipt
                .owners
                .iter()
                .all(|owner| owner.roots_before_cleanup == 0 && owner.image_file_removed)
        );
        println!(
            "{}",
            json!({"event": "owner_cost_cleanup", "owners": owners, "shutdown_ns": started.elapsed().as_nanos(), "joined": receipt.joined, "roots": receipt.owners.iter().map(|o| o.roots_before_cleanup).collect::<Vec<_>>(), "files_removed": true})
        );
        observed(progress, "all owners retired");
    }
}
fn main() {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    assert!(
        arguments
            .iter()
            .all(|arg| arg == "--test" || arg == "--bench"),
        "supported arguments: --test (smoke), --bench (Cargo marker)"
    );
    let (progress, events) = mpsc::channel();
    let watchdog = thread::spawn(move || {
        let mut last = "start";
        loop {
            match events.recv_timeout(GATE) {
                Ok(event) => last = event,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    eprintln!("OWNER-COST-STALL last={last} cleanup=UNVERIFIED");
                    std::process::exit(70);
                }
            }
        }
    });
    run(&progress, arguments.iter().any(|arg| arg == "--test"));
    drop(progress);
    watchdog.join().unwrap();
}
