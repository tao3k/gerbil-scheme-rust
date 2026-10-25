//! Same native order computation and projection for synchronous/asynchronous arms.
use super::{GATE, corpus, observation::Mode};
use gerbil_scheme::instances::{
    AsyncInstanceTicket, InstanceDispatcher, InstanceError, NativeRoot, NativeSession,
    NumericServiceBatch, SchemeServiceRequest, SchemeServiceRouter,
};
use gerbil_scheme_python::{
    owner_dispatch::{OwnerResult, WindowBudget},
    owner_flow::{FlowControl, owner_flow_backpressured, owner_flow_ready_backpressured},
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ShortMode {
    Batch,
    Native(Mode),
    Dedicated(Mode),
}
impl ShortMode {
    pub(super) fn native(self) -> bool {
        self != Self::Batch
    }
    fn observed(self) -> bool {
        match self {
            Self::Batch => false,
            Self::Native(mode) | Self::Dedicated(mode) => mode.enabled(),
        }
    }
    fn counters(self) -> bool {
        match self {
            Self::Batch => false,
            Self::Native(mode) | Self::Dedicated(mode) => mode.counters(),
        }
    }
    pub(super) fn dedicated(self) -> bool {
        matches!(self, Self::Dedicated(_))
    }
}
#[derive(Debug)]
pub(super) struct Reply {
    pub values: Vec<Vec<i64>>,
    pub owner: usize,
    pub entered: Instant,
    pub constructed: Instant,
    pub projection: Instant,
    pub diagnostics: Option<[[f64; 6]; 3]>,
    pub snapshot_ns: Option<[u128; 3]>,
    pub entry: Option<Arc<gerbil_scheme::instances::SchemeServiceEntryTrace>>,
    pub completion: Option<Arc<gerbil_scheme::instances::SchemeCompletionTrace>>,
    pub claim: Option<gerbil_scheme::instances::InstanceClaimProgress>,
}
pub(super) struct Orders {
    request: NumericServiceBatch<i64>,
    entered: Option<Instant>,
    constructed: Option<Instant>,
    short_mode: ShortMode,
    construction_before: Option<[f64; 6]>,
    construction_after: Option<[f64; 6]>,
    snapshot_ns: [u128; 3],
    entry: Option<Arc<gerbil_scheme::instances::SchemeServiceEntryTrace>>,
    completion: Option<Arc<gerbil_scheme::instances::SchemeCompletionTrace>>,
    _bulk_permit: Option<tokio::sync::OwnedSemaphorePermit>,
}
impl SchemeServiceRequest for Orders {
    type Output = Reply;
    fn entry_trace(&self) -> Option<Arc<gerbil_scheme::instances::SchemeServiceEntryTrace>> {
        self.entry.clone()
    }
    fn short_service(&self) -> bool {
        self.short_mode.dedicated()
    }
    fn completion_trace(&self) -> Option<Arc<gerbil_scheme::instances::SchemeCompletionTrace>> {
        self.completion.clone()
    }
    fn work_units(&self) -> usize {
        self.request.work_units()
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.entered = Some(Instant::now());
        if self.short_mode.counters() {
            (self.construction_before, self.snapshot_ns[0]) = super::observation::counter(session);
        }
        let value = if self.short_mode.native() {
            assert_eq!(self.request.items.len(), 1);
            let tag = session.numeric_vector(&[77_i64, 1])?;
            let input = session.numeric_vector(&self.request.items[0])?;
            session.vector(&[&tag, &input])?
        } else {
            self.request.construct(session)?
        };
        if self.short_mode.counters() {
            (self.construction_after, self.snapshot_ns[1]) = super::observation::counter(session);
        }
        self.constructed = Some(Instant::now());
        Ok(value)
    }
    fn construct_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &gerbil_scheme::instances::SchemeServiceInput<'_>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        if self.short_mode.native() {
            let value = self.construct(session)?;
            return input.wrap(session, &value);
        }
        self.entered = Some(Instant::now());
        if self.short_mode.counters() {
            (self.construction_before, self.snapshot_ns[0]) = super::observation::counter(session);
        }
        let value = input.numeric_batch(session, &self.request.items, self.request.yield_every)?;
        if self.short_mode.counters() {
            (self.construction_after, self.snapshot_ns[1]) = super::observation::counter(session);
        }
        self.constructed = Some(Instant::now());
        Ok(value)
    }
    fn enter_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &gerbil_scheme::instances::SchemeServiceInput<'_>,
    ) -> Result<gerbil_scheme::instances::SchemeServiceSubmission<'vm>, InstanceError> {
        if self.short_mode.native() || !super::settings::native_entry_ab() {
            let value = self.construct_service(session, input)?;
            return input.submit(session, &value);
        }
        self.entered = Some(Instant::now());
        let receipt = self.request.enter_service(session, input)?;
        self.constructed = Some(Instant::now());
        Ok(receipt)
    }
    fn project(
        mut self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Reply, InstanceError> {
        let projection = Instant::now();
        let diagnostics = if self.short_mode.counters() {
            let (last, cost) = super::observation::counter(session);
            self.snapshot_ns[2] = cost;
            self.construction_before
                .zip(self.construction_after)
                .zip(last)
                .map(|((a, b), c)| [a, b, c])
        } else {
            None
        };
        let values = if self.short_mode.native() {
            vec![session.copy_numeric(output)?]
        } else {
            self.request.project(session, output)?
        };
        Ok(Reply {
            values,
            owner: session.owner_index(),
            entered: self.entered.unwrap(),
            constructed: self.constructed.unwrap(),
            projection,
            diagnostics,
            snapshot_ns: self.short_mode.counters().then_some(self.snapshot_ns),
            entry: self.entry,
            completion: self.completion,
            claim: None,
        })
    }
}
pub(super) fn orders(
    items: Vec<Vec<i64>>,
    short_mode: ShortMode,
    bulk_permit: Option<tokio::sync::OwnedSemaphorePermit>,
) -> Orders {
    Orders {
        request: NumericServiceBatch {
            items,
            yield_every: 1,
        },
        entered: None,
        constructed: None,
        short_mode,
        entry: short_mode
            .observed()
            .then(|| Arc::new(gerbil_scheme::instances::SchemeServiceEntryTrace::default())),
        completion: (super::settings::retire_publication_ab() && short_mode.native())
            .then(|| Arc::new(gerbil_scheme::instances::SchemeCompletionTrace::default())),
        construction_before: None,
        construction_after: None,
        snapshot_ns: [0; 3],
        _bulk_permit: bulk_permit,
    }
}
async fn routed(
    router: &SchemeServiceRouter,
    items: Vec<Vec<i64>>,
    short_mode: ShortMode,
    timeout: Duration,
) -> Result<AsyncInstanceTicket<Reply>, InstanceError> {
    let request = orders(items, short_mode, None);
    let selected = if short_mode.native() {
        router.submit_short(request, timeout).await?
    } else {
        router.submit_batch(request, timeout).await?
    };
    Ok(selected.ticket)
}
pub(super) async fn submit(
    dispatch: InstanceDispatcher,
    owner: usize,
    items: Vec<Vec<i64>>,
    asynchronous: bool,
    short_mode: ShortMode,
    timeout: Duration,
    bulk_permit: Option<tokio::sync::OwnedSemaphorePermit>,
) -> Result<AsyncInstanceTicket<Reply>, InstanceError> {
    let request = orders(items, short_mode, bulk_permit);
    if asynchronous {
        dispatch
            .submit_scheme_service_backpressured_on(owner, request, timeout)
            .await
    } else {
        dispatch
            .submit_native_weighted_backpressured_on(
                owner,
                request.work_units(),
                move |session| {
                    let mut request = request;
                    let payload = request.construct(session)?;
                    let operation = session.numeric_vector(&[6_i64])?;
                    let input = session.vector(&[&operation, &payload])?;
                    let output = session.call_export(c"gerbil_owner_service_reference", &input)?;
                    request.project(session, &output)
                },
                timeout,
            )
            .await
    }
}
pub(super) async fn bulk(
    dispatch: InstanceDispatcher,
    inputs: Arc<Vec<Vec<i64>>>,
    asynchronous: bool,
    released: Instant,
    ready_completion: bool,
    split_pair: bool,
    router: Option<SchemeServiceRouter>,
) -> Vec<OwnerResult<Reply>> {
    let jobs = inputs
        .chunks(8)
        .enumerate()
        .map(|(index, items)| (index / 4 % 4, items.to_vec()))
        .collect();
    let control = FlowControl {
        budget: WindowBudget {
            released,
            timeout: GATE,
        },
        maximum_in_flight: 128,
    };
    // Equal explicit client admission cap in both split-layout arms. This
    // producer cannot occupy all four native request slots with bulk callbacks.
    let limits: Arc<Vec<_>> = Arc::new(
        (0..4)
            .map(|_| Arc::new(tokio::sync::Semaphore::new(3)))
            .collect(),
    );
    let submitter = move |owner: usize, items| {
        let limit = Arc::clone(&limits[owner]);
        let dispatch = dispatch.clone();
        let router = router.clone();
        async move {
            if let Some(router) = router {
                return routed(
                    &router,
                    items,
                    ShortMode::Batch,
                    GATE.saturating_sub(released.elapsed()),
                )
                .await;
            }
            let permit = if split_pair {
                Some(
                    tokio::time::timeout(
                        GATE.saturating_sub(released.elapsed()),
                        limit.acquire_owned(),
                    )
                    .await
                    .map_err(|_| InstanceError::Timeout)?
                    .map_err(|_| InstanceError::Closed)?,
                )
            } else {
                None
            };
            submit(
                dispatch,
                owner,
                items,
                asynchronous,
                ShortMode::Batch,
                GATE.saturating_sub(released.elapsed()),
                permit,
            )
            .await
        }
    };
    if ready_completion {
        owner_flow_ready_backpressured(jobs, control, submitter).await
    } else {
        owner_flow_backpressured(jobs, control, submitter).await
    }
}
#[derive(Default)]
pub(super) struct Probes {
    pub owners: Vec<usize>,
    pub arrival_clock: Vec<Option<[u128; 4]>>,
    pub actual: Vec<u128>,
    pub serial_backlog: Vec<u128>,
    pub before_wait: Vec<u128>,
    pub deadline_resume: Vec<u128>,
    pub client_delivery: Vec<u128>,
    pub client_total: Vec<u128>,
    pub wait_offset: Vec<u128>,
    pub issue_offset: Vec<u128>,
    pub native_offset: Vec<u128>,
    pub observed_offset: Vec<u128>,
    pub timer_armed: Vec<Option<bool>>,
    pub ready_poll_start: Vec<u128>,
    pub ready_poll_end: Vec<u128>,
    pub pending_polls: Vec<u64>,
    pub completion: Vec<Option<Arc<gerbil_scheme::instances::SchemeCompletionTrace>>>,
    pub claim: Vec<Option<gerbil_scheme::instances::InstanceClaimProgress>>,
    pub issue_instants: Vec<Instant>,
    pub scheduled: Vec<u128>,
    pub placement_observed: Vec<u128>,
    pub service: Vec<u128>,
    pub construct: Vec<u128>,
    pub before_entry: Vec<u128>,
    pub diagnostics: Vec<Option<[[f64; 6]; 3]>>,
    pub snapshot_ns: Vec<Option<[u128; 3]>>,
    pub pending: Vec<u128>,
    pub projection: Vec<u128>,
    pub completed: Vec<Instant>,
    pub entry: Vec<Option<serde_json::Value>>,
}
pub(super) async fn probes(
    dispatch: InstanceDispatcher,
    schedule: ProbeSchedule,
    asynchronous: bool,
    released: Instant,
    short_mode: ShortMode,
    router: Option<SchemeServiceRouter>,
    short_pool: Option<gerbil_scheme::instances::SchemeShortPool>,
) -> Probes {
    let mut values = Probes::default();
    let mut clock = super::arrival::Clock::new(schedule.kernel_clock);
    let mut previous_observed = None;
    for id in 0..schedule.count {
        let scheduled = released + Duration::from_micros(u64::try_from(id * 500).unwrap());
        let arrival =
            gerbil_scheme::instances::InstanceArrival::new(scheduled, previous_observed).unwrap();
        let (issue, clock_trace) = if schedule.kernel_clock {
            clock.issue(arrival, scheduled, released).await
        } else if schedule.deadline_aware {
            (arrival.wait_until_due().await.unwrap(), None)
        } else {
            tokio::time::sleep_until(tokio::time::Instant::from_std(scheduled)).await;
            (arrival.issue().unwrap(), None)
        };
        values.arrival_clock.push(clock_trace);
        let start = issue.issued_at();
        let anchor = short_mode.observed().then(super::native_wait::anchor);
        let mut input = corpus::order(100_000 + id, false);
        input.truncate(14);
        let ticket = if let Some(short_pool) = short_pool.as_ref() {
            pooled_probe(
                short_pool,
                schedule.queue_wait,
                (
                    input.clone(),
                    short_mode,
                    GATE.saturating_sub(released.elapsed()),
                ),
            )
            .await
        } else {
            let placed = if let Some(router) = router.as_ref() {
                routed(
                    router,
                    vec![input.clone()],
                    short_mode,
                    GATE.saturating_sub(released.elapsed()),
                )
                .await
            } else {
                submit(
                    dispatch.clone(),
                    id % 4,
                    vec![input.clone()],
                    asynchronous,
                    short_mode,
                    GATE.saturating_sub(released.elapsed()),
                    None,
                )
                .await
            };
            placed.map(ProbeTicket::Placed)
        }
        .unwrap();
        let placement_observed = Instant::now();
        values
            .placement_observed
            .push(placement_observed.duration_since(start).as_nanos());
        let observed = ticket
            .wait_observed(
                issue,
                GATE.saturating_sub(released.elapsed()),
                schedule.ready_service,
            )
            .await
            .unwrap();
        let timing = observed.progress.unwrap();
        let reply = observed.result.unwrap();
        let completed = timing.native_completed;
        previous_observed = Some(timing.client_observed);
        values.client_progress(timing, released);
        values.completion.push(reply.completion.clone());
        values.claim.push(reply.claim);
        corpus::verify(&input, &reply.values[0]);
        values.native_result(&reply, start, completed);
        values
            .scheduled
            .push(completed.saturating_duration_since(scheduled).as_nanos());
        values
            .entry
            .push(super::entry::receipt(&reply, start, anchor.as_ref()));
        values.diagnostics.push(reply.diagnostics);
        values.snapshot_ns.push(reply.snapshot_ns);
        values.completed.push(completed);
        values.owners.push(reply.owner);
    }
    values
}

