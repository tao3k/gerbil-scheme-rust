//! Open arrivals go directly to the public bounded resident producer API.
use super::super::{
    GATE, arrival, corpus,
    observation::Mode,
    workload::{self, ShortMode},
};
use super::{Config, Construction, Input, Record, ns, record_buffer, signal};
use gerbil_scheme::instances::{InstanceArrival, InstancePool, SchemeServiceContinuousTicket};
use gerbil_scheme::instances::{InstanceError, SchemeServiceStreamConfig};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

static CURRENT_FLOW: std::sync::Mutex<Option<std::sync::Weak<Flow>>> = std::sync::Mutex::new(None);
pub(super) fn report_stalled_flow() {
    let current = CURRENT_FLOW
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .and_then(std::sync::Weak::upgrade);
    if let Some(current) = current {
        current.report_stall();
    }
}
fn register_flow(config: Config, released: Instant, count: usize) -> Option<Arc<Flow>> {
    let flow = config.backpressure().then(|| {
        Arc::new(Flow::new(
            released,
            config.seconds,
            count,
            config.exchange(),
        ))
    });
    if let Some(flow) = &flow {
        *CURRENT_FLOW
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::downgrade(flow));
    }
    flow
}
pub(super) async fn run(
    pool: &InstancePool,
    config: Config,
    progress: mpsc::Sender<&'static str>,
) -> (Vec<Record>, Duration, usize, Duration) {
    let count = config
        .count
        .max(config.rate * usize::try_from(config.seconds).unwrap());
    let mut rows = record_buffer(count);
    let feed = open(pool, config).await;
    let released = Instant::now();
    let mut clock = arrival::Clock::new(true);
    let mut tasks = tokio::task::JoinSet::new();
    let done = Arc::new(AtomicU64::new(0));
    let costs = config.costs().then(|| Arc::new(Costs::default()));
    let flow = register_flow(config, released, count);
    let mut id = 0;
    let planned = |id: usize| {
        released
            + Duration::from_nanos(
                u64::try_from((id as u128) * 1_000_000_000 / config.rate as u128).unwrap(),
            )
    };
    while id < count {
        while let Some(row) = tasks.try_join_next() {
            rows.push(row.unwrap());
        }
        let next = planned(id);
        if next > Instant::now() {
            let timing = InstanceArrival::new(next, None).unwrap();
            let _ = clock.issue(timing, next, released).await;
        }
        for _ in 0..128 {
            if id == count {
                break;
            }
            let plan = planned(id);
            let issued = Instant::now();
            if plan > issued {
                break;
            }
            let input = Input {
                id,
                planned: plan,
                issued,
                short: true,
            };
            let started = Instant::now();
            let prepare =
                || Ok(prepare(id).with_tagged_service(config.construction == Construction::Tagged));
            let ticket = if config.backpressure() {
                feed.submit_with(plan + GATE, prepare).await
            } else {
                feed.try_submit_with(plan + GATE, prepare)
            };
            if let Some(flow) = &flow {
                flow.admission(&ticket, plan);
            }

            match ticket {
                Ok(ticket) => {
                    let (done, progress) = (Arc::clone(&done), progress.clone());
                    tasks.spawn(observe(
                        ticket,
                        input,
                        started,
                        done,
                        progress,
                        released,
                        (costs.clone(), flow.clone()),
                    ));
                }
                Err(error) => rows.push(admission_error(input, config.backpressure(), &error)),
            }
            id += 1;
        }
        if id < count && planned(id) <= Instant::now() {
            tokio::task::yield_now().await;
        }
    }
    let offered_elapsed = released.elapsed();
    feed.seal();
    while let Some(row) = tasks.join_next().await {
        rows.push(row.unwrap());
    }
    drop(feed);
    if let Some(flow) = &flow {
        flow.report();
        *CURRENT_FLOW
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }
    if let Some(costs) = costs {
        costs.report(config.feed);
    }
    assert_eq!(rows.len(), count);
    (rows, released.elapsed(), count, offered_elapsed)
}

fn admission_error(input: Input, backpressure: bool, error: &InstanceError) -> Record {
    let status = if *error == InstanceError::QueueFull {
        1
    } else if backpressure {
        if *error == InstanceError::Timeout {
            3
        } else {
            4
        }
    } else {
        eprintln!("CONTINUOUS-ADMISSION-ERROR id={} error={error:?}", input.id);
        2
    };
    Record {
        total: ns(Instant::now().duration_since(input.planned)),
        queue: 0,
        generator: ns(input.issued.duration_since(input.planned)),
        pre_entry: 0,
        native: 0,
        delivery: 0,
        owner: usize::MAX,
        short: true,
        status,
    }
}

