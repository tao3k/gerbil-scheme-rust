//! Sustained routed service-message calibration, separate from pipeline A/B.
use super::{
    GATE, arrival, corpus, measurement,
    observation::Mode,
    workload::{self, ShortMode},
};
use gerbil_scheme::instances::{
    InstanceArrival, InstancePool, SchemeIngressConfig, SchemeRouteClass, SchemeServiceIngress,
    SchemeServiceRouter,
};
use serde_json::json;
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
pub(super) fn is_argument(value: &str) -> bool {
    value == "--capacity"
        || value == "--capacity-mixed"
        || value == "--capacity-ingress-ab"
        || value == "--capacity-owner-claim-ab"
        || [
            "--capacity-count=",
            "--capacity-clients=",
            "--capacity-owners=",
            "--capacity-rust-workers=",
            "--capacity-rate=",
            "--capacity-warmup-seconds=",
            "--capacity-seconds=",
        ]
        .iter()
        .any(|prefix| value.starts_with(prefix))
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ComparisonMode {
    Single,
    Ingress,
    OwnerClaim,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ShortBinding {
    Routed,
    RunningOwner,
}
#[derive(Clone, Copy)]
pub(super) struct Config {
    pub(super) owners: usize,
    pub(super) rust_workers: usize,
    pub(super) class_ingress: bool,
    comparison: ComparisonMode,
    binding: ShortBinding,
    count: usize,
    clients: usize,
    rate: usize,
    seconds: u64,
    warmup: u64,
    mixed: bool,
}
impl Config {
    pub(super) fn from_args(args: &[String]) -> Option<Self> {
        let selected = args.iter().any(|s| s == "--capacity");
        assert!(
            selected || !args.iter().any(|s| s.starts_with("--capacity-")),
            "capacity options require --capacity"
        );
        if !selected {
            return None;
        }
        let value = |key: &str, default: usize| {
            let xs: Vec<_> = args.iter().filter_map(|s| s.strip_prefix(key)).collect();
            assert!(xs.len() <= 1, "duplicate capacity option");
            xs.first()
                .map_or(default, |s| s.parse().expect("unsigned capacity option"))
        };
        let mut out = Self {
            owners: value("--capacity-owners=", 4),
            rust_workers: value("--capacity-rust-workers=", 2),
            binding: ShortBinding::Routed,
            class_ingress: args.iter().any(|s| s == "--capacity-owner-claim-ab"),
            comparison: if args.iter().any(|s| s == "--capacity-owner-claim-ab") {
                ComparisonMode::OwnerClaim
            } else if args.iter().any(|s| s == "--capacity-ingress-ab") {
                ComparisonMode::Ingress
            } else {
                ComparisonMode::Single
            },
            count: value("--capacity-count=", 1_000_000),
            clients: value("--capacity-clients=", 50),
            rate: value("--capacity-rate=", 0),
            seconds: value("--capacity-seconds=", 30).try_into().unwrap(),
            warmup: value("--capacity-warmup-seconds=", 5).try_into().unwrap(),
            mixed: args.iter().any(|s| s == "--capacity-mixed"),
        };
        if out.rate == 0 {
            out.class_ingress = false;
        }
        assert!((1..=1_000_000).contains(&out.count) && (1..=4096).contains(&out.clients));
        assert!((1..=64).contains(&out.owners) && (1..=64).contains(&out.rust_workers));
        assert!(out.clients >= 2);
        assert!(
            out.comparison != ComparisonMode::Ingress || out.rate > 0,
            "ingress A/B requires planned arrivals"
        );
        assert!(out.seconds <= 120 && out.warmup <= 30 && out.rate <= 1_000_000);
        assert!(
            out.rate
                .saturating_mul(usize::try_from(out.seconds).unwrap())
                <= 10_000_000,
            "measurement sample bound"
        );
        if out.rate > 0 {
            assert!(
                out.count
                    .max(out.rate * usize::try_from(out.seconds).unwrap())
                    <= out.rate * 120,
                "planned capacity load must fit 120 seconds; lower count for low rates"
            );
        }
        Some(out)
    }
}
#[derive(Clone, Copy)]
struct Input {
    id: usize,
    planned: Instant,
    issued: Instant,
}
struct Record {
    total: u64,
    queue: u64,
    generator: u64,
    pre_entry: u64,
    native: u64,
    delivery: u64,
    owner: usize,
    short: bool,
    status: u8,
}
fn ns(d: Duration) -> u64 {
    d.as_nanos().try_into().expect("bounded elapsed")
}
async fn request(router: &SchemeServiceRouter, input: Input, mixed: bool) -> Record {
    let short = input.id % 17 == 16;
    let started = Instant::now();
    let mut items = if short {
        vec![corpus::order(input.id, false)]
    } else {
        (0..8)
            .map(|n| corpus::order(input.id * 8 + n, mixed))
            .collect()
    };
    if short {
        items[0].truncate(14);
    }
    let mode = if short {
        ShortMode::Dedicated(Mode::Off)
    } else {
        ShortMode::Batch
    };
    let deadline = input.planned + GATE;
    let remaining = || deadline.saturating_duration_since(Instant::now());
    let message = workload::orders(items.clone(), mode, None);
    let placed = if short {
        router.submit_short(message, remaining()).await
    } else {
        router.submit_batch(message, remaining()).await
    };
    let result = match placed {
        Ok(selected) => selected.ticket.wait_completed(remaining()).await,
        Err(error) => Err(error),
    };
    let observed = Instant::now();
    match result {
        Ok((reply, completed)) => {
            assert_eq!(reply.values.len(), items.len());
            for (input, output) in items.iter().zip(&reply.values) {
                corpus::verify(input, output);
            }
            if !short {
                corpus::peers(&reply.values);
            }
            let record = Record {
                total: ns(observed.duration_since(input.planned)),
                queue: ns(started.duration_since(input.issued)),
                generator: ns(input.issued.duration_since(input.planned)),
                pre_entry: ns(reply.entered.duration_since(started)),
                native: ns(completed.duration_since(reply.entered)),
                delivery: ns(observed.duration_since(completed)),
                owner: reply.owner,
                short,
                status: 0,
            };
            assert_eq!(
                record.total,
                record.generator
                    + record.queue
                    + record.pre_entry
                    + record.native
                    + record.delivery,
                "same-request elapsed conservation"
            );
            record
        }
        Err(error) => {
            eprintln!("CAPACITY-REQUEST-ERROR id={} error={error:?}", input.id);
            Record {
                total: ns(observed.duration_since(input.planned)),
                queue: ns(started.duration_since(input.issued)),
                generator: ns(input.issued.duration_since(input.planned)),
                pre_entry: 0,
                native: 0,
                delivery: 0,
                owner: usize::MAX,
                short,
                status: 2,
            }
        }
    }
}
fn signal(
    progress: &mpsc::Sender<&'static str>,
    stamp: &AtomicU64,
    record: &Record,
    released: Instant,
) {
    if record.status != 0 {
        return;
    }
    let now = ns(released.elapsed());
    let previous = stamp.load(Ordering::Relaxed);
    if (previous == 0 || now.saturating_sub(previous) >= 250_000_000)
        && stamp
            .compare_exchange(previous, now.max(1), Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    {
        progress.send("capacity native result verified").unwrap();
    }
}
async fn closed(
    router: SchemeServiceRouter,
    config: Config,
    progress: mpsc::Sender<&'static str>,
    warm: bool,
) -> (Vec<Record>, Duration) {
    let released = Instant::now();
    let next = Arc::new(AtomicUsize::new(0));
    let done = Arc::new(AtomicU64::new(0));
    let mut workers = Vec::new();
    let seconds = Duration::from_secs(if warm { config.warmup } else { config.seconds });
    let minimum = if warm { 0 } else { config.count };
    for _ in 0..config.clients {
        let (router, next, done, progress) = (
            router.clone(),
            Arc::clone(&next),
            Arc::clone(&done),
            progress.clone(),
        );
        workers.push(tokio::spawn(async move {
            let mut rows = Vec::new();
            loop {
                let id = next.fetch_add(1, Ordering::Relaxed);
                if id >= 10_000_000 || (id >= minimum && released.elapsed() >= seconds) {
                    break;
                }
                let now = Instant::now();
                let row = request(
                    &router,
                    Input {
                        id,
                        planned: now,
                        issued: now,
                    },
                    config.mixed,
                )
                .await;
                signal(&progress, &done, &row, released);
                if warm {
                    assert_eq!(row.status, 0, "capacity warmup failed");
                } else {
                    rows.push(row);
                }
            }
            rows
        }));
    }
    let mut records = Vec::new();
    for task in workers {
        records.extend(task.await.unwrap());
    }
    (records, released.elapsed())
}
async fn offered(
    router: SchemeServiceRouter,
    config: Config,
    progress: mpsc::Sender<&'static str>,
) -> (Vec<Record>, Duration, usize, Duration) {
    let released = Instant::now();
    let count = config
        .count
        .max(config.rate * usize::try_from(config.seconds).unwrap());
    if config.class_ingress {
        return class_offered(router, config, progress).await;
    }
    let (send, receive) = tokio::sync::mpsc::channel::<Input>(128);
    let receive = Arc::new(tokio::sync::Mutex::new(receive));
    let done = Arc::new(AtomicU64::new(0));
    let mut workers = Vec::new();
    for _ in 0..config.clients {
        let (router, receive, done, progress) = (
            router.clone(),
            Arc::clone(&receive),
            Arc::clone(&done),
            progress.clone(),
        );
        workers.push(tokio::spawn(async move {
            let mut rows = Vec::new();
            loop {
                let input = { receive.lock().await.recv().await };
                let Some(input) = input else {
                    break;
                };
                let row = request(&router, input, config.mixed).await;
                signal(&progress, &done, &row, released);
                rows.push(row);
            }
            rows
        }));
    }
    let (mut records, offered_elapsed) =
        emit(count, config.rate, Sink::Shared(send), released).await;
    for task in workers {
        records.extend(task.await.unwrap());
    }
    assert_eq!(
        records.len(),
        count,
        "every planned arrival must have an outcome"
    );
    (records, released.elapsed(), count, offered_elapsed)
}
// A future deadline arms the kernel clock once. Already-due arrivals retain
// their original plan and are emitted in queue-sized turns, never retimed.
async fn emit(count: usize, rate: usize, send: Sink, released: Instant) -> (Vec<Record>, Duration) {
    let mut records = Vec::new();
    let mut clock = arrival::Clock::new(true);
    let mut id = 0;
    let planned = |id: usize| {
        released
            + Duration::from_nanos(
                u64::try_from((id as u128) * 1_000_000_000 / rate as u128).unwrap(),
            )
    };
    while id < count {
        let next = planned(id);
        if next > Instant::now() {
            let timing = InstanceArrival::new(next, None).unwrap();
            let _ = clock.issue(timing, next, released).await;
        }
        for _ in 0..128 {
            if id == count {
                break;
            }
            let planned = planned(id);
            let issued = Instant::now();
            if planned > issued {
                break;
            }
            if let Some(input) = send.offer(Input {
                id,
                planned,
                issued,
            }) {
                records.push(Record {
                    total: ns(Instant::now().duration_since(input.planned)),
                    queue: 0,
                    generator: ns(input.issued.duration_since(input.planned)),
                    pre_entry: 0,
                    native: 0,
                    delivery: 0,
                    owner: usize::MAX,
                    short: input.id % 17 == 16,
                    status: 1,
                });
            }
            id += 1;
        }
        if id < count && planned(id) <= Instant::now() {
            tokio::task::yield_now().await;
        }
    }
    send.close();
    (records, released.elapsed())
}
fn percentile(rows: &[Record], short: bool, field: fn(&Record) -> u64, p: usize) -> Option<u128> {
    let mut xs: Vec<_> = rows
        .iter()
        .filter(|r| r.short == short && r.status == 0)
        .map(field)
        .collect();
    super::throughput::rank(&mut xs, p).map(u128::from)
}
pub(super) fn run(
    pool: &InstancePool,
    runtime: &tokio::runtime::Runtime,
    progress: &mpsc::Sender<&'static str>,
    comparison: measurement::Comparison,
    config: Config,
) {
    let (router, _settings) = measurement::configure_service(pool, comparison, true);
    let router = if config.binding == ShortBinding::RunningOwner {
        router.with_owner_claimed_shorts().unwrap()
    } else {
        router
    };
    println!(
        "{}",
        json!({"event":"capacity_configuration","owner_claim":config.binding==ShortBinding::RunningOwner,"api":"routed-typed-service-messages","qualifies_pool_pipeline_candidate":false,"count_minimum":config.count,"minimum_seconds":config.seconds,"sample_hard_bound":10_000_000,"clients":config.clients,"offered_rate":config.rate,"queue_capacity":if config.rate>0{Some(128)}else{None},"load_mode":if config.rate==0{"closed-loop-saturation"}else{"open-loop-planned-arrivals"},"bulk_messages_per_short":16,"bulk_items_per_message":8,"bulk_lines_per_item":48,"short_lines":4,"mixed":config.mixed,"warmup_seconds":config.warmup,"owners":config.owners,"rust_workers":config.rust_workers,"class_ingress":config.class_ingress,"native_progress_watchdog_seconds":5,"defaults_changed":false})
    );
    if config.warmup > 0 {
        runtime.block_on(closed(router.clone(), config, progress.clone(), true));
    }
    measurement::idle(pool);
    let (rows, elapsed, offered_count, offered_elapsed) = if config.rate == 0 {
        let (rows, time) =
            runtime.block_on(closed(router.clone(), config, progress.clone(), false));
        let count = rows.len();
        (rows, time, count, time)
    } else {
        runtime.block_on(offered(router.clone(), config, progress.clone()))
    };
    progress
        .send("capacity measurement finished; outcomes collected")
        .unwrap();
    measurement::idle(pool);
    router.close();
    assert!(
        router
            .loads()
            .iter()
            .all(|r| r.short_requests == 0 && r.batch_requests == 0 && r.batch_work == 0)
    );
    let successful = rows.iter().filter(|r| r.status == 0).count();
    let rejected = rows.iter().filter(|r| r.status == 1).count();
    let failed = rows.len() - successful - rejected;
    let short = rows.iter().filter(|r| r.status == 0 && r.short).count();
    let bulk = successful - short;
    let owners: Vec<_> = (0..config.owners)
        .map(|owner| {
            rows.iter()
                .filter(|r| r.status == 0 && r.owner == owner)
                .count()
        })
        .collect();
    let classes:Vec<_>=[false,true].into_iter().map(|short|json!({"class":if short{"short"}else{"bulk"},"samples":rows.iter().filter(|r|r.short==short && r.status==0).count(),"planned_to_consumed_p50_ns":percentile(&rows,short,|r|r.total,5000),"planned_to_consumed_p95_ns":percentile(&rows,short,|r|r.total,9500),"planned_to_consumed_p99_ns":percentile(&rows,short,|r|r.total,9900),"planned_to_consumed_p999_ns":percentile(&rows,short,|r|r.total,9990),"generator_lag_p99_ns":percentile(&rows,short,|r|r.generator,9900),"ingress_queue_p99_ns":percentile(&rows,short,|r|r.queue,9900),"preparation_admission_owner_wait_p99_ns":percentile(&rows,short,|r|r.pre_entry,9900),"native_p99_ns":percentile(&rows,short,|r|r.native,9900),"delivery_p99_ns":percentile(&rows,short,|r|r.delivery,9900)})).collect();
    let number = |value: usize| f64::from(u32::try_from(value).expect("bounded benchmark count"));
    println!(
        "{}",
        json!({"event":"capacity_result","owner_claim":config.binding==ShortBinding::RunningOwner,"class_ingress":config.class_ingress,"measurement_valid":config.rate>0 || (elapsed>=Duration::from_secs(config.seconds) && rows.len()>=config.count),"performance_accepted":false,"completed_messages":successful,"bulk_messages":bulk,"short_messages":short,"bulk_items":bulk*8,"completed_input_items":bulk*8+short,"offered_messages":offered_count,"rejected_before_admission":rejected,"failed_messages":failed,"elapsed_ns":elapsed.as_nanos(),"offered_elapsed_ns":offered_elapsed.as_nanos(),"completed_messages_per_second":number(successful)/elapsed.as_secs_f64(),"emitted_messages_per_second":number(offered_count)/offered_elapsed.as_secs_f64(),"bulk_items_per_second":number(bulk*8)/elapsed.as_secs_f64(),"owner_completed_messages":owners,"owner_bulk_messages":(0..config.owners).map(|o|rows.iter().filter(|r|r.status==0 && r.owner==o && !r.short).count()).collect::<Vec<_>>(),"owner_short_messages":(0..config.owners).map(|o|rows.iter().filter(|r|r.status==0 && r.owner==o && r.short).count()).collect::<Vec<_>>(),"latency":classes,"latency_population":"successful messages; all rejected and failed arrivals remain in outcome accounting","benchmark_record_bytes":rows.capacity()*std::mem::size_of::<Record>(),"latency_includes_payload_preparation":true,"throughput_includes_result_oracle":true,"closed_loop_is_not_open_loop_slo_evidence":config.rate==0,"pipeline_performance_acceptance":false})
    );
    assert_eq!(successful + rejected + failed, offered_count);
}

enum Sink {
    Shared(tokio::sync::mpsc::Sender<Input>),
    Classes(SchemeServiceIngress<Input>),
}
impl Sink {
    fn offer(&self, input: Input) -> Option<Input> {
        match self {
            Self::Shared(send) => send
                .try_send(input)
                .err()
                .map(tokio::sync::mpsc::error::TrySendError::into_inner),
            Self::Classes(ingress) => {
                let class = if input.id % 17 == 16 {
                    SchemeRouteClass::Short
                } else {
                    SchemeRouteClass::Batch
                };
                match ingress.try_enqueue(class, input) {
                    Ok(admission) => admission.reclaimed,
                    Err(rejected) => Some(rejected.value),
                }
            }
        }
    }
    fn close(self) {
        if let Self::Classes(ingress) = self {
            ingress.close();
        }
    }
}
async fn class_offered(
    router: SchemeServiceRouter,
    config: Config,
    progress: mpsc::Sender<&'static str>,
) -> (Vec<Record>, Duration, usize, Duration) {
    let released = Instant::now();
    let count = config
        .count
        .max(config.rate * usize::try_from(config.seconds).unwrap());
    let ingress = SchemeServiceIngress::new(SchemeIngressConfig {
        queue_capacity: 128,
        client_capacity: config.clients,
        queue_reserved: [8, 1],
        client_reserved: [config.owners.min(config.clients - 1), 1],
    })
    .unwrap();
    let done = Arc::new(AtomicU64::new(0));
    let mut workers = Vec::new();
    for _ in 0..config.clients {
        let (ingress, router, progress, done) = (
            ingress.clone(),
            router.clone(),
            progress.clone(),
            Arc::clone(&done),
        );
        workers.push(tokio::spawn(async move {
            let mut rows = Vec::new();
            while let Some(mut claim) = ingress.recv().await {
                let input = claim.take().unwrap();
                let row = request(&router, input, config.mixed).await;
                signal(&progress, &done, &row, released);
                rows.push(row);
                drop(claim); // Client charge returns only after result consumption and verification.
            }
            rows
        }));
    }
    let (mut rows, offered_elapsed) =
        emit(count, config.rate, Sink::Classes(ingress.clone()), released).await;
    for task in workers {
        rows.extend(task.await.unwrap());
    }
    assert_eq!(rows.len(), count);
    assert_eq!(ingress.snapshot().queued, [0, 0]);
    assert_eq!(ingress.snapshot().clients, [0, 0]);
    (rows, released.elapsed(), count, offered_elapsed)
}

pub(super) fn run_profiles(
    pool: &InstancePool,
    runtime: &tokio::runtime::Runtime,
    progress: &mpsc::Sender<&'static str>,
    comparison: measurement::Comparison,
    mut config: Config,
) {
    let identities: Vec<_> = pool.owners().iter().map(|r| r.state).collect();
    run(pool, runtime, progress, comparison, config);
    if config.comparison == ComparisonMode::Ingress {
        config.class_ingress = true;
        run(pool, runtime, progress, comparison, config);
    }
    if config.comparison == ComparisonMode::OwnerClaim {
        config.binding = ShortBinding::RunningOwner;
        run(pool, runtime, progress, comparison, config);
    }

    assert_eq!(
        identities,
        pool.owners().iter().map(|r| r.state).collect::<Vec<_>>()
    );
}