pub(super) struct ProbeSchedule {
    pub count: usize,
    pub deadline_aware: bool,
    pub kernel_clock: bool,
    pub ready_service: bool,
    pub queue_wait: QueueWait,
}
impl Probes {
    fn native_result(&mut self, reply: &Reply, start: Instant, completed: Instant) {
        self.before_entry
            .push(reply.entered.duration_since(start).as_nanos());
        self.actual.push(completed.duration_since(start).as_nanos());
        self.service
            .push(completed.duration_since(reply.entered).as_nanos());
        self.construct
            .push(reply.constructed.duration_since(reply.entered).as_nanos());
        self.pending.push(
            reply
                .projection
                .duration_since(reply.constructed)
                .as_nanos(),
        );
        self.projection
            .push(completed.duration_since(reply.projection).as_nanos());
    }
    fn client_progress(
        &mut self,
        timing: gerbil_scheme::instances::InstanceClientProgress,
        released: Instant,
    ) {
        self.serial_backlog.push(timing.serial_backlog().as_nanos());
        self.before_wait.push(timing.before_wait().as_nanos());
        self.deadline_resume
            .push(timing.deadline_resume().as_nanos());
        self.client_delivery.push(timing.delivery().as_nanos());
        self.client_total.push(
            timing
                .client_observed
                .duration_since(timing.planned)
                .as_nanos(),
        );
        self.wait_offset
            .push(timing.wait_started.duration_since(released).as_nanos());
        self.issue_offset
            .push(timing.issued.duration_since(released).as_nanos());
        self.native_offset
            .push(timing.native_completed.duration_since(released).as_nanos());
        self.observed_offset
            .push(timing.client_observed.duration_since(released).as_nanos());
        self.timer_armed.push(timing.timer_armed);
        self.ready_poll_start.push(
            timing
                .response_poll_started
                .duration_since(released)
                .as_nanos(),
        );
        self.ready_poll_end.push(
            timing
                .response_poll_ready
                .duration_since(released)
                .as_nanos(),
        );
        self.pending_polls.push(timing.pending_polls);
        self.issue_instants.push(timing.issued);
    }
}

