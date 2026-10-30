//! One VM, one outstanding request: component costs, not production capacity.
#[path = "support/orders.rs"]
mod corpus;
#[path = "single_vm_service/request.rs"]
mod request;
#[path = "single_vm_service/scaling.rs"]
mod scaling;
#[path = "single_vm_service/watchdog.rs"]
mod watchdog;
use gerbil_scheme::instances::{InstanceConfig, InstanceDispatcher, InstancePool, IsolatedProgram};
use request::{Reply, Request};
use serde_json::json;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use watchdog::{Event, Progress, Watchdog};
const GATE: Duration = Duration::from_secs(5);
#[derive(Clone, Copy, Debug)]
enum Arm {
    NativeCycle,
    CallerService,
    WorkerService,
}
impl Arm {
    fn label(self) -> &'static str {
        match self {
            Self::NativeCycle => "owner_native_service_cycle",
            Self::CallerService => "caller_driven_service",
            Self::WorkerService => "tokio_worker_service",
        }
    }
}
struct Sample {
    latency: Vec<u128>,
    entry: Vec<u128>,
    construct: Vec<u128>,
    service: Vec<u128>,
    projection: Vec<u128>,
    recovery: Vec<u128>,
    oracle_ns: u128,
    rows: usize,
    started: Instant,
    elapsed_ns: u128,
}
impl Sample {
    fn new() -> Self {
        Self {
            latency: vec![],
            entry: vec![],
            construct: vec![],
            service: vec![],
            projection: vec![],
            recovery: vec![],
            oracle_ns: 0,
            rows: 0,
            started: Instant::now(),
            elapsed_ns: 0,
        }
    }
    fn record(&mut self, input: &[Vec<i64>], short: bool, started: Instant, reply: &Reply) {
        let consumed = Instant::now();
        self.latency
            .push(consumed.duration_since(started).as_nanos());
        self.entry
            .push(reply.entered.duration_since(started).as_nanos());
        self.construct
            .push(reply.constructed.duration_since(reply.entered).as_nanos());
        self.service.push(
            reply
                .projecting
                .duration_since(reply.constructed)
                .as_nanos(),
        );
        self.projection
            .push(reply.projected.duration_since(reply.projecting).as_nanos());
        self.recovery
            .push(consumed.duration_since(reply.projected).as_nanos());
        let oracle = Instant::now();
        assert_eq!(input.len(), reply.values.len());
        for (input, output) in input.iter().zip(&reply.values) {
            corpus::verify(input, output);
        }
        if short {
            assert_eq!(&reply.values[0][3..5], &[1, 0]);
        } else {
            corpus::peers(&reply.values);
        }
        self.oracle_ns += oracle.elapsed().as_nanos();
        self.rows += input.len();
    }
    fn finish(&mut self) {
        self.elapsed_ns = self.started.elapsed().as_nanos();
    }
    fn emit(mut self, arm: Arm, profile: &str, block: usize, warmup: bool) {
        println!("{}", self.report(arm, profile, block, warmup));
    }
    fn report(&mut self, arm: Arm, profile: &str, block: usize, warmup: bool) -> serde_json::Value {
        let elapsed_ns = self.elapsed_ns;
        let messages = self.latency.len();
        json!({"event":"single_vm_sample","arm":arm.label(),"profile":profile,
            "block":block,"warmup":warmup,"messages":messages,"completed_messages":messages,
            "rejected_messages":0,"native_request_errors":0,"order_rows":self.rows,
            "elapsed_ns":elapsed_ns,"messages_per_second":rate(messages, elapsed_ns),
            "order_rows_per_second":rate(self.rows, elapsed_ns),
            "oracle_ns":self.oracle_ns,"latency_ns":quantiles(&mut self.latency),
            "client_to_constructor_ns":quantiles(&mut self.entry),
            "native_construction_ns":quantiles(&mut self.construct),
            "constructed_to_projection_ns":quantiles(&mut self.service),
            "projection_ns":quantiles(&mut self.projection),
            "projected_to_consumed_ns":quantiles(&mut self.recovery)})
    }
}
fn quantiles(values: &mut [u128]) -> serde_json::Value {
    let p99 = corpus::percentile(values, 99);
    let at = |n: usize, d: usize| values[(values.len() * n).div_ceil(d).saturating_sub(1)];
    json!({"p50":at(50,100),"p95":at(95,100),"p99":p99,"p999":at(999,1000),"max":values.last()})
}
fn rate(count: usize, elapsed_ns: u128) -> f64 {
    let count = f64::from(u32::try_from(count).expect("bounded sample count"));
    let elapsed = Duration::from_nanos(u64::try_from(elapsed_ns).expect("bounded sample time"));
    count / elapsed.as_secs_f64()
}
#[derive(Clone, Copy, Debug)]
struct Partition {
    owner: usize,
    owners: usize,
    messages: usize,
}

