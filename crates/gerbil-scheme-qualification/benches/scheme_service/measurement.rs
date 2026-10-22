//! Finite same-owner sample; phase labels do not claim isolated CPU execution.
use super::{GATE, corpus, observation::Mode, workload};
use gerbil_scheme::instances::InstancePool;
use serde_json::{Value, json};
use std::{sync::Arc, time::Instant};
use workload::ShortMode;
pub(super) fn idle(pool: &InstancePool) {
    let start = Instant::now();
    while !pool.dispatcher().owner_capacity().iter().all(|c| {
        c.available == c.limit && c.work_available == c.work_limit && c.producer_available == 1
    }) {
        assert!(start.elapsed() < GATE, "service credits did not return");
        std::thread::yield_now();
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Comparison {
    Service,
    Combined,
    ShortLayout,
    ClassRouting,
    ShortService,
    Feed,
    Budget,
    Ingress,
    Notification,
    Reselection,
    OwnerQos,
    Resident,
    Pooled,
    PooledWake,
    ObserverCost,
    ShortRouteLoad,
    SupplyAdmission,
    PoolProtocol,
    SelectiveWake,
    RetirementRefill,
    ProgressFair,
}
impl Comparison {
    pub(super) fn is_split(self) -> bool {
        self != Self::Service
    }
    fn supply_admission(self, arm: bool) -> gerbil_scheme::instances::SchemeServiceSupplyAdmission {
        if self == Self::ProgressFair && arm {
            return gerbil_scheme::instances::SchemeServiceSupplyAdmission::ProgressFair;
        }
        if self == Self::RetirementRefill && arm {
            return gerbil_scheme::instances::SchemeServiceSupplyAdmission::RetirementRefill;
        }
        if matches!(
            self,
            Self::PoolProtocol | Self::SelectiveWake | Self::RetirementRefill | Self::ProgressFair
        ) || self == Self::SupplyAdmission && arm
        {
            gerbil_scheme::instances::SchemeServiceSupplyAdmission::OwnerLocal
        } else {
            gerbil_scheme::instances::SchemeServiceSupplyAdmission::Queued
        }
    }
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::ProgressFair => {
                "same native progress receipts; FIFO versus residual-service joint fairness"
            }
            Self::RetirementRefill => {
                "same selective wake and native work; ordinary refill versus retirement-bounded refill"
            }
            Self::SelectiveWake => {
                "same owner-claimed short supply; broadcast versus FIFO registered selective wake"
            }
            Self::PoolProtocol => {
                "same owner-local supply; watch host routing versus shared owner-claimed short requests"
            }
            Self::SupplyAdmission => {
                "same pooled supply; queued/wrapped versus owner-local native credit ownership"
            }
            Self::ShortRouteLoad => {
                "same pooled supply; short owner occupancy versus class-load routing"
            }
            Self::ObserverCost => {
                "same pooled native workload; observation off versus request observation"
            }
            Self::Service => "synchronous versus Scheme-owned service",
            Self::Combined => "batch-short/FIFO versus native short/ready completion",
            Self::ShortLayout => "same FIFO completion; batch-short versus native short",
            Self::Reselection => {
                "same native work/queues/quotas; pinned versus capacity-reserved owner reselection"
            }
            Self::PooledWake => {
                "same pooled supply/budget; native port versus Scheme completion condition wake"
            }
            Self::Pooled => {
                "same progress-aware short routing; fixed versus pooled bounded resident supply"
            }
            Self::Resident => {
                "same fixed-owner service and short routing; host refill versus bounded owner-resident supply"
            }
            Self::OwnerQos => {
                "same reselected native service; restored original versus explicit owner OS QoS"
            }
            Self::Notification => {
                "same native port/budget/work; coalesced versus independent short notification"
            }
            Self::Ingress => "same cooperative budget/native work; watcher versus native port wake",
            Self::Budget => {
                "same short queues/router/native work/feed; Scheme cooperative budget disabled versus one"
            }
            Self::Feed => {
                "same short queues/router/native work; feed one versus four admitted inputs per Scheme advance"
            }
            Self::ShortService => {
                "same class router and native short layout; general versus Scheme short scheduling"
            }
            Self::ClassRouting => {
                "same native short/FIFO completion/quotas; fixed owner versus class-specific routing"
            }
        }
    }
}
#[derive(Clone, Copy)]
struct Strategy {
    short_mode: ShortMode,
    ready_completion: bool,
    class_routing: bool,
}
impl Strategy {
    fn for_arm(comparison: Comparison, arm: bool, observation: Mode) -> Self {
        Self {
            short_mode: if matches!(
                comparison,
                Comparison::ShortService
                    | Comparison::Feed
                    | Comparison::Budget
                    | Comparison::Ingress
                    | Comparison::Notification
                    | Comparison::Reselection
                    | Comparison::OwnerQos
                    | Comparison::Resident
                    | Comparison::Pooled
                    | Comparison::PooledWake
                    | Comparison::ObserverCost
                    | Comparison::ShortRouteLoad
                    | Comparison::SupplyAdmission
                    | Comparison::PoolProtocol
                    | Comparison::SelectiveWake
                    | Comparison::RetirementRefill
                    | Comparison::ProgressFair
            ) && (arm
                || matches!(
                    comparison,
                    Comparison::Feed
                        | Comparison::Budget
                        | Comparison::Ingress
                        | Comparison::Notification
                        | Comparison::Reselection
                        | Comparison::OwnerQos
                        | Comparison::Resident
                        | Comparison::Pooled
                        | Comparison::PooledWake
                        | Comparison::ObserverCost
                        | Comparison::ShortRouteLoad
                        | Comparison::SupplyAdmission
                        | Comparison::PoolProtocol
                        | Comparison::SelectiveWake
                        | Comparison::RetirementRefill
                        | Comparison::ProgressFair
                )) {
                ShortMode::Dedicated(observation)
            } else if matches!(
                comparison,
                Comparison::ClassRouting | Comparison::ShortService
            ) || (comparison.is_split() && arm)
            {
                ShortMode::Native(observation)
            } else {
                ShortMode::Batch
            },
            ready_completion: comparison == Comparison::Combined && arm,
            class_routing: matches!(
                comparison,
                Comparison::ShortService
                    | Comparison::Feed
                    | Comparison::Budget
                    | Comparison::Ingress
                    | Comparison::Notification
                    | Comparison::Reselection
                    | Comparison::OwnerQos
                    | Comparison::Resident
                    | Comparison::Pooled
                    | Comparison::PooledWake
                    | Comparison::ObserverCost
                    | Comparison::ShortRouteLoad
                    | Comparison::SupplyAdmission
                    | Comparison::PoolProtocol
                    | Comparison::SelectiveWake
                    | Comparison::RetirementRefill
                    | Comparison::ProgressFair
            ) || (comparison == Comparison::ClassRouting && arm),
        }
    }
}
pub(super) fn run(
    pool: &InstancePool,
    runtime: &tokio::runtime::Runtime,
    inputs: &Arc<Vec<Vec<i64>>>,
    probes: usize,
    comparison: Comparison,
    arm: bool,
    observation: Mode,
) -> Value {
    let observation = observation.paired(comparison, arm);
    let strategy = Strategy::for_arm(comparison, arm, observation);
    let asynchronous = comparison != Comparison::Service || arm;
    idle(pool);
    let router = router(pool, comparison, arm);
    let active_router = strategy.class_routing.then(|| router.clone());
    let settings = super::settings::configure(pool, comparison, arm);
    // Settings replies precede native permit retirement. Policy mutation must
    // observe returned credits, not just a delivered reply.
    idle(pool);
    super::settings::short_wake(pool, comparison, arm);
    idle(pool);
    let qos = (comparison == Comparison::OwnerQos).then(|| super::native_wait::qos(pool, arm));
    let capture = super::observation::Capture::start(
        pool,
        observation,
        comparison == Comparison::ObserverCost,
    );
    let deliveries_before = peer_deliveries(pool);
    let released = Instant::now();
    let bulk_inputs = Arc::clone(inputs);
    let dispatch = pool.dispatcher();
    let probe_dispatch = pool.dispatcher();
    let (rows, probe) = runtime.block_on(async {
        let bulk_router = if matches!(
            comparison,
            Comparison::Resident
                | Comparison::Pooled
                | Comparison::PooledWake
                | Comparison::ObserverCost
                | Comparison::ShortRouteLoad
                | Comparison::SupplyAdmission
                | Comparison::PoolProtocol
                | Comparison::SelectiveWake
                | Comparison::RetirementRefill
                | Comparison::ProgressFair
        ) {
            None
        } else {
            active_router.clone()
        };
        let bulk = runtime.spawn(run_bulk(
            dispatch,
            bulk_inputs,
            released,
            comparison,
            arm,
            strategy,
            bulk_router,
        ));
        let probes = runtime.spawn(workload::probes(
            probe_dispatch,
            probes,
            asynchronous,
            released,
            strategy.short_mode,
            active_router,
            (comparison == Comparison::PoolProtocol && arm
                || matches!(
                    comparison,
                    Comparison::SelectiveWake
                        | Comparison::RetirementRefill
                        | Comparison::ProgressFair
                ))
            .then(|| pool.dispatcher().scheme_short_pool().unwrap()),
        ));
        (bulk.await.unwrap(), probes.await.unwrap())
    });
    let elapsed = released.elapsed().as_nanos();
    idle(pool);
    close_router(&router);
    let mut value = summarize(
        rows,
        inputs,
        &probe,
        released,
        elapsed,
        asynchronous,
        strategy,
    );
    capture.finish(
        &mut value,
        pool,
        inputs,
        probe.diagnostics.iter().flatten().count() * 3,
        released,
    );
    if let Some(qos) = qos {
        value["owner_qos_receipts"] = qos;
        value["owner_qos_restored"] = super::native_wait::qos(pool, false);
    }
    value["peer_inbox_deliveries"] = json!(peer_deliveries(pool) - deliveries_before);
    append_policy(&mut value, pool, comparison, arm, &settings, inputs, probes);
    if observation.full() && comparison == Comparison::Notification && arm {
        super::budget::verify_notifications(&value, probes);
    }
    value
}
async fn run_bulk(
    dispatch: gerbil_scheme::instances::InstanceDispatcher,
    inputs: Arc<Vec<Vec<i64>>>,
    released: Instant,
    comparison: Comparison,
    arm: bool,
    strategy: Strategy,
    router: Option<gerbil_scheme::instances::SchemeServiceRouter>,
) -> Vec<gerbil_scheme_python::owner_dispatch::OwnerResult<workload::Reply>> {
    if matches!(
        comparison,
        Comparison::PooledWake
            | Comparison::ObserverCost
            | Comparison::ShortRouteLoad
            | Comparison::SupplyAdmission
            | Comparison::PoolProtocol
            | Comparison::SelectiveWake
            | Comparison::RetirementRefill
            | Comparison::ProgressFair
    ) || comparison == Comparison::Pooled && arm
    {
        super::pooled::bulk(dispatch, inputs, released, comparison.supply_admission(arm)).await
    } else if comparison == Comparison::Pooled || comparison == Comparison::Resident && arm {
        super::resident::bulk(dispatch, inputs, released).await
    } else {
        workload::bulk(
            dispatch,
            inputs,
            comparison != Comparison::Service || arm,
            released,
            strategy.ready_completion,
            comparison.is_split(),
            router,
        )
        .await
    }
}
fn router(
    pool: &InstancePool,
    comparison: Comparison,
    arm: bool,
) -> gerbil_scheme::instances::SchemeServiceRouter {
    gerbil_scheme::instances::SchemeServiceRouter::with_admission(
        pool.dispatcher(),
        gerbil_scheme::instances::SchemeRoutingConfig {
            owners: vec![0, 1, 2, 3],
            short_slots: 1,
            batch_slots: 3,
            short_max_work: 1,
            batch_max_work: 8,
        },
        if comparison == Comparison::PoolProtocol && arm
            || matches!(
                comparison,
                Comparison::SelectiveWake | Comparison::RetirementRefill | Comparison::ProgressFair
            )
        {
            gerbil_scheme::instances::SchemeRoutingAdmission::ReselectProgressNotified
        } else if comparison == Comparison::ShortRouteLoad && arm {
            gerbil_scheme::instances::SchemeRoutingAdmission::Reselect
        } else if matches!(
            comparison,
            Comparison::Pooled
                | Comparison::PooledWake
                | Comparison::ObserverCost
                | Comparison::ShortRouteLoad
                | Comparison::SupplyAdmission
                | Comparison::PoolProtocol
                | Comparison::SelectiveWake
                | Comparison::RetirementRefill
                | Comparison::ProgressFair
        ) {
            gerbil_scheme::instances::SchemeRoutingAdmission::ReselectProgress
        } else if (comparison == Comparison::Reselection && arm)
            || matches!(
                comparison,
                Comparison::OwnerQos
                    | Comparison::Resident
                    | Comparison::Pooled
                    | Comparison::PooledWake
                    | Comparison::ObserverCost
                    | Comparison::ShortRouteLoad
                    | Comparison::SupplyAdmission
                    | Comparison::PoolProtocol
                    | Comparison::SelectiveWake
                    | Comparison::RetirementRefill
                    | Comparison::ProgressFair
            )
        {
            gerbil_scheme::instances::SchemeRoutingAdmission::Reselect
        } else {
            gerbil_scheme::instances::SchemeRoutingAdmission::Pinned
        },
    )
    .unwrap()
}
fn append_policy(
    value: &mut Value,
    pool: &InstancePool,
    comparison: Comparison,
    arm: bool,
    settings: &[gerbil_scheme::instances::SchemeServiceSettings],
    inputs: &[Vec<i64>],
    probes: usize,
) {
    use gerbil_scheme::instances::SchemeServiceWake;
    value["native_policy_receipts"] = json!(settings.iter().enumerate().map(|(owner,s)|
        json!({"owner":owner,"scheme_budget":s.cooperative_budget,"wake":format!("{:?}",s.wake)})).collect::<Vec<_>>());
    value["owner_phase_names"] =
        json!(["Advance", "Wait", "Submit", "Retire", "Supply", "Release"]);
    value["scheme_budget"] = json!(settings[0].cooperative_budget);
    value["native_port_wake"] = json!(settings[0].wake == SchemeServiceWake::NativePort);
    value["independent_short_notifications"] = json!(comparison == Comparison::Notification && arm);
    value["scheme_budget_counters"] =
        super::budget::snapshot(pool, settings[0].cooperative_budget > 0, inputs, probes);
    value["owner_reselection"] = json!(
        (comparison == Comparison::Reselection && arm)
            || matches!(
                comparison,
                Comparison::OwnerQos
                    | Comparison::Resident
                    | Comparison::Pooled
                    | Comparison::PooledWake
                    | Comparison::ObserverCost
                    | Comparison::ShortRouteLoad
                    | Comparison::SupplyAdmission
                    | Comparison::PoolProtocol
                    | Comparison::SelectiveWake
                    | Comparison::RetirementRefill
                    | Comparison::ProgressFair
            )
    );
    value["owner_resident_supply"] = json!(
        matches!(
            comparison,
            Comparison::Pooled
                | Comparison::PooledWake
                | Comparison::ObserverCost
                | Comparison::ShortRouteLoad
                | Comparison::SupplyAdmission
                | Comparison::PoolProtocol
                | Comparison::SelectiveWake
                | Comparison::RetirementRefill
                | Comparison::ProgressFair
        ) || comparison == Comparison::Resident && arm
    );
    value["pool_resident_supply"] = json!(
        matches!(
            comparison,
            Comparison::PooledWake
                | Comparison::ObserverCost
                | Comparison::ShortRouteLoad
                | Comparison::SupplyAdmission
                | Comparison::PoolProtocol
                | Comparison::SelectiveWake
                | Comparison::RetirementRefill
                | Comparison::ProgressFair
        ) || comparison == Comparison::Pooled && arm
    );
    value["producer_progress_routing"] = json!(
        matches!(
            comparison,
            Comparison::Pooled
                | Comparison::PooledWake
                | Comparison::ObserverCost
                | Comparison::ShortRouteLoad
                | Comparison::SupplyAdmission
                | Comparison::PoolProtocol
                | Comparison::SelectiveWake
                | Comparison::RetirementRefill
                | Comparison::ProgressFair
        ) && !(comparison == Comparison::ShortRouteLoad && arm)
    );
    append_short_policy(value, pool, comparison, arm);
    value["supply_admission"] = json!(format!("{:?}", comparison.supply_admission(arm)));
    value["feed_limit"] = json!(if comparison == Comparison::Feed && arm {
        4
    } else {
        1
    });
}
fn append_short_policy(value: &mut Value, pool: &InstancePool, comparison: Comparison, arm: bool) {
    let deferred = comparison == Comparison::PoolProtocol && arm
        || matches!(
            comparison,
            Comparison::SelectiveWake | Comparison::RetirementRefill | Comparison::ProgressFair
        );
    value["native_progress_receipts"] = json!(
        pool.dispatcher()
            .scheme_service_progress_on(0)
            .unwrap()
            .enabled
    );
    value["joint_progress_fairness"] = json!(comparison == Comparison::ProgressFair && arm);
    value["short_wake_policy"] = json!(
        pool.dispatcher()
            .scheme_short_pool()
            .map(|p| format!("{:?}", p.wake_policy()))
    );
    value["selective_short_wake"] = json!(
        matches!(
            comparison,
            Comparison::RetirementRefill | Comparison::ProgressFair
        ) || comparison == Comparison::SelectiveWake && arm
    );
    value["short_admission_boundary"] = json!(if deferred {
        "owner claim"
    } else {
        "host publication"
    });
    value["deferred_short_owner_claim"] = json!(deferred);
    if deferred {
        value["producer_progress_routing"] = json!(false);
        value["owner_reselection"] = json!(false);
    }
}
fn summarize(
    rows: Vec<gerbil_scheme_python::owner_dispatch::OwnerResult<workload::Reply>>,
    inputs: &[Vec<i64>],
    probe: &workload::Probes,
    released: Instant,
    elapsed: u128,
    asynchronous: bool,
    strategy: Strategy,
) -> Value {
    let count = inputs.len();
    let mut clocks = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut service = Vec::new();
    let mut construct = Vec::new();
    let mut pending = Vec::new();
    let mut projection = Vec::new();
    let mut peer = usize::MAX;
    for (index, row) in rows.into_iter().enumerate() {
        let (reply, completed) = row.unwrap();
        assert_eq!(reply.values.len(), 8);
        for (offset, value) in reply.values.iter().enumerate() {
            corpus::verify(&inputs[index * 8 + offset], value);
        }
        peer = peer.min(corpus::peers(&reply.values));
        service.push(completed.duration_since(reply.entered).as_nanos());
        construct.push(reply.constructed.duration_since(reply.entered).as_nanos());
        pending.push(
            reply
                .projection
                .duration_since(reply.constructed)
                .as_nanos(),
        );
        projection.push(completed.duration_since(reply.projection).as_nanos());
        clocks[reply.owner].extend([completed; 8]);
    }
    // A shared source need not assign work to every owner in a short sample.
    // Keep unused owners in fairness as zero, rather than filtering them out.
    let unused_owners: Vec<_> = clocks
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.is_empty().then_some(i))
        .collect();
    let owner_totals: Vec<_> = clocks.iter().map(Vec::len).collect();
    let end = *clocks.iter().flatten().max().unwrap();
    let first_done = *clocks.iter().filter_map(|c| c.iter().max()).min().unwrap();
    let minimum = clocks
        .iter()
        .map(|c| c.iter().filter(|&&at| at <= first_done).count())
        .min()
        .unwrap();
    let progress_fraction = clocks
        .iter()
        .map(|c| {
            let done = u32::try_from(c.iter().filter(|&&at| at <= first_done).count()).unwrap();
            let total = u32::try_from(c.len()).unwrap();
            if total == 0 {
                0.0
            } else {
                f64::from(done) / f64::from(total)
            }
        })
        .reduce(f64::min)
        .unwrap();
    // Finish spread covers participating owners; unused_owners remains explicit.
    let finish_spread = clocks
        .iter()
        .filter_map(|c| c.iter().max())
        .map(|at| at.duration_since(first_done).as_nanos())
        .max()
        .unwrap();
    let owner_native_ns: Vec<Vec<u128>> = clocks
        .iter()
        .map(|c| {
            c.iter()
                .map(|at| at.duration_since(released).as_nanos())
                .collect()
        })
        .collect();
    json!({"orders":count,"asynchronous":asynchronous,"short_layout":strategy.short_mode.native(),"ready_completion":strategy.ready_completion,"class_routing":strategy.class_routing,"short_service":strategy.short_mode.dedicated(),"elapsed_ns":elapsed,"bulk_elapsed_ns":end.duration_since(released).as_nanos(),
        "bulk_owner_native_ns":owner_native_ns,"bulk_service_ns":service,"bulk_construct_ns":construct,
        "bulk_pending_before_projection_ns":pending,"bulk_projection_retirement_ns":projection,
        "probe_owners":probe.owners,"probe_entry":probe.entry,"probe_actual_ns":probe.actual,"probe_scheduled_ns":probe.scheduled,"probe_admission_ns":probe.admission,"probe_service_ns":probe.service,"probe_native_diagnostics":probe.diagnostics,"probe_native_snapshot_ns":probe.snapshot_ns,"probe_post_admission_wait_ns":probe.post_admission,"probe_construct_ns":probe.construct,"probe_pending_ns":probe.pending,"probe_projection_ns":probe.projection,
        "probe_overlap":probe.completed.iter().filter(|&&at|at<=end).count(),"owner_progress_at_first_done":minimum,"owner_totals":owner_totals,"unused_owners":unused_owners,"owner_progress_fraction_min":progress_fraction,
        "owner_finish_spread_ns":finish_spread,"actor_peer_min":peer})
}
fn close_router(router: &gerbil_scheme::instances::SchemeServiceRouter) {
    assert!(
        router.loads().iter().all(|load| load.short_requests == 0
            && load.batch_requests == 0
            && load.batch_work == 0)
    );
    router.close();
}