impl ProbeSchedule {
    pub(super) fn for_arm(count: usize, arm: bool) -> Self {
        Self {
            count,
            deadline_aware: super::settings::client_arrival_cost_ab() && arm,
            ready_service: super::settings::completion_service_active(arm),
            kernel_clock: super::settings::arrival_clock_ab() && arm
                || super::settings::completion_service_active(arm),
            queue_wait: if (arm && super::settings::claim_progress_ab())
                || super::settings::ready_owner_ab()
                || super::settings::client_turn_ab()
                || (super::settings::arrival_clock_ab() || super::settings::completion_service_ab())
            {
                QueueWait::Progress
            } else if arm && super::settings::completion_await_ab() {
                QueueWait::Completion
            } else {
                QueueWait::Placement
            },
        }
    }
}

async fn pooled_probe(
    pool: &gerbil_scheme::instances::SchemeShortPool,
    queue_wait: QueueWait,
    request: (Vec<i64>, ShortMode, Duration),
) -> Result<ProbeTicket, gerbil_scheme::instances::InstanceError> {
    let (input, short_mode, timeout) = request;
    let message = orders(vec![input], short_mode, None);
    match queue_wait {
        QueueWait::Progress => pool
            .submit_progress(message, timeout)
            .await
            .map(ProbeTicket::Progress),
        QueueWait::Completion => pool
            .submit_queued(message, timeout)
            .await
            .map(ProbeTicket::Queued),
        QueueWait::Placement => pool
            .submit(message, timeout)
            .await
            .map(|selected| ProbeTicket::Placed(selected.ticket)),
    }
}