async fn service_sample(
    dispatch: InstanceDispatcher,
    inputs: Arc<Vec<Vec<Vec<i64>>>>,
    short: bool,
    progress: Progress,
    messages: usize,
) -> Sample {
    service_partition(
        dispatch,
        inputs,
        short,
        progress,
        Partition {
            owner: 0,
            owners: 1,
            messages,
        },
    )
    .await
}
async fn service_partition(
    dispatch: InstanceDispatcher,
    inputs: Arc<Vec<Vec<Vec<i64>>>>,
    short: bool,
    progress: Progress,
    partition: Partition,
) -> Sample {
    let mut sample = Sample::new();
    for input in inputs
        .iter()
        .cycle()
        .skip(partition.owner)
        .step_by(partition.owners)
        .take(partition.messages)
    {
        let started = Instant::now();
        let reply = dispatch
            .submit_scheme_service_backpressured_on(
                partition.owner,
                Request::new(input.clone(), short),
                GATE,
            )
            .await
            .unwrap()
            .wait_completed(GATE)
            .await
            .unwrap()
            .0;
        sample.record(input, short, started, &reply);
        progress.record(Event::ServiceCompleted);
    }
    sample.finish();
    sample
}
fn native_sample(
    dispatch: &InstanceDispatcher,
    inputs: &[Vec<Vec<i64>>],
    short: bool,
    progress: &Progress,
) -> Sample {
    let mut sample = Sample::new();
    for input in inputs {
        let started = Instant::now();
        let request = Request::new(input.clone(), short);
        let reply = dispatch
            .submit_native_on(0, move |session| request.native(session))
            .unwrap()
            .wait(GATE)
            .unwrap();
        sample.record(input, short, started, &reply);
        progress.record(Event::NativeCompleted);
    }
    sample.finish();
    sample
}
fn run(progress: &Progress, smoke: bool, million: bool, scaling_owners: Option<usize>) {
    let owners = scaling_owners.unwrap_or(1);
    let image = std::env::var("GERBIL_SERVICE_BENCH_IMAGE")
        .unwrap_or_else(|_| gerbil_scheme_qualification::owner_instance_image().to_owned());
    // SAFETY: Cargo builds this qualification image and its rooted service ABI.
    let program = unsafe {
        IsolatedProgram::from_image(&image, c"gerbil_owner_orders_call")
            .unwrap()
            .with_exports(&[c"gerbil_owner_service_reference"])
            .with_scheme_service(c"gerbil_owner_service_call")
    };
    let startup = Instant::now();
    let mut pool = InstancePool::start_observed(
        &program,
        InstanceConfig {
            owners,
            queue_capacity: 128,
            maximum_request_bytes: 16 * 1024 * 1024,
            maximum_reply_bytes: 16 * 1024 * 1024,
            startup_timeout: GATE,
        },
        |_| progress.record(Event::Initialized),
    )
    .unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    let caller = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let worker = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(owners)
        .enable_all()
        .build()
        .unwrap();
    let corpus_messages = if smoke { 64 } else { 2048 };
    let messages = if million { 1_000_000 } else { corpus_messages };
    println!(
        "{}",
        json!({"event":if scaling_owners.is_some() {"fixed_owner_scaling_configuration"} else {"single_vm_configuration"},"owners":owners,"outstanding":owners,
        "tokio_workers":owners,"messages_per_sample":messages,"startup_ns":startup.elapsed().as_nanos(),
        "service_completion":"Snapshot","defaults_changed":false,"deadline_ms":GATE.as_millis(),
        "profiles":if million { vec!["short_cpu"] } else { vec!["short_cpu","bulk_cpu","bulk_mixed"] },
        "million_baseline":million,"corpus_messages":corpus_messages,
        "fixed_owner_scaling":scaling_owners.is_some(),"outstanding_per_owner":1,
        "arrival_mode":if scaling_owners.is_some() {"closed_loop_one_outstanding_per_owner"} else {"closed_loop_one_outstanding"},
        "exclusive_physical_core_affinity":false,
        "explicit_image_override":std::env::var_os("GERBIL_SERVICE_BENCH_IMAGE").is_some(),
        "measured_arms":if million { vec![Arm::WorkerService.label()] } else { vec![Arm::NativeCycle.label(),Arm::CallerService.label(),Arm::WorkerService.label()] },"bulk_orders":8,"bulk_lines":48,
        "short_orders":1,"short_lines":4,"yield_every":1,"service_workers":4,"actors_per_batch":4,
        "throughput_unit":"service_messages_per_second_including_oracle",
        "comparison_scope":if scaling_owners.is_some() {"same_short_CPU_workload_fixed_owner_scaling_not_open_arrival"} else {"one_VM_component_costs_not_capacity_or_open_arrival_p99"},
        "native_scope":"host_owner_queue_plus_full_Scheme_service_cycle_not_pure_Scheme_upper_bound",
        "caller_scope":"public_async_API_on_current_thread_Tokio",
        "worker_scope":"same_public_API_on_one_Tokio_worker",
        "phase_percentiles_are_not_additive":true,
        "progress_observer":"atomic_verified_epoch_no_per_request_notification"})
    );
    exception_controls(&dispatch, &caller);
    progress.record(Event::ControlsCompleted);
    if let Some(owners) = scaling_owners {
        scaling::run(&dispatch, &worker, progress, owners);
    } else {
        profiles(
            &dispatch,
            &caller,
            &worker,
            progress,
            smoke,
            corpus_messages,
            million,
        );
    }
    cleanup(&mut pool, progress);
}
fn input_corpus(messages: usize, short: bool, mixed: bool) -> Arc<Vec<Vec<Vec<i64>>>> {
    Arc::new(
        (0..messages)
            .map(|n| {
                (0..if short { 1 } else { 8 })
                    .map(|row| {
                        let mut input = corpus::order(n * 8 + row, mixed);
                        if short {
                            input.truncate(14);
                        }
                        input
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    )
}

fn profiles(
    dispatch: &InstanceDispatcher,
    caller: &tokio::runtime::Runtime,
    worker: &tokio::runtime::Runtime,
    progress: &Progress,
    smoke: bool,
    messages: usize,
    million: bool,
) {
    for (profile, short, mixed) in [
        ("short_cpu", true, false),
        ("bulk_cpu", false, false),
        ("bulk_mixed", false, true),
    ] {
        if million && !short {
            continue;
        }
        let inputs = input_corpus(messages, short, mixed);
        for warmup in [true, false] {
            let data = if warmup {
                Arc::new(inputs[..32].to_vec())
            } else {
                inputs.clone()
            };
            for block in 0..if warmup || smoke || million { 1 } else { 3 } {
                let arms: &[Arm] = if million {
                    &[Arm::WorkerService]
                } else {
                    &[
                        Arm::NativeCycle,
                        Arm::CallerService,
                        Arm::WorkerService,
                        Arm::WorkerService,
                        Arm::CallerService,
                        Arm::NativeCycle,
                    ]
                };
                for &arm in arms {
                    let count = if million && !warmup {
                        1_000_000
                    } else {
                        data.len()
                    };
                    // One request is consumed before the next is submitted; no
                    // Rust service ticket/permit may overlap the native reference.
                    caller.block_on(async {
                        let start = Instant::now();
                        while dispatch
                            .owner_capacity()
                            .iter()
                            .any(|c| c.available != c.limit || c.work_available != c.work_limit)
                        {
                            assert!(start.elapsed() < GATE, "credits did not retire");
                            tokio::task::yield_now().await;
                        }
                    });
                    let before = snapshot(dispatch);
                    let sample = match arm {
                        Arm::NativeCycle => native_sample(dispatch, &data, short, progress),
                        Arm::CallerService => caller.block_on(service_sample(
                            dispatch.clone(),
                            data.clone(),
                            short,
                            progress.clone(),
                            count,
                        )),
                        Arm::WorkerService => worker
                            .block_on(worker.spawn(service_sample(
                                dispatch.clone(),
                                data.clone(),
                                short,
                                progress.clone(),
                                count,
                            )))
                            .unwrap(),
                    };
                    assert_eq!(
                        sample.latency.len(),
                        count,
                        "all requested messages must complete"
                    );
                    let after = snapshot(dispatch);
                    assert_eq!(after.0[3].to_bits(), 0.0_f64.to_bits());
                    let counters = before
                        .1
                        .zip(after.1)
                        .map(|(a, b)| std::array::from_fn::<_, 6, _>(|i| b[i] - a[i]));
                    println!(
                        "{}",
                        json!({"event":"single_vm_boundary_counters","arm":arm.label(),
                        "profile":profile,"block":block,"warmup":warmup,
                        "gc_collections":after.0[0]-before.0[0],"heap_bytes_before":before.0[1],
                        "heap_bytes_after":after.0[1],"roots_after":after.0[3],
                        "diagnostic_delta":counters,
                        "diagnostic_order":["gc_user_s","gc_system_s","gc_wall_s","owner_thread_cpu_s","roots","gc_count"],
                        "read_scope":"sample_boundaries_outside_timing"})
                    );
                    sample.emit(arm, profile, block, warmup);
                }
            }
        }
    }
}
fn cleanup(pool: &mut InstancePool, progress: &Progress) {
    let dispatch = pool.dispatcher();
    let owners = pool.owners().len();
    for owner in 0..owners {
        dispatch
            .submit_native_on(owner, |session| {
                session.collect()?;
                assert_eq!(session.usage()?[3].to_bits(), 0.0_f64.to_bits());
                Ok(())
            })
            .unwrap()
            .wait(GATE)
            .unwrap();
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
        json!({"event":if owners==1 {"single_vm_cleanup"} else {"fixed_owner_scaling_cleanup"},"joined":receipt.joined,"roots":0,"files_removed":true})
    );
    progress.record(Event::Retired);
}
fn snapshot(dispatch: &InstanceDispatcher) -> ([f64; 4], Option<[f64; 6]>) {
    snapshot_on(dispatch, 0)
}
fn snapshot_on(dispatch: &InstanceDispatcher, owner: usize) -> ([f64; 4], Option<[f64; 6]>) {
    dispatch
        .submit_native_on(owner, |session| {
            Ok((session.usage()?, session.diagnostics()))
        })
        .unwrap()
        .wait(GATE)
        .unwrap()
}
fn exception_controls(dispatch: &InstanceDispatcher, caller: &tokio::runtime::Runtime) {
    // A malformed native order must fail in both paths. Later successful
    // warmups and final zero roots are required, not just an error return.
    let bad = Request::new(vec![vec![0, 0, 0]], true);
    let failure = dispatch
        .submit_native_on(0, move |session| bad.native(session))
        .unwrap()
        .wait(GATE);
    match failure {
        Err(
            gerbil_scheme::instances::InstanceError::Native { .. }
            | gerbil_scheme::instances::InstanceError::Scheme(_),
        ) => {}
        _ => panic!("native reference did not propagate the Scheme exception"),
    }
    let failure = caller.block_on(async {
        dispatch
            .submit_scheme_service_backpressured_on(
                0,
                Request::new(vec![vec![0, 0, 0]], true),
                GATE,
            )
            .await
            .unwrap()
            .wait_completed(GATE)
            .await
    });
    assert!(matches!(
        failure,
        Err(gerbil_scheme::instances::InstanceError::Scheme(_))
    ));
    println!(
        "{}",
        json!({"event":"single_vm_controls","native_exception":true,
        "service_exception":true,"subsequent_reuse_required":true})
    );
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert!(
        args.iter().all(|a| a == "--test"
            || a == "--bench"
            || a == "--million-short-cpu"
            || a == "--fixed-owner-scaling=1"
            || a == "--fixed-owner-scaling=12"),
        "supported arguments: --test, --bench, --million-short-cpu, --fixed-owner-scaling=1|12"
    );
    let scaling: Vec<_> = args
        .iter()
        .filter_map(|a| a.strip_prefix("--fixed-owner-scaling="))
        .collect();
    assert!(
        scaling.len() <= 1,
        "one fixed-owner scaling target per process"
    );
    let scaling_owners = scaling.first().map(|s| s.parse::<usize>().unwrap());
    assert!(
        !(scaling_owners.is_some() && args.iter().any(|a| a == "--million-short-cpu")),
        "select one million-message profile"
    );
    let million = scaling_owners.is_some() || args.iter().any(|a| a == "--million-short-cpu");
    let smoke = args.iter().any(|a| a == "--test");
    assert!(
        !(million && smoke),
        "million baseline cannot be labeled as smoke"
    );
    let (watchdog, progress) = Watchdog::start(GATE);
    run(&progress, smoke, million, scaling_owners);
    watchdog.finish();
}
