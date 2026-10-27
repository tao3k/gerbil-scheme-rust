//! Sustained routed service-message calibration, separate from pipeline A/B.
#[path = "capacity_continuous.rs"]
mod continuous;
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
        || value == "--capacity-short-only"
        || value == "--capacity-continuous"
        || value == "--capacity-actor-supply-ab"
        || value == "--capacity-tagged-input-ab"
        || value == "--capacity-native-exchange-ab"
        || value == "--capacity-native-exchange-only"
        || value == "--capacity-short-slots-ab"
        || value == "--capacity-ingress-ab"
        || value == "--capacity-owner-claim-ab"
        || [
            "--capacity-count=",
            "--capacity-clients=",
            "--capacity-owners=",
            "--capacity-rust-workers=",
            "--capacity-rate=",
            "--capacity-short-slots=",
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
    ShortConcurrency,
    ActorSupply,
    TaggedInput,
    NativeExchange,
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
    supply: SupplyPath,
    short_only: bool,
    short_slots: usize,
    feed: usize,
    construction: Construction,
    handoff: ServiceHandoff,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ServiceHandoff {
    Inline,
    NativeExchange,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Construction {
    Composed,
    Tagged,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum SupplyPath {
    Clients,
    Resident,
    ResidentObserved,
}
impl Config {
    fn continuous(self) -> bool {
        self.supply != SupplyPath::Clients
    }

    fn costs(self) -> bool {
        self.supply == SupplyPath::ResidentObserved
    }
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
            comparison: if args.iter().any(|s| {
                matches!(
                    s.as_str(),
                    "--capacity-native-exchange-ab" | "--capacity-native-exchange-only"
                )
            }) {
                ComparisonMode::NativeExchange
            } else if args.iter().any(|s| s == "--capacity-tagged-input-ab") {
                ComparisonMode::TaggedInput
            } else if args.iter().any(|s| s == "--capacity-actor-supply-ab") {
                ComparisonMode::ActorSupply
            } else if args.iter().any(|s| s == "--capacity-short-slots-ab") {
                ComparisonMode::ShortConcurrency
            } else if args.iter().any(|s| s == "--capacity-owner-claim-ab") {
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
            supply: if args.iter().any(|s| s == "--capacity-continuous") {
                if args.iter().any(|s| s == "--phase-observation") {
                    SupplyPath::ResidentObserved
                } else {
                    SupplyPath::Resident
                }
            } else {
                SupplyPath::Clients
            },
            short_only: args.iter().any(|s| s == "--capacity-short-only"),
            short_slots: value("--capacity-short-slots=", 1),
            feed: 1,
            construction: if args.iter().any(|s| {
                matches!(
                    s.as_str(),
                    "--capacity-native-exchange-ab" | "--capacity-native-exchange-only"
                )
            }) {
                Construction::Tagged
            } else {
                Construction::Composed
            },
            handoff: if args.iter().any(|s| s == "--capacity-native-exchange-only") {
                ServiceHandoff::NativeExchange
            } else {
                ServiceHandoff::Inline
            },
        };
        if out.rate == 0 {
            out.class_ingress = false;
        }
        out.validate();
        Some(out)
    }
    fn exchange(self) -> bool {
        self.handoff == ServiceHandoff::NativeExchange
    }
    fn backpressure(self) -> bool {
        self.comparison == ComparisonMode::NativeExchange
    }
    fn validate(self) {
        assert!(self.count <= 1_000_000 && (1..=4096).contains(&self.clients));
        assert!(
            self.count > 0 || (self.rate > 0 && self.seconds > 0),
            "zero count minimum requires a positive timed arrival plan"
        );
        assert!((1..=3).contains(&self.short_slots));
        assert!(
            self.short_slots == 1
                || (self.short_only
                    && matches!(
                        self.comparison,
                        ComparisonMode::Single
                            | ComparisonMode::ActorSupply
                            | ComparisonMode::TaggedInput
                            | ComparisonMode::NativeExchange
                    )),
            "explicit short concurrency is a directed short-only profile"
        );
        assert!(
            self.comparison != ComparisonMode::ShortConcurrency
                || (self.short_only && self.short_slots == 1 && self.rate > 0),
            "short concurrency A/B requires short-only arrivals and the one-slot control"
        );
        assert!((1..=64).contains(&self.owners) && (1..=64).contains(&self.rust_workers));
        assert!(self.clients >= 2);
        assert!(
            self.comparison != ComparisonMode::Ingress || self.rate > 0,
            "ingress A/B requires planned arrivals"
        );
        assert!(self.seconds <= 120 && self.warmup <= 30 && self.rate <= 1_000_000);
        assert!(
            self.rate
                .saturating_mul(usize::try_from(self.seconds).unwrap())
                <= 10_000_000,
            "measurement sample bound"
        );
        if self.rate > 0 {
            assert!(
                self.count
                    .max(self.rate * usize::try_from(self.seconds).unwrap())
                    <= self.rate * 120,
                "planned capacity load must fit 120 seconds; lower count for low rates"
            );
        }
        assert!(
            !self.continuous()
                || (self.owners == 1 && self.short_only && self.rate > 0 && !self.class_ingress),
            "continuous profile requires one VM and short-only open arrivals"
        );
        assert!(
            !matches!(
                self.comparison,
                ComparisonMode::ActorSupply
                    | ComparisonMode::TaggedInput
                    | ComparisonMode::NativeExchange
            ) || (self.continuous() && self.short_slots == 3),
            "Actor supply A/B requires continuous short service with three active credits"
        );
    }
}
#[derive(Clone, Copy)]
struct Input {
    id: usize,
    planned: Instant,
    issued: Instant,
    short: bool,
}
#[derive(Clone, Copy)]
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
// Statistics storage is bounded by the known plan, not runtime queue capacity.
// Use nonzero sentinels and black_box so prefaulting is not optimized away.
fn record_buffer(count: usize) -> Vec<Record> {
    let mut rows = vec![
        Record {
            total: 1,
            queue: 1,
            generator: 1,
            pre_entry: 1,
            native: 1,
            delivery: 1,
            owner: usize::MAX,
            short: false,
            status: u8::MAX,
        };
        count
    ];
    std::hint::black_box(rows.as_slice());
    rows.clear();
    rows
}
fn ns(d: Duration) -> u64 {
    d.as_nanos().try_into().expect("bounded elapsed")
}
async fn request(router: &SchemeServiceRouter, input: Input, mixed: bool) -> Record {
    let short = input.short;
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
                        short: config.short_only || id % 17 == 16,
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
    let count = config
        .count
        .max(config.rate * usize::try_from(config.seconds).unwrap());
    if config.class_ingress {
        return class_offered(router, config, progress).await;
    }
    let records = record_buffer(count);
    let released = Instant::now();
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
    let (mut records, offered_elapsed) = emit(
        count,
        config.rate,
        Sink::Shared(send),
        released,
        config.short_only,
        records,
    )
    .await;
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
async fn emit(
    count: usize,
    rate: usize,
    send: Sink,
    released: Instant,
    short_only: bool,
    mut records: Vec<Record>,
) -> (Vec<Record>, Duration) {
    assert!(records.is_empty() && records.capacity() >= count);
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
                short: short_only || id % 17 == 16,
            }) {
                records.push(Record {
                    total: ns(Instant::now().duration_since(input.planned)),
                    queue: 0,
                    generator: ns(input.issued.duration_since(input.planned)),
                    pre_entry: 0,
                    native: 0,
                    delivery: 0,
                    owner: usize::MAX,
                    short: input.short,
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
fn generator_quantiles(rows: &[Record]) -> serde_json::Value {
    let mut lag: Vec<_> = rows.iter().map(|row| row.generator).collect();
    json!({"population":"all planned messages, including rejected and failed",
        "p99_ns":super::throughput::rank(&mut lag,9900),
        "max_ns":lag.iter().max()})
}
pub(super) fn run(
    pool: &InstancePool,
    runtime: &tokio::runtime::Runtime,
    progress: &mpsc::Sender<&'static str>,
    comparison: measurement::Comparison,
    config: Config,
) {
    let (router, _settings) = measurement::configure_service(pool, comparison, true);
    let router = if config.short_slots == 1 {
        router
    } else {
        // Reuse the public routing API. Repartition four existing slots;
        // preserve one bulk slot and the unchanged native work/queue bounds.
        router.close();
        SchemeServiceRouter::new(
            pool.dispatcher(),
            gerbil_scheme::instances::SchemeRoutingConfig {
                owners: (0..config.owners).collect(),
                short_slots: config.short_slots,
                batch_slots: 4 - config.short_slots,
                short_max_work: 1,
                batch_max_work: 8,
            },
        )
        .unwrap()
    };
    let router = if config.binding == ShortBinding::RunningOwner {
        router.with_owner_claimed_shorts().unwrap()
    } else {
        router
    };
    println!(
        "{}",
        json!({"event":"capacity_configuration","continuous_supply":config.continuous(),"feed":config.feed,"diagnostic_only":config.costs(),"continuous_outstanding_bound":if config.continuous(){Some(128)}else{None},"continuous_native_active":if config.continuous(){Some(config.short_slots)}else{None},"owner_claim":config.binding==ShortBinding::RunningOwner,"api":if config.continuous(){"continuous-short-service-messages"}else{"routed-typed-service-messages"},"qualifies_pool_pipeline_candidate":false,"count_minimum":config.count,"minimum_seconds":config.seconds,"sample_hard_bound":10_000_000,"clients":config.clients,"offered_rate":config.rate,"queue_capacity":if config.rate>0{Some(128)}else{None},"load_mode":if config.rate==0{"closed-loop-saturation"}else{"open-loop-planned-arrivals"},"short_only":config.short_only,"router_short_slots":config.short_slots,"router_batch_slots":4-config.short_slots,"native_slots":4,"native_work":32,"planned_messages":if config.rate>0{Some(config.count.max(config.rate*usize::try_from(config.seconds).unwrap()))}else{None},"bulk_messages_per_short":if config.short_only{0}else{16},"bulk_items_per_message":8,"bulk_lines_per_item":48,"short_lines":4,"mixed":config.mixed,"warmup_seconds":config.warmup,"owners":config.owners,"rust_workers":config.rust_workers,"class_ingress":config.class_ingress,"native_progress_watchdog_seconds":5,"defaults_changed":false})
    );
    configure_actor_supply(pool, config);
    if config.warmup > 0 {
        runtime.block_on(closed(router.clone(), config, progress.clone(), true));
    }
    measurement::idle(pool);
    let phases_before = config.costs().then(|| phase_snapshot(pool));
    let (rows, elapsed, offered_count, offered_elapsed) = if config.rate == 0 {
        let (rows, time) =
            runtime.block_on(closed(router.clone(), config, progress.clone(), false));
        let count = rows.len();
        (rows, time, count, time)
    } else if config.continuous() {
        runtime.block_on(continuous::run(pool, config, progress.clone()))
    } else {
        runtime.block_on(offered(router.clone(), config, progress.clone()))
    };
    progress
        .send("capacity measurement finished; outcomes collected")
        .unwrap();
    measurement::idle(pool);
    if let Some(before) = phases_before {
        println!(
            "{}",
            json!({"event":"capacity_owner_costs","feed":config.feed,"phase_order":["Advance","Wait","Submit","Retire","Supply","Release"],"diagnostic_only":true,"phases":super::observation::delta(&before,&phase_snapshot(pool))})
        );
    }
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
    let mut output = json!({"event":"capacity_result","continuous_supply":config.continuous(),"library_admission_attempted_messages":if config.continuous(){offered_count}else{successful+failed},"benchmark_ingress_rejected_messages":if config.continuous(){0}else{rejected},"library_rejected_before_admission":if config.continuous(){rejected}else{0},"router_short_slots":config.short_slots,"router_batch_slots":4-config.short_slots,"planned_duration_seconds":config.seconds,"owner_claim":config.binding==ShortBinding::RunningOwner,"class_ingress":config.class_ingress,"measurement_valid":config.rate>0 || (elapsed>=Duration::from_secs(config.seconds) && rows.len()>=config.count),"performance_accepted":false,"completed_messages":successful,"bulk_messages":bulk,"short_messages":short,"bulk_items":bulk*8,"completed_input_items":bulk*8+short,"offered_messages":offered_count,"planned_rate_messages_per_second":config.rate,"ingress_attempted_messages":offered_count,"ingress_accepted_messages":successful+failed,"library_started_messages":successful+failed,"rejected_before_admission":rejected,"failed_messages":failed,"elapsed_ns":elapsed.as_nanos(),"offered_elapsed_ns":offered_elapsed.as_nanos(),"all_arrival_generator_lag":generator_quantiles(&rows),"completed_messages_per_second":number(successful)/elapsed.as_secs_f64(),"emitted_messages_per_second":number(offered_count)/offered_elapsed.as_secs_f64(),"bulk_items_per_second":number(bulk*8)/elapsed.as_secs_f64(),"owner_completed_messages":owners,"owner_bulk_messages":(0..config.owners).map(|o|rows.iter().filter(|r|r.status==0 && r.owner==o && !r.short).count()).collect::<Vec<_>>(),"owner_short_messages":(0..config.owners).map(|o|rows.iter().filter(|r|r.status==0 && r.owner==o && r.short).count()).collect::<Vec<_>>(),"latency":classes,"latency_population":"successful messages; all rejected and failed arrivals remain in outcome accounting","statistics_prefaulted_before_clock":config.rate>0,"benchmark_record_bytes":rows.capacity()*std::mem::size_of::<Record>(),"latency_includes_payload_preparation":true,"throughput_includes_result_oracle":true,"closed_loop_is_not_open_loop_slo_evidence":config.rate==0,"pipeline_performance_acceptance":false});
    if config.backpressure() {
        let unadmitted = rows.iter().filter(|r| matches!(r.status, 3 | 4)).count();
        output["ingress_accepted_messages"] = json!(successful + failed - unadmitted);
        output["library_started_messages"] = json!(successful + failed - unadmitted);
        output["admission_timeouts"] = json!(rows.iter().filter(|r| r.status == 3).count());
        output["accepted_timeouts"] = json!(rows.iter().filter(|r| r.status == 5).count());
        output["other_admission_errors"] = json!(rows.iter().filter(|r| r.status == 4).count());
        output["emitted_population"] =
            json!("attempts including upstream expiry; not admission rate");
    }
    output["native_exchange"] = json!(config.exchange());
    output["bounded_backpressure"] = json!(config.backpressure());
    output["feed"] = json!(config.feed);
    output["tagged_input"] = json!(config.construction == Construction::Tagged);
    output["diagnostic_only"] = json!(config.costs());
    println!("{output}");
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
                let class = if input.short {
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
    let count = config
        .count
        .max(config.rate * usize::try_from(config.seconds).unwrap());
    let records = record_buffer(count);
    let released = Instant::now();
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
    let (mut rows, offered_elapsed) = emit(
        count,
        config.rate,
        Sink::Classes(ingress.clone()),
        released,
        config.short_only,
        records,
    )
    .await;
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
    if config.comparison == ComparisonMode::NativeExchange && !config.exchange() {
        config.handoff = ServiceHandoff::NativeExchange;
        run(pool, runtime, progress, comparison, config);
    }
    if config.comparison == ComparisonMode::TaggedInput {
        config.construction = Construction::Tagged;
        run(pool, runtime, progress, comparison, config);
    }
    if config.comparison == ComparisonMode::ActorSupply {
        config.feed = 3;
        run(pool, runtime, progress, comparison, config);
    }
    if config.comparison == ComparisonMode::ShortConcurrency {
        config.short_slots = 3;
        run(pool, runtime, progress, comparison, config);
    }
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
    if config.comparison == ComparisonMode::ShortConcurrency {
        println!(
            "{}",
            json!({"event":"capacity_short_concurrency_pair","same_vm_identities":true,"offered_rate":config.rate,"duration_seconds_per_arm":config.seconds,"short_slots":[1,3],"native_slots":4,"queue_capacity":128,"defaults_changed":false})
        );
    }
}

fn phase_snapshot(pool: &InstancePool) -> Vec<gerbil_scheme::instances::SchemeServiceObservation> {
    (0..pool.owners().len())
        .map(|owner| {
            pool.dispatcher()
                .scheme_service_observation_on(owner)
                .unwrap()
                .wait(GATE)
                .unwrap()
        })
        .collect()
}

fn configure_actor_supply(pool: &InstancePool, config: Config) {
    if config.backpressure() {
        pool.dispatcher()
            .configure_scheme_service_exchange_on(0, false)
            .unwrap()
            .wait(GATE)
            .unwrap();
    }
    if config.continuous() {
        pool.dispatcher()
            .configure_scheme_service_feed_on(0, config.feed)
            .unwrap();
    }
    if matches!(
        config.comparison,
        ComparisonMode::ActorSupply | ComparisonMode::TaggedInput | ComparisonMode::NativeExchange
    ) {
        use gerbil_scheme::instances::SchemeServiceCompletion;
        let policy = if config.feed == 1 {
            SchemeServiceCompletion::Snapshot
        } else {
            SchemeServiceCompletion::NativeCycle
        };
        pool.dispatcher()
            .configure_scheme_service_completion_on(0, policy)
            .unwrap()
            .wait(GATE)
            .unwrap();
        assert_eq!(
            pool.dispatcher()
                .scheme_service_completion_on(0)
                .unwrap()
                .wait(GATE)
                .unwrap()
                .policy,
            policy
        );
    }
    if config.backpressure() {
        pool.dispatcher()
            .configure_scheme_service_exchange_on(0, config.exchange())
            .unwrap()
            .wait(GATE)
            .unwrap();
    }
}

pub(super) fn report_stalled_flow() {
    continuous::report_stalled_flow();
}