#[derive(Clone, Copy)]
pub(super) enum QueueWait {
    Placement,
    Completion,
    Progress,
}

enum ProbeTicket {
    Progress(gerbil_scheme::instances::ProgressSchemeTicket<Reply>),
    Placed(gerbil_scheme::instances::AsyncInstanceTicket<Reply>),
    Queued(gerbil_scheme::instances::QueuedSchemeTicket<Reply>),
}
impl ProbeTicket {
    async fn wait_observed(
        self,
        issue: gerbil_scheme::instances::InstanceIssue,
        timeout: Duration,
        ready_service: bool,
    ) -> Result<
        gerbil_scheme::instances::InstanceObservedResult<Reply>,
        gerbil_scheme::instances::InstanceError,
    > {
        match self {
            Self::Progress(ticket) => {
                let mut received = if ready_service {
                    ticket.wait_observed_ready(issue, timeout).await?
                } else {
                    ticket.wait_observed(issue, timeout).await?
                };
                if let Ok(reply) = &mut received.observed.result {
                    reply.claim = Some(received.claim);
                }
                Ok(received.observed)
            }
            Self::Placed(ticket) => ticket.wait_observed(issue, timeout).await,
            Self::Queued(ticket) => ticket.wait_observed(issue, timeout).await,
        }
    }
}
