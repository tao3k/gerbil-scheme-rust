//! One directed pair: same VMs, native work and explicit budgets, Scheme-owned async service.
#[path = "scheme_service/budget.rs"]
mod budget;
#[path = "support/orders.rs"]
mod corpus;
#[path = "scheme_service/entry.rs"]
mod entry;
#[path = "scheme_service/measurement.rs"]
mod measurement;
#[path = "scheme_service/native_wait.rs"]
mod native_wait;
#[path = "scheme_service/observation.rs"]
mod observation;
#[path = "scheme_service/os_attribution.rs"]
mod os_attribution;
#[path = "scheme_service/os_calibration.rs"]
mod os_calibration;
#[path = "scheme_service/os_sampling.rs"]
mod os_sampling;
#[path = "scheme_service/os_threads.rs"]
mod os_threads;
#[path = "scheme_service/pooled.rs"]
mod pooled;
#[path = "scheme_service/resident.rs"]
mod resident;
#[path = "scheme_service/settings.rs"]
mod settings;
#[path = "scheme_service/workload.rs"]
mod workload;
use gerbil_scheme::instances::{InstanceConfig, InstancePool, IsolatedProgram};
use measurement::Comparison;
use observation::Mode;
use serde_json::json;
use std::{
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};
const GATE: Duration = Duration::from_secs(5);
fn run(
    progress: &mpsc::Sender<&'static str>,
    smoke: bool,
    comparison: Comparison,
    observation: Mode,
    trace_once: bool,
    blocks: usize,
) {
    let split = comparison.is_split();
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
    settings::short_pool(&pool, comparison);
    settings::progress(&pool, comparison);
    if observation.full() {
        for owner in 0..4 {
            pool.dispatcher().observe_scheme_service_on(owner).unwrap();
        }
    }
    let identities: Vec<_> = pool.owners().iter().map(|r| r.state).collect();
    println!(
        "{}",
        json!({"event":"service_configuration","pool_startup_ns":startup.elapsed().as_nanos(),"sdk_setup_ns":pool.owners().iter().map(|r|r.setup_time.as_nanos()).collect::<Vec<_>>(),"owners":4,"states":identities,"rust_workers":2,"slots":4,"work":32,"batch":8,
        "quantum":1,"scheme_service_workers":4,"actor_workers_per_batch":4,"probe_interval_us":500,"defaults_changed":false,"phase_observation":observation.full(),"observation_mode":format!("{observation:?}"),"trace_once":trace_once,"blocks":blocks,
        "split_layout_ab":split,"short_layout_only":comparison==Comparison::ShortLayout,"class_route_ab":comparison==Comparison::ClassRouting,"client_bulk_callbacks_per_owner":if split { Some(3) } else { None },"comparison":comparison.label()})
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    if observation.full() {
        os_calibration::run(&pool);
    }
    for mixed in [false, true] {
        let inputs = Arc::new(
            (0..if smoke { 256 } else { 8192 })
                .map(|id| corpus::order(id, mixed))
                .collect::<Vec<_>>(),
        );
        let warm = Arc::new(inputs[..256].to_vec());
        for arm in [false, true] {
            measurement::run(&pool, &runtime, &warm, 8, comparison, arm, observation);
            progress.send("service warmup validated").unwrap();
        }
        let mut samples = [Vec::new(), Vec::new()];
        for block in 0..blocks {
            for reverse in [false, true]
                .into_iter()
                .take(if trace_once { 1 } else { 2 })
            {
                for arm in if reverse { [1, 0] } else { [0, 1] } {
                    let sample = measurement::run(
                        &pool,
                        &runtime,
                        &inputs,
                        if smoke { 8 } else { 64 },
                        comparison,
                        arm == 1,
                        observation,
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
    cleanup(&mut pool);
}
fn cleanup(pool: &mut InstancePool) {
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
fn select_comparison(args: &[String]) -> Comparison {
    if args.iter().any(|s| s == "--progress-fair-ab") {
        Comparison::ProgressFair
    } else if args.iter().any(|s| s == "--retirement-refill-ab") {
        Comparison::RetirementRefill
    } else if args.iter().any(|s| s == "--selective-wake-ab") {
        Comparison::SelectiveWake
    } else if args.iter().any(|s| s == "--pool-protocol-ab") {
        Comparison::PoolProtocol
    } else if args.iter().any(|s| s == "--supply-admission-ab") {
        Comparison::SupplyAdmission
    } else if args.iter().any(|s| s == "--short-route-load-ab") {
        Comparison::ShortRouteLoad
    } else if args.iter().any(|s| s == "--observer-ab") {
        Comparison::ObserverCost
    } else if args.iter().any(|s| s == "--pooled-wake-ab") {
        Comparison::PooledWake
    } else if args.iter().any(|s| s == "--pooled-ab") {
        Comparison::Pooled
    } else if args.iter().any(|s| s == "--resident-ab") {
        Comparison::Resident
    } else if args.iter().any(|s| s == "--owner-qos-ab") {
        Comparison::OwnerQos
    } else if args.iter().any(|s| s == "--reselection-ab") {
        Comparison::Reselection
    } else if args.iter().any(|s| s == "--notification-ab") {
        Comparison::Notification
    } else if args.iter().any(|s| s == "--ingress-ab") {
        Comparison::Ingress
    } else if args.iter().any(|s| s == "--budget-ab") {
        Comparison::Budget
    } else if args.iter().any(|s| s == "--feed-ab") {
        Comparison::Feed
    } else if args.iter().any(|s| s == "--short-service-ab") {
        Comparison::ShortService
    } else if args.iter().any(|s| s == "--class-route-ab") {
        Comparison::ClassRouting
    } else if args.iter().any(|s| s == "--short-layout-only") {
        Comparison::ShortLayout
    } else if args.iter().any(|s| s == "--split-layout-ab") {
        Comparison::Combined
    } else {
        Comparison::Service
    }
}
fn repetitions(args: &[String]) -> usize {
    let requested: Vec<_> = args
        .iter()
        .filter_map(|s| s.strip_prefix("--blocks="))
        .collect();
    assert!(requested.len() <= 1, "duplicate block count");
    let compact = args.iter().any(|s| s == "--test" || s == "--trace-once");
    assert!(
        !compact || requested.is_empty(),
        "compact capture has a fixed block count"
    );
    let blocks = requested.first().map_or(if compact { 1 } else { 5 }, |s| {
        s.parse::<usize>().expect("positive block count")
    });
    assert!((1..=50).contains(&blocks), "block count must be 1..=50");
    blocks
}
fn await_trace_start(args: &[String]) {
    if args.iter().any(|s| s == "--await-trace-start") {
        use std::io::{Read, Write};
        println!(
            "{}",
            json!({"event":"trace_attach_ready","pid":std::process::id()})
        );
        std::io::stdout().flush().unwrap();
        let mut start = [0_u8];
        std::io::stdin().read_exact(&mut start).unwrap();
        assert_eq!(start, [b'R'], "trace start token");
    }
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert!(args.iter().all(|s| s.starts_with("--blocks=")
        || s == "--test"
        || s == "--bench"
        || s == "--split-layout-ab"
        || s == "--short-layout-only"
        || s == "--class-route-ab"
        || s == "--short-service-ab"
        || s == "--phase-observation"
        || s == "--probe-observation"
        || s == "--entry-observation"
        || s == "--state-observation"
        || s == "--observer-ab"
        || s == "--short-route-load-ab"
        || s == "--supply-admission-ab"
        || s == "--pool-protocol-ab"
        || s == "--selective-wake-ab"
        || s == "--retirement-refill-ab"
        || s == "--progress-fair-ab"
        || s == "--fixed-short-selective"
        || s == "--trace-once"
        || s == "--await-trace-start"
        || s == "--notification-ab"
        || s == "--reselection-ab"
        || s == "--owner-qos-ab"
        || s == "--resident-ab"
        || s == "--pooled-ab"
        || s == "--pooled-wake-ab"
        || s == "--ingress-ab"
        || s == "--budget-ab"
        || s == "--feed-ab"));
    let trace_once = args.iter().any(|s| s == "--trace-once");
    let observation = observation::select(&args);
    assert!(!trace_once || observation.enabled());
    assert!(
        !args.iter().any(|s| s == "--observer-ab")
            || matches!(observation, Mode::Off | Mode::Entry | Mode::State)
    );
    await_trace_start(&args);
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
    assert!(
        args.iter()
            .filter(|s| matches!(
                s.as_str(),
                "--notification-ab"
                    | "--observer-ab"
                    | "--short-route-load-ab"
                    | "--supply-admission-ab"
                    | "--pool-protocol-ab"
                    | "--selective-wake-ab"
                    | "--reselection-ab"
                    | "--owner-qos-ab"
                    | "--resident-ab"
                    | "--pooled-ab"
                    | "--pooled-wake-ab"
                    | "--ingress-ab"
                    | "--budget-ab"
                    | "--feed-ab"
                    | "--split-layout-ab"
                    | "--short-layout-only"
                    | "--class-route-ab"
                    | "--short-service-ab"
            ))
            .count()
            <= 1
    );
    let comparison = select_comparison(&args);
    run(
        &progress,
        args.iter().any(|s| s == "--test"),
        comparison,
        observation,
        trace_once,
        repetitions(&args),
    );
    drop(progress);
    watchdog.join().unwrap();
}