async fn observe(
    ticket: SchemeServiceContinuousTicket<workload::Reply>,
    input: Input,
    started: Instant,
    done: Arc<AtomicU64>,
    progress: mpsc::Sender<&'static str>,
    released: Instant,
    observations: (Option<Arc<Costs>>, Option<Arc<Flow>>),
) -> Record {
    let (costs, flow) = observations;
    let plan = input.planned;
    let issued = input.issued;
    let result = ticket
        .wait_completed((plan + GATE).saturating_duration_since(Instant::now()))
        .await;
    let observed = Instant::now();
    let row = match result {
        Ok((reply, completed)) => {
            if let Some(flow) = &flow {
                flow.consumed(observed);
            }
            if let Some(costs) = costs {
                costs.count.fetch_add(1, Ordering::Relaxed);
                costs.construct.fetch_add(
                    ns(reply.constructed.duration_since(reply.entered)),
                    Ordering::Relaxed,
                );
                costs.progress.fetch_add(
                    ns(reply.projection.duration_since(reply.constructed)),
                    Ordering::Relaxed,
                );
                costs.projection.fetch_add(
                    ns(completed.duration_since(reply.projection)),
                    Ordering::Relaxed,
                );
            }
            let mut expected = corpus::order(input.id, false);
            expected.truncate(14);
            assert_eq!(reply.values.len(), 1);
            corpus::verify(&expected, &reply.values[0]);
            Record {
                total: ns(observed.duration_since(plan)),
                queue: ns(started.duration_since(issued)),
                generator: ns(issued.duration_since(plan)),
                pre_entry: ns(reply.entered.duration_since(started)),
                native: ns(completed.duration_since(reply.entered)),
                delivery: ns(observed.duration_since(completed)),
                owner: reply.owner,
                short: true,
                status: 0,
            }
        }
        Err(error) => {
            if let Some(flow) = &flow {
                flow.failed(observed);
            }
            if flow.is_none() {
                eprintln!("CONTINUOUS-REQUEST-ERROR id={} error={error:?}", input.id);
            }
            Record {
                total: ns(observed.duration_since(plan)),
                queue: 0,
                generator: ns(issued.duration_since(plan)),
                pre_entry: 0,
                native: 0,
                delivery: 0,
                owner: usize::MAX,
                short: true,
                status: if flow.is_some() && error == InstanceError::Timeout {
                    5
                } else {
                    2
                },
            }
        }
    };
    signal(&progress, &done, &row, released);
    row
}

#[derive(Default)]
struct Costs {
    count: AtomicU64,
    construct: AtomicU64,
    progress: AtomicU64,
    projection: AtomicU64,
}
impl Costs {
    fn report(&self, feed: usize) {
        println!(
            "{}",
            serde_json::json!({"event":"capacity_success_costs","diagnostic_only":true,"feed":feed,
            "successful_samples":self.count.load(Ordering::Relaxed),
            "payload_construction_sum_ns":self.construct.load(Ordering::Relaxed),
            "construct_end_to_projection_begin_sum_ns":self.progress.load(Ordering::Relaxed),
            "projection_begin_to_native_complete_sum_ns":self.projection.load(Ordering::Relaxed),
            "progress_interval_is_not_pure_actor_cpu":true,
            "projection_interval_includes_native_cleanup":true})
        );
    }
}

async fn open(
    pool: &InstancePool,
    config: Config,
) -> gerbil_scheme::instances::SchemeServiceContinuous<workload::Orders> {
    pool.dispatcher()
        .open_scheme_service_continuous::<workload::Orders>(SchemeServiceStreamConfig {
            owner: 0,
            maximum_active: config.short_slots,
            maximum_staged_work: 128,
            timeout: Duration::from_secs(config.seconds) + GATE,
        })
        .await
        .unwrap()
}

fn prepare(id: usize) -> workload::Orders {
    let mut value = corpus::order(id, false);
    value.truncate(14);
    workload::orders(vec![value], ShortMode::Dedicated(Mode::Off), None)
}

