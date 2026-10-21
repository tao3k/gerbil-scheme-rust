//! One directed pair: same VMs, native work and explicit budgets, Scheme-owned async service.
#[path = "support/orders.rs"]
mod corpus;
#[path = "scheme_service/measurement.rs"]
mod measurement;
#[path = "scheme_service/workload.rs"]
mod workload;
use gerbil_scheme::instances::{InstanceConfig, InstancePool, IsolatedProgram};
use serde_json::json;
use std::{
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};
const GATE: Duration = Duration::from_secs(5);
fn run(progress: &mpsc::Sender<&'static str>, smoke: bool) {
    // SAFETY: the build-owned image exports the admitted native service protocol.
    let program = unsafe {
        IsolatedProgram::from_image(
            gerbil_scheme_qualification::owner_instance_image(),
            c"gerbil_owner_orders_call",
        )
        .unwrap()
        .with_exports(&[c"gerbil_owner_service_reference"])
        .with_scheme_service(c"gerbil_owner_service_call")
    };
    let startup = Instant::now();
    let mut pool = InstancePool::start_observed(
        &program,
        InstanceConfig {
            owners: 4,
            queue_capacity: 128,
            maximum_request_bytes: 16 * 1024 * 1024,
            maximum_reply_bytes: 16 * 1024 * 1024,
            startup_timeout: GATE,
        },
        |_| {
            progress.send("Scheme service owner initialized").unwrap();
        },
    )
    .unwrap();
    pool.dispatcher()
        .configure_owner_budgets(4, Some(32))
        .unwrap();
    let identities: Vec<_> = pool.owners().iter().map(|r| r.state).collect();
    println!(
        "{}",
        json!({"event":"service_configuration","pool_startup_ns":startup.elapsed().as_nanos(),"sdk_setup_ns":pool.owners().iter().map(|r|r.setup_time.as_nanos()).collect::<Vec<_>>(),"owners":4,"states":identities,"rust_workers":2,"slots":4,"work":32,"batch":8,
        "quantum":1,"scheme_service_workers":4,"actor_workers_per_batch":4,"probe_interval_us":500,"defaults_changed":false,
        "comparison":"same native constructor, handler, projection and VM; synchronous return versus Scheme-owned pending service"})
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    for mixed in [false, true] {
        let inputs = Arc::new(
            (0..if smoke { 256 } else { 8192 })
                .map(|id| corpus::order(id, mixed))
                .collect::<Vec<_>>(),
        );
        let warm = Arc::new(inputs[..256].to_vec());
        for arm in [false, true] {
            measurement::run(&pool, &runtime, &warm, 8, arm);
            progress.send("service warmup validated").unwrap();
        }
        let mut samples = [Vec::new(), Vec::new()];
        for block in 0..if smoke { 1 } else { 5 } {
            for reverse in [false, true] {
                for arm in if reverse { [1, 0] } else { [0, 1] } {
                    let sample = measurement::run(
                        &pool,
                        &runtime,
                        &inputs,
                        if smoke { 8 } else { 64 },
                        arm == 1,
                    );
                    println!(
                        "{}",
                        json!({"event":"service_sample","arm":arm,"mixed":mixed,"block":block,"reverse":reverse,"sample":sample})
                    );
                    samples[arm].push(sample);
                    progress.send("service same-load sample validated").unwrap();
                }
            }
        }
        for (arm, values) in samples.iter().enumerate() {
            measurement::emit(values, arm, mixed);
        }
        assert_eq!(
            identities,
            pool.owners().iter().map(|r| r.state).collect::<Vec<_>>()
        );
    }
    let closed = pool.shutdown(GATE);
    assert!(closed.is_clean(), "{closed:?}");
    assert_eq!(closed.joined, 4);
    assert!(closed.owners.iter().all(|o| o.roots_before_cleanup == 0
        && o.finalizer_status == Some(0)
        && o.image_file_removed));
    println!(
        "{}",
        json!({"event":"service_cleanup","joined":4,"roots":[0,0,0,0],"files_removed":true,"service_quiesced":true})
    );
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert!(args.iter().all(|s| s == "--test" || s == "--bench"));
    let (progress, events) = mpsc::channel();
    let watchdog = std::thread::spawn(move || {
        let mut last = "start";
        loop {
            match events.recv_timeout(GATE) {
                Ok(event) => last = event,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    eprintln!("SERVICE-BENCH-STALL last={last} cleanup=UNVERIFIED");
                    std::process::exit(70);
                }
            }
        }
    });
    run(&progress, args.iter().any(|s| s == "--test"));
    drop(progress);
    watchdog.join().unwrap();
}