fn quantile(samples: &[Value], field: &str, percent: usize) -> u128 {
    let mut data: Vec<_> = samples
        .iter()
        .flat_map(|s| s[field].as_array().unwrap())
        .map(|v| u128::from(v.as_u64().unwrap()))
        .collect();
    corpus::percentile(&mut data, percent)
}
pub(super) fn emit(samples: &[Value], arm: usize, mixed: bool) {
    let mut duration: Vec<_> = samples
        .iter()
        .map(|s| u128::from(s["bulk_elapsed_ns"].as_u64().unwrap()))
        .collect();
    let median = corpus::percentile(&mut duration, 50);
    let seconds = std::time::Duration::from_nanos(u64::try_from(median).unwrap()).as_secs_f64();
    println!(
        "{}",
        json!({"event":"service_result","arm":arm,"mixed":mixed,"samples":samples.len(),
        "bulk_orders_per_second":samples[0]["orders"].as_f64().unwrap()/seconds,
        "bulk_service_p99_ns":quantile(samples,"bulk_service_ns",99),"bulk_pending_p99_ns":quantile(samples,"bulk_pending_before_projection_ns",99),
        "probe_actual_p99_ns":quantile(samples,"probe_actual_ns",99),"probe_scheduled_p99_ns":quantile(samples,"probe_scheduled_ns",99),
        "probe_admission_p99_ns":quantile(samples,"probe_admission_ns",99),
        "probe_post_admission_wait_p99_ns":quantile(samples,"probe_post_admission_wait_ns",99),
        "probe_construct_p99_ns":quantile(samples,"probe_construct_ns",99),"probe_pending_p99_ns":quantile(samples,"probe_pending_ns",99),"probe_projection_p99_ns":quantile(samples,"probe_projection_ns",99),
        "owner_progress_min":samples.iter().map(|s|s["owner_progress_at_first_done"].as_u64().unwrap()).min(),
        "owner_progress_fraction_min":samples.iter().map(|s|s["owner_progress_fraction_min"].as_f64().unwrap()).reduce(f64::min),
        "owner_finish_spread_max_ns":samples.iter().map(|s|s["owner_finish_spread_ns"].as_u64().unwrap()).max(),
        "actor_peer_min":samples.iter().map(|s|s["actor_peer_min"].as_u64().unwrap()).min(),
        "probe_overlap_min":samples.iter().map(|s|s["probe_overlap"].as_u64().unwrap()).min()})
    );
}

fn peer_deliveries(pool: &InstancePool) -> u64 {
    (0..4)
        .map(|owner| {
            pool.dispatcher()
                .scheme_service_progress_on(owner)
                .unwrap()
                .peer_deliveries
        })
        .sum()
}
