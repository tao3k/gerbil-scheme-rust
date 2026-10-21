//! Focused same-VM owner capacity/backpressure and explicit Scheme Actor batches.
#[path = "support/orders.rs"]
mod corpus;
#[path = "owner_capacity/measurement.rs"]
mod measurement;
#[path = "owner_capacity/producer.rs"]
mod producer;
#[path = "owner_capacity/transfer.rs"]
mod transfer;
#[path = "owner_capacity/workload.rs"]
mod workload;
use gerbil_scheme::instances::{
    BatchAdmission, InstanceConfig, InstanceError, InstancePool, IsolatedProgram,
};
use measurement::Case;
use serde_json::json;
use std::{
    sync::{Arc, mpsc},
    time::Duration,
};
const GATE: Duration = Duration::from_secs(5);
// One targeted pair: identical batch-8/work-8 workload, caller refill versus native owner producer.
const PAIRS: [[Case; 2]; 1] = [[
    Case {
        capacity: 4,
        work: 8,
        batch: 8,
        producer: false,
        transfer_control: None,
    },
    Case {
        capacity: 4,
        work: 8,
        batch: 8,
        producer: true,
        transfer_control: None,
    },
]];
const TRANSFER_PAIRS: [[Case; 2]; 1] = [[
    Case {
        capacity: 4,
        work: 8,
        batch: 8,
        producer: false,
        transfer_control: Some(false),
    },
    Case {
        capacity: 4,
        work: 8,
        batch: 8,
        producer: false,
        transfer_control: Some(true),
    },
]];
fn batch_contracts(
    pool: &InstancePool,
    runtime: &tokio::runtime::Runtime,
    progress: &mpsc::Sender<&'static str>,
) {
    for owner in 0..4 {
        let dispatch = pool.dispatcher();
        let mut bad: Vec<_> = (0..8).map(|id| corpus::order(id, false)).collect();
        bad[0][3] = -1;
        let failure = runtime
            .block_on(async {
                dispatch
                    .submit_numeric_batch_backpressured(
                        bad,
                        BatchAdmission {
                            owner,
                            yield_every: 1,
                            timeout: GATE,
                        },
                    )
                    .await
                    .unwrap()
                    .wait_completed(GATE)
                    .await
            })
            .unwrap_err();
        assert!(
            matches!(&failure, InstanceError::Scheme(message) if message.contains("Scheme actor batch handler failed"))
        );
        let inputs: Vec<_> = (8..16).map(|id| corpus::order(id, false)).collect();
        let (outputs, _) = runtime
            .block_on(async {
                dispatch
                    .submit_numeric_batch_backpressured(
                        inputs.clone(),
                        BatchAdmission {
                            owner,
                            yield_every: 1,
                            timeout: GATE,
                        },
                    )
                    .await
                    .unwrap()
                    .wait_completed(GATE)
                    .await
            })
            .unwrap();
        for (input, output) in inputs.iter().zip(&outputs) {
            corpus::verify(input, output);
        }
        corpus::peers(&outputs);
        println!(
            "{}",
            json!({"event": "capacity_scheme_exception", "owner": owner, "batch": 8,
            "error": failure.to_string(), "recovery": true})
        );
        progress
            .send("native batch failure drained and recovered")
            .unwrap();
    }
    measurement::idle(pool);
}
fn run(progress: &mpsc::Sender<&'static str>, smoke: bool, transfer: bool) {
    // SAFETY: the existing Cargo graph owns the unchanged Scheme native batch ABI.
    let program = unsafe {
        IsolatedProgram::from_image(
            gerbil_scheme_qualification::owner_instance_image(),
            c"gerbil_owner_orders_call",
        )
    }
    .unwrap();
    let mut pool = InstancePool::start_observed(
        &program,
        InstanceConfig {
            owners: 4,
            queue_capacity: 128,
            maximum_request_bytes: 16 * 1024 * 1024,
            maximum_reply_bytes: 16 * 1024 * 1024,
            startup_timeout: GATE,
        },
        |_| progress.send("owner initialized").unwrap(),
    )
    .unwrap();
    pool.dispatcher()
        .configure_owner_budgets(128, Some(8))
        .unwrap();
    let states: Vec<_> = pool.owners().iter().map(|o| o.state).collect();
    println!(
        "{}",
        json!({"event": "capacity_configuration", "states": states, "owners": 4,
        "rust_workers": 2, "bulk_invocation_credit": 128, "probe_interval_us": 500,
        "case_scope": if transfer { "one bulk-native/rooted transfer pair; same VM and typed Scheme work" } else { "one batch-8 producer pair at work budget 8; same VM, no automatic default promotion" }})
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    batch_contracts(&pool, &runtime, progress);
    for mixed in [false, true] {
        let count = if smoke { 256 } else { 8192 };
        let probes = if smoke { 8 } else { 64 };
        let inputs = Arc::new(
            (0..count)
                .map(|id| corpus::order(id, mixed))
                .collect::<Vec<_>>(),
        );
        compare(
            &pool,
            &runtime,
            &inputs,
            (probes, smoke, mixed),
            if transfer { &TRANSFER_PAIRS } else { &PAIRS },
            progress,
        );
        assert_eq!(
            states,
            pool.owners().iter().map(|o| o.state).collect::<Vec<_>>()
        );
    }
    for owner in 0..4 {
        pool.dispatcher()
            .submit_native_on(owner, |session| {
                let input = session.numeric_vector(&[7_i64])?;
                let output = session.call(&input)?;
                assert_eq!(session.copy_numeric::<i64>(&output)?, [0]);
                drop(output);
                drop(input);
                session.collect()?;
                assert_eq!(session.usage()?[3].to_bits(), 0_f64.to_bits());
                Ok(())
            })
            .unwrap()
            .wait(GATE)
            .unwrap();
        progress.send("actor shutdown and GC").unwrap();
    }
    let closed = pool.shutdown(GATE);
    assert!(closed.is_clean(), "{closed:?}");
    assert_eq!(closed.joined, 4);
    assert!(
        closed
            .owners
            .iter()
            .all(|o| o.roots_before_cleanup == 0 && o.image_file_removed)
    );
    println!(
        "{}",
        json!({"event": "capacity_cleanup", "joined": 4, "roots": [0,0,0,0], "files_removed": true})
    );
}
fn compare(
    pool: &InstancePool,
    runtime: &tokio::runtime::Runtime,
    inputs: &Arc<Vec<Vec<i64>>>,
    (probes, smoke, mixed): (usize, bool, bool),
    pairs: &[[Case; 2]],
    progress: &mpsc::Sender<&'static str>,
) {
    for (pair, cases) in pairs.iter().enumerate() {
        let warm = Arc::new(inputs[..256.min(inputs.len())].to_vec());
        for &case in cases {
            measurement::run(pool, runtime, &warm, 8, case);
            progress.send("validated native warmup").unwrap();
        }
        let mut samples = [Vec::new(), Vec::new()];
        for block in 0..if smoke { 1 } else { 5 } {
            for reverse in [false, true] {
                for arm in if reverse { [1, 0] } else { [0, 1] } {
                    let sample = measurement::run(pool, runtime, inputs, probes, cases[arm]);
                    println!(
                        "{}",
                        json!({"event": "capacity_sample", "pair": pair, "arm": arm,
                        "block": block, "reverse": reverse, "mixed": mixed, "sample": sample})
                    );
                    samples[arm].push(sample);
                    progress
                        .send("validated native bulk and scheduled probes")
                        .unwrap();
                }
            }
        }
        for (arm, values) in samples.iter().enumerate() {
            measurement::emit(values, pair, arm, mixed);
        }
    }
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert!(
        args.iter()
            .all(|s| s == "--test" || s == "--bench" || s == "--transfer-ab")
    );
    let (progress, events) = mpsc::channel();
    let watchdog = std::thread::spawn(move || {
        let mut last = "start";
        loop {
            match events.recv_timeout(GATE) {
                Ok(event) => last = event,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    eprintln!("CAPACITY-BENCH-STALL last={last} cleanup=UNVERIFIED");
                    std::process::exit(70);
                }
            }
        }
    });
    run(
        &progress,
        args.iter().any(|s| s == "--test"),
        args.iter().any(|s| s == "--transfer-ab"),
    );
    drop(progress);
    watchdog.join().unwrap();
}