// These are producer/client observations. Native stages remain separate and no
// admission or completion clock is restarted after an upstream wait.
struct Flow {
    start: Instant,
    end: Instant,
    seconds: u64,
    planned: usize,
    exchange: bool,
    accepted: AtomicU64,
    accepted_by_end: AtomicU64,
    failed_by_end: AtomicU64,
    completed_by_end: AtomicU64,
    accepted_failed_by_end: AtomicU64,
    maximum_wait: AtomicU64,
    admission_errors: [AtomicU64; 3],
    observed: [AtomicU64; 2],
}
impl Flow {
    fn new(start: Instant, seconds: u64, planned: usize, exchange: bool) -> Self {
        Self {
            start,
            end: start + Duration::from_secs(seconds),
            seconds,
            planned,
            exchange,
            accepted: AtomicU64::new(0),
            accepted_by_end: AtomicU64::new(0),
            failed_by_end: AtomicU64::new(0),
            completed_by_end: AtomicU64::new(0),
            accepted_failed_by_end: AtomicU64::new(0),
            maximum_wait: AtomicU64::new(0),
            admission_errors: std::array::from_fn(|_| AtomicU64::new(0)),
            observed: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }
    fn admission<R>(
        &self,
        ticket: &Result<SchemeServiceContinuousTicket<R>, InstanceError>,
        plan: Instant,
    ) {
        let now = Instant::now();
        self.maximum_wait
            .fetch_max(ns(now.saturating_duration_since(plan)), Ordering::Relaxed);
        if ticket.is_ok() {
            self.accepted.fetch_add(1, Ordering::Relaxed);
            if now <= self.end {
                self.accepted_by_end.fetch_add(1, Ordering::Relaxed);
            }
        } else {
            let index = match ticket.as_ref().err().unwrap() {
                InstanceError::QueueFull => 0,
                InstanceError::Timeout => 1,
                _ => 2,
            };
            self.admission_errors[index].fetch_add(1, Ordering::Relaxed);
            if now <= self.end {
                self.failed_by_end.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    fn consumed(&self, now: Instant) {
        self.observed[0].fetch_add(1, Ordering::Relaxed);
        if now <= self.end {
            self.completed_by_end.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn failed(&self, now: Instant) {
        self.observed[1].fetch_add(1, Ordering::Relaxed);
        if now <= self.end {
            self.accepted_failed_by_end.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn report_stall(&self) {
        let accepted = self.accepted.load(Ordering::Relaxed);
        let errors = self
            .admission_errors
            .each_ref()
            .map(|v| v.load(Ordering::Relaxed));
        let observed = self.observed.each_ref().map(|v| v.load(Ordering::Relaxed));
        println!(
            "{}",
            serde_json::json!({"event":"capacity_flow_stall","partial":true,"native_exchange":self.exchange,
            "elapsed_ns":self.start.elapsed().as_nanos(),"planned":self.planned,"api_admitted":accepted,
            "successful_observed":observed[0],"accepted_failed_observed":observed[1],"queue_full":errors[0],
            "admission_timeouts":errors[1],"other_admission_errors":errors[2],
            "upstream_unattempted":u64::try_from(self.planned).unwrap().saturating_sub(accepted+errors.iter().sum::<u64>()),
            "performance_accepted":false,"watchdog_seconds":5,"full_benchmark_completed":false})
        );
    }
    fn report(&self) {
        let admitted = self.accepted_by_end.load(Ordering::Relaxed);
        let failed = self.failed_by_end.load(Ordering::Relaxed);
        let completed = self.completed_by_end.load(Ordering::Relaxed);
        let accepted_failed = self.accepted_failed_by_end.load(Ordering::Relaxed);
        println!(
            "{}",
            serde_json::json!({"event":"capacity_flow","native_exchange":self.exchange,
            "bounded_backpressure":true,"planned_messages":self.planned,"accepted_messages":self.accepted.load(Ordering::Relaxed),
            "accepted_in_planned_window":admitted,"successful_observed_in_planned_window":completed,
            "admitted_per_planned_second":f64::from(u32::try_from(admitted).unwrap())/f64::from(u32::try_from(self.seconds).unwrap()),
            "upstream_failed_in_planned_window":failed,"upstream_pending_at_plan_end":u64::try_from(self.planned).unwrap()-admitted-failed,
            "accepted_not_successfully_observed_at_plan_end":admitted-completed,
            "accepted_failed_in_planned_window":accepted_failed,
            "accepted_unobserved_at_plan_end":admitted-completed-accepted_failed,
            "maximum_plan_to_admission_outcome_ns":self.maximum_wait.load(Ordering::Relaxed),
            "original_deadlines_preserved":true,"defaults_changed":false,"performance_accepted":false})
        );
    }
}
