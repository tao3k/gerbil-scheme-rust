//! Same live native Scheme owners and input corpus across Rust dispatchers.
#[path = "owner_dispatch/attribution.rs"]
mod attribution;
#[path = "owner_dispatch/comparison.rs"]
mod comparison;
#[path = "support/orders.rs"]
mod corpus;
#[path = "owner_dispatch/executor.rs"]
mod executor;
#[path = "owner_dispatch/grouped_attribution.rs"]
mod grouped_attribution;
#[path = "owner_dispatch/sample.rs"]
mod sample;
use executor::Executor;
use gerbil_scheme::instances::{InstanceConfig, InstanceError, InstancePool, IsolatedProgram};
use serde_json::json;
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
const GATE: Duration = Duration::from_secs(5);
const CASES: [(&str, usize); 7] = [
    ("direct", 0),
    ("tokio", 1),
    ("rayon", 1),
    ("tokio", 2),
    ("rayon", 2),
    ("tokio", 4),
    ("rayon", 4),
];
fn observed(progress: &mpsc::Sender<&'static str>, event: &'static str) {
    progress.send(event).expect("progress watchdog");
}
fn contracts(pool: &InstancePool, executor: &Executor, progress: &mpsc::Sender<&'static str>) {
    // All dispatchers must propagate real Scheme failure and recover on every
    // owner, while the async API retains the same pre-admission bounds.
    assert!(matches!(
        pool.dispatcher()
            .submit_numeric_batch_async_on::<i64>(0, vec![], 1),
        Err(InstanceError::InvalidConfig(
            "native batch item/quantum bound"
        ))
    ));
    for owner in 0..pool.owners().len() {
        let mut bad = corpus::order(0, false);
        bad[3] = -1;
        let peers = [corpus::order(71, false), corpus::order(72, false)];
        let result = executor.window(
            &pool.dispatcher(),
            vec![
                (owner, vec![bad]),
                (owner, vec![peers[0].clone()]),
                (owner, vec![peers[1].clone()]),
            ],
            Instant::now(),
        );
        let mut replies = result.into_iter();
        let error = replies.next().unwrap().unwrap_err();
        for input in &peers {
            let (output, _) = replies.next().unwrap().unwrap();
            corpus::verify(input, &output[0]);
        }
        assert!(replies.next().is_none());
        println!(
            "{}",
            json!({"event": "dispatch_scheme_exception", "mode": executor.name, "workers": executor.workers, "grouped": executor.grouped, "owner": owner, "error": error.to_string()})
        );
        assert!(
            matches!(&error, InstanceError::Scheme(message) if message.contains("Scheme actor batch handler failed")),
            "expected drained Scheme Actor handler failure, got {error:?}"
        );
        let input = corpus::order(1, false);
        let result = executor.window(
            &pool.dispatcher(),
            vec![(owner, vec![input.clone()])],
            Instant::now(),
        );
        let (output, _) = result.into_iter().next().unwrap().unwrap();
        corpus::verify(&input, &output[0]);
        observed(
            progress,
            "observed Scheme exception propagation and recovery",
        );
    }
}
fn run(progress: &mpsc::Sender<&'static str>, smoke: bool, profile: bool, grouped_ab: bool) {
    // SAFETY: the existing Cargo build owns this Scheme image and native root ABI.
    let program = unsafe {
        IsolatedProgram::from_image(
            gerbil_scheme_qualification::owner_instance_image(),
            c"gerbil_owner_orders_call",
        )
    }
    .unwrap();
    for owners in [1, 4] {
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
        let states: Vec<_> = pool.owners().iter().map(|o| o.state).collect();
        println!(
            "{}",
            json!({"event": "dispatch_owners", "owners": owners, "states": states, "caller_threads": 1, "watchdog_threads": 1})
        );
        observed(progress, "native Scheme owners initialized");
        for &(mode, workers) in &CASES {
            for grouped in [false, true] {
                if grouped && !grouped_ab {
                    continue;
                }
                let executor = Executor::start(mode, workers, profile, grouped);
                contracts(&pool, &executor, progress);
                executor.finish();
                observed(progress, "dispatcher contract workers retired");
            }
        }
        comparison::run(&pool, smoke, profile, grouped_ab, progress);
        finish(&mut pool, progress);
    }
}
fn finish(pool: &mut InstancePool, progress: &mpsc::Sender<&'static str>) {
    let owners = pool.owners().len();
    for owner in 0..owners {
        pool.dispatcher()
            .submit_native_on(owner, |session| {
                let input = session.numeric_vector(&[7_i64])?;
                let output = session.call(&input)?;
                assert_eq!(session.copy_numeric::<i64>(&output)?, [0]);
                drop(output);
                drop(input);
                session.collect()?;
                assert_eq!(session.usage()?[3].to_bits(), 0.0_f64.to_bits());
                Ok(())
            })
            .unwrap()
            .wait(GATE)
            .unwrap();
        observed(progress, "Scheme Actor services closed and roots balanced");
    }
    let receipt = pool.shutdown(GATE);
    assert!(receipt.is_clean(), "{receipt:?}");
    assert_eq!(receipt.joined, owners);
    assert!(
        receipt
            .owners
            .iter()
            .all(|o| o.roots_before_cleanup == 0 && o.image_file_removed)
    );
    println!(
        "{}",
        json!({"event": "dispatch_cleanup", "owners": owners, "joined": receipt.joined, "roots": receipt.owners.iter().map(|o| o.roots_before_cleanup).collect::<Vec<_>>(), "files_removed": true})
    );
    observed(progress, "native Scheme owners retired");
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert!(
        args.iter()
            .all(|a| a == "--test" || a == "--bench" || a == "--profile" || a == "--grouped-ab"),
        "supported arguments: --test, --bench, --profile, --grouped-ab"
    );
    let (progress, events) = mpsc::channel();
    let watchdog = thread::spawn(move || {
        let mut last = "start";
        loop {
            match events.recv_timeout(GATE) {
                Ok(event) => last = event,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    eprintln!("DISPATCH-STALL last={last} cleanup=UNVERIFIED");
                    std::process::exit(70);
                }
            }
        }
    });
    println!(
        "{}",
        json!({"event": "dispatch_configuration", "diagnostic": args.iter().any(|a| a == "--profile"), "grouped_ab": args.iter().any(|a| a == "--grouped-ab")})
    );
    run(
        &progress,
        args.iter().any(|a| a == "--test"),
        args.iter().any(|a| a == "--profile"),
        args.iter().any(|a| a == "--grouped-ab"),
    );
    drop(progress);
    watchdog.join().unwrap();
}
