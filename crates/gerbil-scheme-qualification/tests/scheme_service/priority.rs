//! Real native queue ordering, bounded retirement and rejected declarations.
use super::*;
struct PriorityRequest {
    id: i64,
    short: bool,
    units: usize,
    affine: bool,
    trace: Option<std::sync::Arc<gerbil_scheme::instances::SchemeServiceEntryTrace>>,
    on_project: Option<Box<dyn FnOnce() + Send>>,
}
impl SchemeServiceRequest for PriorityRequest {
    type Output = Vec<i64>;
    fn entry_trace(
        &self,
    ) -> Option<std::sync::Arc<gerbil_scheme::instances::SchemeServiceEntryTrace>> {
        self.trace.clone()
    }
    fn short_service(&self) -> bool {
        self.short
    }
    fn work_units(&self) -> usize {
        self.units
    }
    fn lane(&self) -> Option<SchemeServiceLane> {
        self.affine.then_some(SchemeServiceLane(0))
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        session.numeric_vector(&[self.id])
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        let result = session.copy_numeric(output)?;
        if let Some(hook) = self.on_project {
            hook();
        }
        Ok(result)
    }
}
pub(super) fn contract(runtime: &tokio::runtime::Runtime, progress: &mpsc::Sender<&'static str>) {
    use gerbil_scheme::instances::SchemeServiceCompletion;
    for policy in [
        SchemeServiceCompletion::Snapshot,
        SchemeServiceCompletion::FairHandoff,
        SchemeServiceCompletion::CoalescedHandoff,
        SchemeServiceCompletion::NativeCycle,
        SchemeServiceCompletion::PreparedCycle,
        SchemeServiceCompletion::PipelineCycle,
    ] {
        contract_policy(runtime, progress, policy);
    }
    coalesced_snapshot(runtime, progress);
}
fn contract_policy(
    runtime: &tokio::runtime::Runtime,
    progress: &mpsc::Sender<&'static str>,
    policy: gerbil_scheme::instances::SchemeServiceCompletion,
) {
    // SAFETY: same build-owned private service ABI, one persistent Scheme worker.
    let program = unsafe {
        IsolatedProgram::from_image(
            gerbil_scheme_qualification::owner_instance_image(),
            c"gerbil_owner_orders_call",
        )
        .unwrap()
        .with_exports(&[c"gerbil_owner_service_observe"])
        .with_scheme_service(c"gerbil_owner_service_single")
    };
    let mut settings = config();
    settings.owners = 1;
    let mut pool = InstancePool::start(&program, settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(8, Some(8)).unwrap();
    assert!(matches!(
        dispatch.scheme_service_observation_on(0),
        Err(InstanceError::InvalidConfig(_))
    ));
    dispatch.observe_scheme_service_on(0).unwrap();
    configure_feed(&dispatch);
    dispatch
        .configure_scheme_service_completion_on(0, policy)
        .unwrap()
        .wait(GATE)
        .unwrap();
    configure_refill(&dispatch, 0, policy);
    native_entry::reject(&dispatch);
    let trace = std::sync::Arc::new(gerbil_scheme::instances::SchemeServiceEntryTrace::default());
    runtime.block_on(order_fixture(&dispatch, &trace, policy));
    check_observation(&dispatch);
    check_entry(&trace);
    runtime.block_on(bulk_fixture(&dispatch, policy));
    if policy != gerbil_scheme::instances::SchemeServiceCompletion::Snapshot {
        runtime.block_on(late_short_fixture(&dispatch));
    }
    let custody = dispatch
        .scheme_service_completion_on(0)
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert_eq!(custody.policy, policy);
    assert!(!custody.token_owned && !custody.opportunity_pending && custody.claimed == 0);
    if matches!(
        policy,
        gerbil_scheme::instances::SchemeServiceCompletion::CoalescedHandoff
            | gerbil_scheme::instances::SchemeServiceCompletion::NativeCycle
    ) {
        assert!(custody.batches > 0 && custody.results >= custody.batches);
    }
    let shutdown = pool.shutdown(GATE);
    assert!(shutdown.is_clean(), "{shutdown:?}");
    assert_eq!(shutdown.joined, 1);
    assert!(
        shutdown
            .owners
            .iter()
            .all(|owner| owner.roots_before_cleanup == 0 && owner.image_file_removed)
    );
    progress.send("native single-worker short/general FIFO, alternating work budget, exception and retirement contract passed").unwrap();
}

async fn order_fixture(
    dispatch: &InstanceDispatcher,
    trace: &std::sync::Arc<gerbil_scheme::instances::SchemeServiceEntryTrace>,
    policy: gerbil_scheme::instances::SchemeServiceCompletion,
) {
    for (units, affine) in [(2, false), (1, true)] {
        let invalid = PriorityRequest {
            id: 0,
            short: true,
            units,
            affine,
            trace: None,
            on_project: None,
        };
        assert!(matches!(
            dispatch
                .submit_scheme_service_backpressured_on(0, invalid, GATE)
                .await,
            Err(InstanceError::InvalidConfig(_))
        ));
    }
    let mut tickets = Vec::new();
    // First handler blocks; general requests arrive before both short requests.
    for (id, short) in [(90, false), (2, false), (4, false), (1, true), (91, true)] {
        tickets.push(
            dispatch
                .submit_scheme_service_backpressured_on(
                    0,
                    PriorityRequest {
                        id,
                        short,
                        units: 1,
                        affine: false,
                        trace: (id == 1).then(|| std::sync::Arc::clone(trace)),
                        on_project: None,
                    },
                    GATE,
                )
                .await
                .unwrap(),
        );
    }
    let start = Instant::now();
    while command(dispatch, 0, 12)[0] < 5 {
        assert!(
            start.elapsed() < GATE,
            "service did not accept queue-order fixture"
        );
        tokio::task::yield_now().await;
    }
    assert_eq!(command(dispatch, 0, 12)[2..], [90]);
    assert!(dispatch.configure_scheme_service_feed_on(0, 1).is_err());
    reject_active_budget(dispatch);
    command(dispatch, 0, 8);
    for (index, ticket) in tickets.into_iter().enumerate() {
        let result = ticket.wait_completed(GATE).await;
        if index == 4 {
            assert!(matches!(result, Err(InstanceError::Scheme(_))));
        } else {
            assert_eq!(result.unwrap().0, [[90, 2, 4, 1][index]]);
        }
    }
    idle(dispatch).await;
    let observed = command(dispatch, 0, 12);
    assert_eq!(
        observed[2..],
        [90, 1, 2, 91, 4],
        "bounded short/general turns must preserve each FIFO"
    );
    assert!(
        observed[1]
            <= if matches!(
                policy,
                gerbil_scheme::instances::SchemeServiceCompletion::CoalescedHandoff
                    | gerbil_scheme::instances::SchemeServiceCompletion::NativeCycle
            ) {
                5
            } else {
                2
            },
        "completion harvest exceeded its policy's bounded custody"
    );
}
async fn bulk_fixture(
    dispatch: &InstanceDispatcher,
    policy: gerbil_scheme::instances::SchemeServiceCompletion,
) {
    command(dispatch, 0, 9);
    command(dispatch, 0, 17);
    let before = command(dispatch, 0, 12)[0];
    let mut tickets = Vec::new();
    for id in [90, 2, 4] {
        tickets.push(
            dispatch
                .submit_scheme_service_backpressured_on(
                    0,
                    PriorityRequest {
                        id,
                        short: false,
                        units: 1,
                        affine: false,
                        trace: None,
                        on_project: None,
                    },
                    GATE,
                )
                .await
                .unwrap(),
        );
    }
    let start = Instant::now();
    while command(dispatch, 0, 12)[0] < before + 3 {
        assert!(start.elapsed() < GATE, "bulk handoff fixture did not enter");
        tokio::task::yield_now().await;
    }
    command(dispatch, 0, 8);
    for (ticket, id) in tickets.into_iter().zip([90, 2, 4]) {
        assert_eq!(ticket.wait_completed(GATE).await.unwrap().0, [id]);
    }
    idle(dispatch).await;
    if policy == gerbil_scheme::instances::SchemeServiceCompletion::FairHandoff {
        assert_eq!(
            command(dispatch, 0, 12)[1],
            1,
            "bulk-only harvest must retain its one-result boundary without short jobs"
        );
    }
}

async fn late_short_fixture(dispatch: &InstanceDispatcher) {
    use std::{
        future::Future,
        task::{Context, Poll, Waker},
    };
    command(dispatch, 0, 9);
    let before = command(dispatch, 0, 12)[0];
    let (send, receive) = mpsc::channel();
    let submit = dispatch.clone();
    let runtime = tokio::runtime::Handle::current();
    let hook = Box::new(move || {
        // This fixture polls Tokio admission from the fixed native owner; it
        // explicitly supplies the caller runtime's timer context, not a VM move.
        let _context = runtime.enter();
        let mut request = std::pin::pin!(submit.submit_scheme_service_backpressured_on(
            0,
            PriorityRequest {
                id: 1,
                short: true,
                units: 1,
                affine: false,
                trace: None,
                on_project: None
            },
            GATE
        ));
        let Poll::Ready(ticket) = request
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        else {
            panic!("fixture has spare native credit; short must publish without blocking");
        };
        send.send(ticket.unwrap()).unwrap();
    });
    let first = dispatch
        .submit_scheme_service_backpressured_on(
            0,
            PriorityRequest {
                id: 90,
                short: false,
                units: 1,
                affine: false,
                trace: None,
                on_project: Some(hook),
            },
            GATE,
        )
        .await
        .unwrap();
    let mut bulk = Vec::new();
    for id in [2, 4] {
        bulk.push(
            dispatch
                .submit_scheme_service_backpressured_on(
                    0,
                    PriorityRequest {
                        id,
                        short: false,
                        units: 1,
                        affine: false,
                        trace: None,
                        on_project: None,
                    },
                    GATE,
                )
                .await
                .unwrap(),
        );
    }
    let start = Instant::now();
    while command(dispatch, 0, 12)[0] < before + 3 {
        assert!(start.elapsed() < GATE);
        tokio::task::yield_now().await;
    }
    command(dispatch, 0, 8);
    assert_eq!(first.wait_completed(GATE).await.unwrap().0, [90]);
    let short = receive.recv_timeout(GATE).unwrap();
    assert_eq!(short.wait_completed(GATE).await.unwrap().0, [1]);
    for (ticket, id) in bulk.into_iter().zip([2, 4]) {
        assert_eq!(ticket.wait_completed(GATE).await.unwrap().0, [id]);
    }
    idle(dispatch).await;
    let starts = command(dispatch, 0, 12);
    assert_eq!(
        starts[starts.len() - 4..],
        [90, 1, 2, 4],
        "short admitted by completion must participate before next bulk selection"
    );
}

fn check_observation(dispatch: &InstanceDispatcher) {
    let observation = dispatch
        .scheme_service_observation_on(0)
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert_eq!(observation.phases[2].calls, 5);
    assert_eq!(observation.phases[3].calls, 5);
    assert!(observation.phases[0].calls > 0);
    assert!(
        observation
            .phases
            .iter()
            .all(|phase| phase.cpu_ns.is_some() && phase.gc_real_ns.is_some())
    );
    let budget = dispatch
        .scheme_service_budget_on(0)
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert_eq!((budget.short_units, budget.bulk_units), (2, 4));
    assert_eq!(budget.suspensions, 1);
    assert_eq!(
        (budget.queued_short, budget.queued_bulk, budget.active),
        (0, 0, 0)
    );
}

fn configure_feed(dispatch: &InstanceDispatcher) {
    assert!(dispatch.configure_scheme_service_feed_on(0, 0).is_err());
    assert!(dispatch.configure_scheme_service_feed_on(0, 9).is_err());
    dispatch.configure_scheme_service_feed_on(0, 4).unwrap();
    dispatch
        .configure_scheme_service_budget_on(0, 1)
        .unwrap()
        .wait(GATE)
        .unwrap();
}

fn reject_active_budget(dispatch: &InstanceDispatcher) {
    assert!(
        dispatch
            .configure_scheme_service_completion_on(
                0,
                gerbil_scheme::instances::SchemeServiceCompletion::FairHandoff
            )
            .unwrap()
            .wait(GATE)
            .is_err()
    );
    assert!(
        dispatch
            .configure_scheme_service_wake_on(
                0,
                gerbil_scheme::instances::SchemeServiceWake::NativePort
            )
            .unwrap()
            .wait(GATE)
            .is_err()
    );
    assert!(
        dispatch
            .configure_scheme_service_budget_on(0, 0)
            .unwrap()
            .wait(GATE)
            .is_err()
    );
}

fn check_entry(trace: &gerbil_scheme::instances::SchemeServiceEntryTrace) {
    let row = trace.snapshot();
    let chain: Vec<_> = [2, 3, 6, 7, 8]
        .map(|index| row.marks[index].unwrap())
        .to_vec();
    assert!(chain.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(row.wake.is_some() && !row.history_lost);
    assert!(row.marks[4].unwrap() <= row.marks[5].unwrap());
}

fn coalesced_snapshot(runtime: &tokio::runtime::Runtime, progress: &mpsc::Sender<&'static str>) {
    let mut config = config();
    config.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), config).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    dispatch
        .configure_scheme_service_completion_on(
            0,
            gerbil_scheme::instances::SchemeServiceCompletion::CoalescedHandoff,
        )
        .unwrap()
        .wait(GATE)
        .unwrap();
    runtime.block_on(async {
        let mut tickets = Vec::new();
        for _ in 0..3 {
            tickets.push(
                dispatch
                    .submit_scheme_service_backpressured_on(0, batch(true), GATE)
                    .await
                    .unwrap(),
            );
        }
        let start = Instant::now();
        while command(&dispatch, 0, 10)[0] < 3 {
            assert!(start.elapsed() < GATE);
            tokio::task::yield_now().await;
        }
        // Scheme releases the latch and drives real work until all three graphs
        // are ready, without harvesting a token or projecting on Rust's behalf.
        command(&dispatch, 0, 18);
        for ticket in tickets {
            let (values, _) = ticket.wait_completed(GATE).await.unwrap();
            assert_eq!(values.len(), 8);
            corpus::peers(&values);
        }
        idle(&dispatch).await;
    });
    let actual = dispatch
        .scheme_service_completion_on(0)
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert_eq!((actual.batches, actual.results), (1, 3));
    assert_eq!(actual.claimed, 0);
    assert_eq!(actual.ready_queued, 0);
    assert!(!actual.token_owned && !actual.opportunity_pending);
    command(&dispatch, 0, 19);
    let carry = dispatch
        .scheme_service_completion_on(0)
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert_eq!((carry.batches, carry.results), (3, 5));
    assert!(!carry.token_owned && !carry.opportunity_pending);
    assert_eq!((carry.claimed, carry.ready_queued), (0, 0));
    let closed = pool.shutdown(GATE);
    assert!(closed.is_clean(), "{closed:?}");
    progress.send("native three-result snapshot uses one linear token, per-result projection, acknowledgement and clean close").unwrap();
}
