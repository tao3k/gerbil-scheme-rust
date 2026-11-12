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
        if self == Self::ProgressFair && (arm || super::settings::completion_ab()) {
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
            Self::ProgressFair if super::settings::activation_plan_ab() => activation_label(),
            Self::ProgressFair if super::settings::lease_plan_ab() => {
                "same VMs and native typed workload; Scheme event-owned class permits, cold layouts and lease release accounting"
            }
            Self::ProgressFair if super::settings::work_plan_protocol_ab() => {
                "same VMs and published native-pipeline configuration; control versus joint Scheme dispatch plus lexical bounded work plans"
            }
            Self::ProgressFair if super::settings::work_plan_ab() => {
                "same VMs and joint dispatch; original execution versus Scheme lexical work plans with explicit four-record contention / 48-record in-flight bounds"
            }
            Self::ProgressFair if super::settings::work_budget_ab() => {
                "same VMs, admission and joint dispatch; whole handler leases versus Scheme executed-work safe spans and ready-generation handoff"
            }
            Self::ProgressFair if super::settings::joint_progress_ab() => {
                "same VMs and native round trip; implicit ready fast path versus Scheme-owned dispatch obligations and scheduler checkpoints"
            }
            Self::ProgressFair if super::settings::native_entry_ab() => {
                "same VMs, native payload and refill; rooted envelope handoff versus Scheme-owned construction/admission capability"
            }
            Self::ProgressFair if super::settings::coupled_refill_ab() => {
                "same VMs and fused native inputs; barrier refill versus bounded FIFO/short/supply owner turns"
            }
            Self::ProgressFair if super::settings::pipeline_cycle_ab() => {
                "same VMs, typed inputs and routing; prepared cycle versus fused native construction and credit-bounded refill"
            }
            Self::ProgressFair if super::settings::prepared_cycle_ab() => {
                "same live VMs and typed work; native cycle versus bounded Rust preparation with ready retirement priority"
            }
            Self::ProgressFair if super::settings::native_cycle_ab() => {
                "same live VMs and typed work; snapshot versus native batched owner service cycle"
            }
            Self::ProgressFair if super::settings::completion_coalesced_ab() => {
                "same ProgressFair supply and Selective short routing; snapshot versus epoch-coalesced completion custody"
            }
            Self::ProgressFair if super::settings::completion_fair_ab() => {
                "same ProgressFair supply and Selective short routing; snapshot versus fair completion handoff"
            }
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
pub(super) fn configure_service(
    pool: &InstancePool,
    comparison: Comparison,
    arm: bool,
) -> (
    gerbil_scheme::instances::SchemeServiceRouter,
    Vec<gerbil_scheme::instances::SchemeServiceSettings>,
) {
    super::stall_forensics::phase(1);
    idle(pool);
    super::stall_forensics::phase(2);
    let router = router(pool, comparison, arm);
    super::stall_forensics::phase(3);
    let settings = super::settings::configure(pool, comparison, arm);
    // Settings replies precede native permit retirement. Policy mutation must
    // observe returned credits, not just a delivered reply.
    super::stall_forensics::phase(4);
    idle(pool);
    super::stall_forensics::phase(5);
    super::settings::short_wake(pool, comparison, arm);
    super::stall_forensics::phase(6);
    idle(pool);
    (router, settings)
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
    let (router, settings) = configure_service(pool, comparison, arm);
    let active_router = strategy.class_routing.then(|| router.clone());
    let qos = (comparison == Comparison::OwnerQos).then(|| super::native_wait::qos(pool, arm));
    let capture = super::observation::Capture::start(
        pool,
        observation,
        comparison == Comparison::ObserverCost,
    );
    super::stall_forensics::phase(7);
    let deliveries_before = peer_deliveries(pool);
    let outcomes_before = offer_outcomes(pool);
    let completions_before =
        super::settings::batched_completion_ab().then(|| super::settings::completions(pool));
    let released = Instant::now();
    let bulk_inputs = Arc::clone(inputs);
    let dispatch = pool.dispatcher();
    let probe_dispatch = pool.dispatcher();
    super::stall_forensics::phase(8);
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
        let probe_work = workload::probes(
            probe_dispatch,
            workload::ProbeSchedule::for_arm(probes, arm),
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
        );
        run_clients(
            runtime,
            (dispatch, bulk_inputs, bulk_router),
            released,
            comparison,
            arm,
            strategy,
            probe_work,
        )
        .await
    });
    let elapsed = released.elapsed().as_nanos();
    super::stall_forensics::phase(9);
    idle(pool);
    close_router(&router);
    super::stall_forensics::phase(10);
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
    super::stall_forensics::phase(11);
    append_offer_outcomes(&mut value, pool, deliveries_before, outcomes_before);
    append_completions(&mut value, pool, completions_before);
    append_policy(&mut value, pool, comparison, arm, &settings, inputs, probes);
    if observation.full() && comparison == Comparison::Notification && arm {
        super::budget::verify_notifications(&value, probes);
    }
    super::stall_forensics::phase(12);
    value
}
type BulkClient = (
    gerbil_scheme::instances::InstanceDispatcher,
    Arc<Vec<Vec<i64>>>,
    Option<gerbil_scheme::instances::SchemeServiceRouter>,
);
async fn run_clients<P: std::future::Future<Output = workload::Probes> + Send + 'static>(
    runtime: &tokio::runtime::Runtime,
    bulk: BulkClient,
    released: Instant,
    comparison: Comparison,
    arm: bool,
    strategy: Strategy,
    progress: P,
) -> (
    Vec<gerbil_scheme_python::owner_dispatch::OwnerResult<workload::Reply>>,
    workload::Probes,
) {
    let (dispatch, inputs, router) = bulk;
    if super::settings::client_turn_ab() && arm || super::settings::completion_service_active(arm) {
        return runtime
            .spawn(super::pooled::bulk_progress(
                dispatch,
                inputs,
                released,
                comparison.supply_admission(arm),
                progress,
                !super::settings::supply_cycle_ab()
                    && (!super::settings::input_preparation_ab() || arm),
                super::settings::supply_cycle_ab() && arm,
            ))
            .await
            .unwrap();
    }
    let bulk_work = run_bulk(
        dispatch, inputs, released, comparison, arm, strategy, router,
    );
    let bulk = runtime.spawn(bulk_work);
    let probes = runtime.spawn(progress);
    (bulk.await.unwrap(), probes.await.unwrap())
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
        super::pooled::bulk(
            dispatch,
            inputs,
            released,
            comparison.supply_admission(arm),
            (super::settings::arrival_clock_ab() || super::settings::completion_service_ab())
                || super::settings::client_turn_ab()
                || super::settings::ready_owner_ab() && arm,
        )
        .await
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
            owners: (0..pool.owners().len()).collect(),
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
fn append_offer_outcomes(
    value: &mut Value,
    pool: &InstancePool,
    deliveries_before: u64,
    outcomes_before: [u64; 3],
) {
    value["peer_inbox_deliveries"] = json!(peer_deliveries(pool) - deliveries_before);
    let outcomes = offer_outcomes(pool);
    value["peer_offer_withdrawals"] = json!(outcomes[0] - outcomes_before[0]);
    value["peer_offer_entries"] = json!(outcomes[1] - outcomes_before[1]);
    value["source_wake_signals"] = json!(outcomes[2] - outcomes_before[2]);
}
fn append_completions(value: &mut Value, pool: &InstancePool, before: Option<[u64; 3]>) {
    if let Some(before) = before {
        let after = super::settings::completions(pool);
        value["completion_opportunities"] = json!(after[0] - before[0]);
        value["completion_batches"] = json!(after[1] - before[1]);
        value["completion_results"] = json!(after[2] - before[2]);
    }
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
    value["native_entry_ab"] = json!(super::settings::native_entry_ab());
    value["native_entry"] = json!(
        super::settings::native_entry_ab()
            && (arm || super::settings::joint_progress_ab() || super::settings::work_budget_ab())
    );
    value["native_bulk_projection"] = json!(
        super::settings::native_entry_ab()
            && (arm || super::settings::joint_progress_ab() || super::settings::work_budget_ab())
    );
    value["joint_progress_ab"] = json!(super::settings::joint_progress_ab());
    value["joint_progress"] = json!(super::settings::joint_enabled(arm));
    if super::settings::joint_progress_ab() || super::settings::work_budget_ab() {
        let joint = super::settings::joint_enabled(arm);
        let counts = super::settings::joint_counts(pool, joint);
        let probes = u64::try_from(value["probe_actual_ns"].as_array().unwrap().len()).unwrap();
        assert_eq!(
            counts.iter().map(|row| row[2]).sum::<u64>(),
            if joint { probes } else { 0 }
        );
        value["joint_progress_counts"] = json!(counts);
    }
    append_work_policy(value, pool, arm);
    value["bulk_construct_includes_submit"] = json!(super::settings::native_entry_ab());
    value["coupled_refill_ab"] = json!(super::settings::coupled_refill_ab());
    value["coupled_refill"] = json!(super::settings::coupled_refill_ab() && arm);
    value["pipeline_cycle_ab"] = json!(super::settings::pipeline_cycle_ab());
    value["pipeline_service_cycle"] = json!(
        super::settings::native_entry_ab()
            || super::settings::coupled_refill_ab()
            || (super::settings::pipeline_cycle_ab() && arm)
    );
    value["prepared_cycle_ab"] = json!(super::settings::prepared_cycle_ab());
    value["native_cycle_ab"] = json!(super::settings::native_cycle_ab());
    value["native_service_cycle"] = json!(
        super::settings::native_entry_ab()
            || super::settings::coupled_refill_ab()
            || super::settings::pipeline_cycle_ab()
            || super::settings::prepared_cycle_ab()
            || (super::settings::native_cycle_ab() && arm)
    );
    value["prepared_input_cycle"] = json!(
        super::settings::native_entry_ab()
            || super::settings::coupled_refill_ab()
            || super::settings::pipeline_cycle_ab()
            || (super::settings::prepared_cycle_ab() && arm)
    );
    value["completion_coalesced_ab"] = json!(super::settings::completion_coalesced_ab());
    value["coalesced_completion_handoff"] =
        json!(super::settings::completion_coalesced_ab() && arm);
    value["completion_fair_ab"] = json!(super::settings::completion_fair_ab());
    value["fair_completion_handoff"] = json!(super::settings::completion_fair_ab() && arm);
    value["joint_progress_fairness"] = json!(
        comparison == Comparison::ProgressFair && arm && !super::settings::fixed_short_selective()
    );
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
    let work_counts = super::throughput::counts(count, rows.len(), probe.actual.len());
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
    // Same-request timestamps form an additive partition. Placement observed
    // after await is a client notification clock and can lag native completion.
    for (index, actual) in probe.actual.iter().enumerate() {
        assert_eq!(
            *actual,
            probe.before_entry[index]
                + probe.construct[index]
                + probe.pending[index]
                + probe.projection[index],
            "short request timestamps must conserve actual elapsed time"
        );
    }
    let mut value = json!({"probe_timing_contract":"planned-issued-native-consumed-v3","orders":count,"asynchronous":asynchronous,"short_layout":strategy.short_mode.native(),"ready_completion":strategy.ready_completion,"class_routing":strategy.class_routing,"short_service":strategy.short_mode.dedicated(),"elapsed_ns":elapsed,"bulk_elapsed_ns":end.duration_since(released).as_nanos(),
        "bulk_owner_native_ns":owner_native_ns,"bulk_service_ns":service,"bulk_construct_ns":construct,
        "bulk_pending_before_projection_ns":pending,"bulk_projection_retirement_ns":projection,
        "probe_owners":probe.owners,"probe_entry":probe.entry,"probe_actual_ns":probe.actual,"probe_scheduled_ns":probe.scheduled,"probe_placement_observed_ns":probe.placement_observed,"probe_service_ns":probe.service,"probe_native_diagnostics":probe.diagnostics,"probe_native_snapshot_ns":probe.snapshot_ns,"probe_before_entry_ns":probe.before_entry,"probe_construct_ns":probe.construct,"probe_pending_ns":probe.pending,"probe_projection_ns":probe.projection,
        "probe_overlap":probe.completed.iter().filter(|&&at|at<=end).count(),"owner_progress_at_first_done":minimum,"owner_totals":owner_totals,"unused_owners":unused_owners,"owner_progress_fraction_min":progress_fraction,
        "owner_finish_spread_ns":finish_spread,"actor_peer_min":peer});
    value["work_counts"] = work_counts;
    append_client_progress(&mut value, probe);
    value
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
    let throughput = super::throughput::summary(samples);
    println!(
        "{}",
        json!({"event":"service_result","arm":arm,"mixed":mixed,"samples":samples.len(),
        "throughput":throughput,
        "bulk_service_p99_ns":quantile(samples,"bulk_service_ns",99),"bulk_pending_p99_ns":quantile(samples,"bulk_pending_before_projection_ns",99),
        "probe_client_total_p99_ns":quantile(samples,"probe_client_total_ns",99),"probe_client_delivery_p99_ns":quantile(samples,"probe_client_delivery_ns",99),"probe_serial_backlog_p99_ns":quantile(samples,"probe_serial_backlog_ns",99),"probe_deadline_resume_p99_ns":quantile(samples,"probe_deadline_resume_ns",99),
        "probe_actual_p99_ns":quantile(samples,"probe_actual_ns",99),"probe_scheduled_p99_ns":quantile(samples,"probe_scheduled_ns",99),
        "probe_placement_observed_p99_ns":quantile(samples,"probe_placement_observed_ns",99),
        "probe_before_entry_p99_ns":quantile(samples,"probe_before_entry_ns",99),
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

fn offer_outcomes(pool: &InstancePool) -> [u64; 3] {
    (0..4).fold([0, 0, 0], |mut total, owner| {
        let p = pool.dispatcher().scheme_service_progress_on(owner).unwrap();
        total[0] += p.peer_withdrawals;
        assert!(!p.pending_offer, "offer slot survived sample drain");
        total[1] += p.peer_entries;
        total[2] += p.source_wake_signals;
        total
    })
}

fn append_work_policy(value: &mut Value, pool: &InstancePool, arm: bool) {
    value["actor_turn_counts"] = json!(super::settings::actor_turn_counts(pool));
    value["execution_progress_counts"] = json!(super::settings::execution_counts(pool, arm));
    append_client_policy(value, arm);
    value["native_budget_cost_ab"] = json!(super::settings::native_budget_cost_ab());
    value["native_budget_counts"] = json!(super::settings::native_budget_counts(pool, arm));
    value["runtime_event_cost_ab"] = json!(super::settings::runtime_event_cost_ab());
    value["runtime_cost_ab"] = json!(super::settings::runtime_cost_ab());
    value["runtime_joint_ab"] = json!(super::settings::runtime_joint_ab());
    value["runtime_joint_counts"] = json!(super::settings::runtime_counts(pool, arm));
    value["progress_cycle_ab"] = json!(super::settings::progress_cycle_ab());
    value["service_cycle_plan_ab"] = json!(super::settings::service_cycle_plan_ab());
    value["service_cycle_progress_counts"] =
        json!(super::settings::service_cycle_counts(pool, arm));
    value["execution_checkpoint_ab"] = json!(super::settings::execution_checkpoint_ab());
    value["directed_plan_ab"] = json!(super::settings::directed_plan_ab());
    value["registered_plan_ab"] = json!(super::settings::registered_plan_ab());
    value["receiver_plan_ab"] = json!(super::settings::receiver_plan_ab());
    value["reclamation_plan_ab"] = json!(super::settings::reclamation_plan_ab());
    value["specialized_plan_ab"] = json!(super::settings::specialized_plan_ab());
    value["actor_turn_ab"] = json!(super::settings::actor_turn_ab());
    value["cycle_accounting_ab"] = json!(super::settings::cycle_accounting_ab());
    value["activation_plan_ab"] = json!(super::settings::activation_plan_ab());
    value["lease_plan_ab"] = json!(super::settings::lease_plan_ab());
    value["work_plan_ab"] = json!(super::settings::work_plan_ab());
    value["work_plan_protocol_ab"] = json!(super::settings::work_plan_protocol_ab());
    if super::settings::work_plan_ab() {
        let plans = super::settings::plan_counts(pool, arm);
        let total = value["orders"].as_u64().unwrap()
            + u64::try_from(value["probe_actual_ns"].as_array().unwrap().len()).unwrap();
        assert_eq!(
            plans.iter().map(|p| p[2] + p[3]).sum::<u64>(),
            if super::settings::work_enabled(arm) {
                total
            } else {
                0
            }
        );
        value["work_execution_plans"] = json!(plans);
    }
    value["work_budget_ab"] = json!(super::settings::work_budget_ab());
    if super::settings::work_budget_ab() {
        let counts = super::settings::work_counts(pool, arm);
        let probes = u64::try_from(value["probe_actual_ns"].as_array().unwrap().len()).unwrap();
        assert_eq!(
            counts.iter().map(|r| r[1]).sum::<u64>(),
            if super::settings::work_enabled(arm) {
                probes * 4
            } else {
                0
            }
        );
        assert_eq!(
            counts.iter().map(|r| r[2]).sum::<u64>(),
            if super::settings::work_enabled(arm) {
                value["orders"].as_u64().unwrap() * 48
            } else {
                0
            }
        );
        assert_eq!(
            counts.iter().map(|r| r[5]).sum::<u64>(),
            if super::settings::work_enabled(arm) {
                probes
            } else {
                0
            }
        );
        value["executed_work_counts"] = json!(counts);
    }
}

fn append_client_policy(value: &mut Value, arm: bool) {
    value["probe_ready_poll_kind"] = json!(if (super::settings::claim_progress_ab() && arm)
        || super::settings::ready_owner_ab()
        || super::settings::client_turn_ab()
        || (super::settings::arrival_clock_ab() || super::settings::completion_service_ab())
    {
        "combined-claim-final"
    } else {
        "native-final"
    });
    value["arrival_clock_ab"] =
        json!((super::settings::arrival_clock_ab() || super::settings::completion_service_ab()));
    value["arrival_clock_backend"] = json!(if super::settings::arrival_clock_ab() && arm
        || super::settings::completion_service_active(arm)
    {
        "darwin-kqueue-nanoseconds"
    } else {
        "tokio-sleep"
    });
    value["client_turn_ab"] = json!(super::settings::client_turn_ab());
    value["client_progress_opportunities"] = json!(if super::settings::client_turn_ab() && arm
        || super::settings::completion_service_active(arm)
    {
        "signalled-before-each-bulk-wait"
    } else {
        "independent-tokio-tasks"
    });
    value["ready_owner_ab"] = json!(super::settings::ready_owner_ab());
    value["input_preparation_ab"] = json!(super::settings::input_preparation_ab());
    value["supply_cycle_ab"] = json!(super::settings::supply_cycle_ab());
    value["rust_input_preparation"] = json!(if super::settings::supply_cycle_ab() && arm {
        "capacity-window-publication"
    } else if !super::settings::supply_cycle_ab()
        && super::settings::completion_service_active(arm)
        && (!super::settings::input_preparation_ab() || arm)
    {
        "bounded-client-service-turns"
    } else {
        "whole-chunk-before-registration"
    });
    value["completion_service_ab"] = json!(super::settings::completion_service_ab());
    value["completion_receive"] = json!(if super::settings::completion_service_active(arm) {
        "one-ready-result-before-coop"
    } else {
        "cooperative-receiver"
    });
    value["bulk_delivery"] = json!(if (super::settings::arrival_clock_ab()
        || super::settings::completion_service_ab())
        || super::settings::client_turn_ab()
        || super::settings::ready_owner_ab() && arm
    {
        "ready-owner-rotation"
    } else {
        "input-fifo"
    });
    value["claim_progress_ab"] = json!(super::settings::claim_progress_ab());
    value["completion_await_ab"] = json!(super::settings::completion_await_ab());
    value["probe_submission_ack"] = json!(if (super::settings::completion_await_ab()
        || super::settings::claim_progress_ab())
        && arm
        || super::settings::ready_owner_ab()
        || super::settings::client_turn_ab()
        || (super::settings::arrival_clock_ab() || super::settings::completion_service_ab())
    {
        "bounded-queue-publication"
    } else {
        "owner-claim"
    });
    value["retire_publication_ab"] = json!(super::settings::retire_publication_ab());
    value["client_arrival_cost_ab"] = json!(super::settings::client_arrival_cost_ab());
}

fn activation_label() -> &'static str {
    if super::settings::supply_cycle_ab() {
        "same VMs/workload/native budgets/kernel clock/joint client service in BOTH arms; whole-source preparation and registration barrier versus capacity-derived preparation windows published while registration progresses"
    } else if super::settings::input_preparation_ab() {
        "same VMs/workload/native budgets/kernel clock/joint client service in BOTH arms; whole-chunk Rust preparation versus count-bounded preparation with client progress at each request boundary"
    } else if super::settings::completion_service_ab() {
        "same original 500us plan/VMs/workload/native budgets; legacy independent clients versus kernel arrival clock with bounded ready completion service at bulk client boundaries"
    } else if super::settings::arrival_clock_ab() {
        "same 500us planned arrivals/native VMs/workload/ready delivery/unified short observer/independent Tokio clients; millisecond Sleep versus one-shot nanosecond-unit kernel timer, always armed"
    } else if super::settings::client_turn_ab() {
        "same ready-owner bulk delivery, unified short observer and native budgets; independent client tasks versus signalled short progress before each bulk service turn"
    } else if super::settings::ready_owner_ab() {
        "same native VM/budget/retirement/legacy timers and unified short observer in BOTH arms; FIFO bulk result reception versus indexed ready-owner rotation"
    } else if super::settings::claim_progress_ab() {
        "same native VM/budget/retire-before-send/legacy timers; separate placement/result waits versus one shared-waker claim/final future preserving early claim progress"
    } else if super::settings::completion_await_ab() {
        "same four VMs/native budget/retirement before send/legacy timers; owner-placement acknowledgement then result versus queue publication then final result only"
    } else if super::settings::retire_publication_ab() {
        "same native VM/policy/work and legacy client timers; immediate send versus per-result retirement before publication, paired short-request send/receive interval trace"
    } else if super::settings::client_arrival_cost_ab() {
        "same native Scheme policy/VMs/work; serial client unconditional timer versus explicit already-due admission, with independent native-ready/client-consumed progress"
    } else if super::settings::native_budget_cost_ab() {
        "native Scheme runnable state and budget/Actor ownership; no shadow context lifecycle versus unobserved ready-first"
    } else if super::settings::runtime_event_cost_ab() {
        "same native VM workload; event-only Runtime with budget-owned grants versus unobserved ready-first"
    } else if super::settings::runtime_cost_ab() {
        "same native VM workload; complete runtime joint protocol and lifecycle cost versus unobserved ready-first"
    } else if super::settings::runtime_joint_ab() {
        "same Scheme lifecycle observations; ready-first versus runnable/lease/external-wait/retirement joint progress"
    } else if super::settings::progress_cycle_ab() {
        "same ready-first cycle; actual logical execution progress gates direct collection, never completion credits"
    } else if super::settings::service_cycle_plan_ab() {
        "same execution debt, plans and counters; checkpoint-first versus ready-first single-pass native harvest"
    } else if super::settings::execution_checkpoint_ab() {
        "same VMs, ABI23 registered plans and broadcast; service-cycle opportunity survives dispatch until first execution receipt"
    } else if super::settings::directed_plan_ab() {
        "same VMs, registered execution debt and work plans; shared broadcast versus private grant notification"
    } else if super::settings::registered_plan_ab() {
        "same VMs and specialized work plans; execution debt survives runnable/registered until first actual grant"
    } else if super::settings::receiver_plan_ab() {
        "same VMs, specialized plans and work bounds; completion priority belongs to an active Scheme harvest receiver"
    } else if super::settings::reclamation_plan_ab() {
        "same VMs, specialized Scheme plans and work bounds; ready short harvest participates in worker grants"
    } else if super::settings::specialized_plan_ab() {
        "same VMs, work bounds and native Scheme payload; activation winds versus shared hooks and continuation-held plans"
    } else if super::settings::actor_turn_ab() {
        "same VMs, bounds and native payload; per-item leases versus Scheme Actor turn ownership with yielding item extents"
    } else if super::settings::cycle_accounting_ab() {
        "same VMs, work and native payload; exit-owned flags versus successful-return accrual and lease-cycle settlement"
    } else {
        "same VMs, work bounds and joint dispatch; lease activation cells versus immutable metadata with independent invocation state"
    }
}

fn append_client_progress(value: &mut Value, probe: &workload::Probes) {
    value["probe_serial_backlog_ns"] = json!(probe.serial_backlog);
    value["probe_before_wait_ns"] = json!(probe.before_wait);
    value["probe_deadline_resume_ns"] = json!(probe.deadline_resume);
    value["probe_client_delivery_ns"] = json!(probe.client_delivery);
    value["probe_client_total_ns"] = json!(probe.client_total);
    value["probe_wait_offset_ns"] = json!(probe.wait_offset);
    value["probe_issue_offset_ns"] = json!(probe.issue_offset);
    value["probe_native_offset_ns"] = json!(probe.native_offset);
    value["probe_observed_offset_ns"] = json!(probe.observed_offset);
    value["probe_arrival_clock_ns"] = json!(probe.arrival_clock);
    value["probe_timer_armed"] = json!(probe.timer_armed);
    value["probe_ready_poll_start_ns"] = json!(probe.ready_poll_start);
    value["probe_ready_poll_end_ns"] = json!(probe.ready_poll_end);
    value["probe_pending_polls"] = json!(probe.pending_polls);
    value["probe_claim_progress"] = json!(probe.claim.iter().zip(&probe.issue_instants).map(|(claim,start)| claim.map(|v| json!({"owner":v.owner,"observed":v.observed.map(|at|at.duration_since(*start).as_nanos()),"claim_polls":v.claim_polls,"final_without_claim":v.final_without_claim,"ready_without_budget":v.ready_without_budget}))).collect::<Vec<_>>());
    value["probe_publication_ns"] = json!(probe.completion.iter().zip(&probe.issue_instants).map(|(trace, start)| trace.as_ref().map(|trace| {
        let t = trace.snapshot();
        json!({"native_ready":t.native_ready.map(|v|v.duration_since(*start).as_nanos()),"retired":t.retired.map(|v|v.duration_since(*start).as_nanos()),"send_start":t.send_start.map(|v|v.duration_since(*start).as_nanos()),"send_end":t.send_end.map(|v|v.duration_since(*start).as_nanos()),"delivered":t.delivered})
    })).collect::<Vec<_>>());
    for i in 0..probe.actual.len() {
        assert_eq!(
            probe.client_total[i],
            probe.serial_backlog[i]
                + probe.before_wait[i]
                + probe.deadline_resume[i]
                + probe.actual[i]
                + probe.client_delivery[i]
        );
    }
}
