//! Native Scheme-owned waiting, continuation affinity and close/error drain.
#[path = "scheme_service/budget.rs"]
mod budget;
#[path = "scheme_service/client_progress.rs"]
mod client_progress;
#[path = "../benches/support/orders.rs"]
mod corpus;
#[path = "scheme_service/deferred.rs"]
mod deferred;
#[path = "scheme_service/elements.rs"]
mod element_snapshots;
#[path = "scheme_service/isolation.rs"]
mod isolation;
#[path = "scheme_service/joint.rs"]
mod joint;
#[path = "scheme_service/native_cycle.rs"]
mod native_cycle;
#[path = "scheme_service/native_entry.rs"]
mod native_entry;
#[path = "../benches/scheme_service/os_attribution.rs"]
mod os_attribution;
#[path = "scheme_service/os_attribution.rs"]
mod os_attribution_tests;
#[path = "scheme_service/pipeline.rs"]
mod pipeline;
#[path = "scheme_service/pool_stream.rs"]
mod pool_stream;
#[path = "scheme_service/prepared.rs"]
mod prepared;
#[path = "scheme_service/priority.rs"]
mod priority;
#[path = "scheme_service/publication.rs"]
mod publication;
#[path = "scheme_service/reselection.rs"]
mod reselection;
#[path = "scheme_service/reselection_contract.rs"]
mod reselection_contract;
#[path = "scheme_service/routing.rs"]
mod routing;
#[path = "scheme_service/runtime_types.rs"]
mod runtime_types;
#[path = "scheme_service/progress.rs"]
mod service_progress;
#[path = "scheme_service/settings.rs"]
mod settings;
#[path = "scheme_service/short.rs"]
mod short;
#[path = "scheme_service/stream.rs"]
mod stream;
#[path = "scheme_service/watchdog.rs"]
mod watchdog;
#[path = "scheme_service/work.rs"]
mod work;
#[path = "scheme_service/work_plan.rs"]
mod work_plan;
use gerbil_scheme::instances::{
    InstanceConfig, InstanceDispatcher, InstanceError, InstancePool, IsolatedProgram, NativeRoot,
    NativeSession, NumericServiceBatch, SchemeServiceLane, SchemeServiceRequest,
};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
use watchdog::GATE;
struct VectorRequest {
    input: Vec<i64>,
    panic: bool,
}
impl SchemeServiceRequest for VectorRequest {
    type Output = Vec<i64>;
    fn lane(&self) -> Option<SchemeServiceLane> {
        Some(SchemeServiceLane(0))
    }
    fn work_units(&self) -> usize {
        1
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        assert_ne!(
            self.input.first(),
            Some(&-1),
            "injected service construction panic"
        );
        session.numeric_vector(&self.input)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        assert!(!self.panic, "injected service projection panic");
        session.copy_numeric(output)
    }
}
fn descriptor(fail: bool) -> IsolatedProgram {
    // SAFETY: same Cargo-owned rooted protocol; failure is an explicit native control.
    unsafe {
        IsolatedProgram::from_image(
            gerbil_scheme_qualification::owner_instance_image(),
            c"gerbil_owner_orders_call",
        )
        .unwrap()
        .with_exports(&[
            c"gerbil_owner_service_observe",
            c"gerbil_owner_snapshot_retarget",
        ])
        .with_scheme_service(if fail {
            c"gerbil_owner_service_fail"
        } else {
            c"gerbil_owner_service_call"
        })
    }
}
fn configure_refill(
    dispatch: &InstanceDispatcher,
    owner: usize,
    policy: gerbil_scheme::instances::SchemeServiceCompletion,
) {
    if policy == gerbil_scheme::instances::SchemeServiceCompletion::PipelineCycle {
        work::enable(dispatch, owner);
        assert!(
            dispatch
                .configure_scheme_service_joint_progress_on(owner, true)
                .unwrap()
                .wait(GATE)
                .unwrap()
        );
        assert!(
            dispatch
                .configure_scheme_service_refill_on(owner, true)
                .unwrap()
                .wait(GATE)
                .unwrap()
        );
    } else {
        assert!(matches!(
            dispatch
                .configure_scheme_service_refill_on(owner, true)
                .unwrap()
                .wait(GATE),
            Err(InstanceError::InvalidConfig(_))
        ));
    }
}
fn config() -> InstanceConfig {
    InstanceConfig {
        owners: 4,
        queue_capacity: 128,
        maximum_request_bytes: 16 * 1024 * 1024,
        maximum_reply_bytes: 16 * 1024 * 1024,
        startup_timeout: GATE,
    }
}
fn observe_command(session: &NativeSession<'_>, operation: i64) -> Result<Vec<i64>, InstanceError> {
    // A prepared-cycle callback can execute while a linear result batch owns
    // another root. This callback must preserve, not release, that custody.
    let before = session.usage()?[3].to_bits();
    let header = session.numeric_vector(&[operation])?;
    let output = session.call_export(c"gerbil_owner_service_observe", &header)?;
    let result = session.copy_numeric::<i64>(&output)?;
    drop(output);
    drop(header);
    session.collect()?;
    assert_eq!(
        session.usage()?[3].to_bits(),
        before,
        "observer callback must preserve pre-existing owner roots"
    );
    Ok(result)
}
fn command(dispatch: &InstanceDispatcher, owner: usize, operation: i64) -> Vec<i64> {
    dispatch
        .submit_native_on(owner, move |session| observe_command(session, operation))
        .unwrap()
        .wait(GATE)
        .unwrap()
}

async fn vector(
    dispatch: &InstanceDispatcher,
    owner: usize,
    input: Vec<i64>,
    panic: bool,
) -> Result<Vec<i64>, InstanceError> {
    dispatch
        .submit_scheme_service_backpressured_on(owner, VectorRequest { input, panic }, GATE)
        .await?
        .wait_completed(GATE)
        .await
        .map(|row| row.0)
}
fn batch(wait: bool) -> NumericServiceBatch<i64> {
    NumericServiceBatch {
        items: (0..8)
            .map(|id| {
                let mut input = corpus::order(id, false);
                if wait {
                    input[1] = 2;
                }
                input
            })
            .collect(),
        yield_every: 1,
    }
}
async fn idle(dispatch: &InstanceDispatcher) {
    let start = Instant::now();
    while !dispatch
        .owner_capacity()
        .iter()
        .all(|c| c.available == c.limit && c.work_available == c.work_limit)
    {
        assert!(start.elapsed() < GATE, "service credits did not return");
        tokio::task::yield_now().await;
    }
}
async fn capabilities(dispatch: &InstanceDispatcher, progress: &mpsc::Sender<&'static str>) {
    for owner in 0..4 {
        let marker = 100 + i64::try_from(owner).unwrap();
        assert_eq!(
            vector(dispatch, owner, vec![-1], false).await.unwrap_err(),
            InstanceError::TaskPanicked
        );
        for (op, expected) in [(1, 23), (2, marker + 7), (3, marker + 9), (4, 31)] {
            if op == 2 || op == 3 {
                budget::toggle_retained(dispatch, owner, op == 3).await;
            }
            assert_eq!(
                vector(dispatch, owner, vec![2, op, marker], false)
                    .await
                    .unwrap(),
                [expected]
            );
            progress
                .send("same Scheme Actor multishot continuation/GC")
                .unwrap();
        }
        assert!(matches!(
            vector(dispatch, owner, vec![11], false).await,
            Err(InstanceError::Scheme(_))
        ));
        let error = vector(dispatch, owner, vec![1], false).await.unwrap_err();
        assert!(matches!(error, InstanceError::Scheme(_)));
        assert_eq!(
            vector(dispatch, owner, vec![5], false).await.unwrap(),
            [1, 1]
        );
        assert_eq!(
            vector(dispatch, owner, vec![0, 1], true).await.unwrap_err(),
            InstanceError::TaskPanicked
        );
        assert_eq!(
            vector(dispatch, owner, vec![0, 1], false).await.unwrap(),
            [10]
        );
        let mut bad = batch(false);
        bad.items[0][3] = -1;
        let failure = dispatch
            .submit_scheme_service_backpressured_on(owner, bad, GATE)
            .await
            .unwrap()
            .wait_completed(GATE)
            .await
            .unwrap_err();
        assert!(matches!(failure, InstanceError::Scheme(_)));
        let input = batch(false);
        let oracle = input.items.clone();
        let (values, _) = dispatch
            .submit_scheme_service_backpressured_on(owner, input, GATE)
            .await
            .unwrap()
            .wait_completed(GATE)
            .await
            .unwrap();
        for (input, value) in oracle.iter().zip(&values) {
            corpus::verify(input, value);
        }
        corpus::peers(&values);
        progress
            .send("service exception and projection panic recovered")
            .unwrap();
    }
    idle(dispatch).await;
}
async fn wake_and_timeout(dispatch: &InstanceDispatcher, progress: &mpsc::Sender<&'static str>) {
    let first = dispatch
        .submit_scheme_service_backpressured_on(0, batch(true), GATE)
        .await
        .unwrap();
    let started = Instant::now();
    while command(dispatch, 0, 10)[0] < 4 {
        assert!(started.elapsed() < GATE);
        tokio::task::yield_now().await;
    }
    let second = dispatch
        .submit_scheme_service_backpressured_on(0, batch(true), GATE)
        .await
        .unwrap();
    drop(second);
    // This finishes while the first Scheme batch is provably waiting on its latch.
    let mut wake_latencies = Vec::new();
    for _ in 0..8 {
        let mut short = batch(false);
        short.items.truncate(1);
        short.items[0].truncate(14);
        let expected = short.items[0].clone();
        let offered = Instant::now();
        let ticket = if wake_latencies.len() % 2 == 0 {
            dispatch
                .submit_scheme_service_backpressured_on(0, short::Short(expected.clone()), GATE)
                .await
        } else {
            dispatch
                .submit_scheme_service_backpressured_on(0, short, GATE)
                .await
        }
        .unwrap();
        let (reply, completed) = ticket.wait_completed(GATE).await.unwrap();
        corpus::verify(&expected, &reply[0]);
        wake_latencies.push(completed.duration_since(offered).as_nanos());
    }
    println!(
        "SERVICE-WAKE-ACCEPTANCE shorts=8 p99-ns={} long-latch-closed=true",
        corpus::percentile(&mut wake_latencies, 99)
    );
    assert!(command(dispatch, 0, 10)[0] >= 4);
    assert_eq!(
        first
            .wait_completed(Duration::from_millis(1))
            .await
            .unwrap_err(),
        InstanceError::Timeout
    );
    let held = dispatch.owner_capacity()[0].clone();
    assert!(held.available <= 2 && held.work_available.unwrap() <= 24);
    command(dispatch, 0, 8);
    idle(dispatch).await;
    progress
        .send("host wake/short request bypasses Scheme wait; timeout keeps work")
        .unwrap();
}
async fn closing_supply(
    dispatch: &InstanceDispatcher,
    owner: usize,
    mode: gerbil_scheme::instances::SchemeServiceSupplyAdmission,
) -> gerbil_scheme::instances::SchemeServiceStream<Vec<Vec<i64>>> {
    command(dispatch, owner, 9);
    let stream = dispatch
        .submit_scheme_service_pool_stream_with_admission(
            (0..4).map(|_| batch(true)).collect(),
            gerbil_scheme::instances::SchemeServicePoolStreamConfig {
                owners: vec![owner],
                maximum_active_per_owner: 2,
                maximum_staged_work: 32,
                timeout: GATE,
            },
            mode,
        )
        .await
        .unwrap();
    let start = Instant::now();
    while command(dispatch, owner, 10)[0] < 8 {
        assert!(start.elapsed() < GATE);
        tokio::task::yield_now().await;
    }
    stream
}
async fn close_full(pool: &mut InstancePool, progress: &mpsc::Sender<&'static str>) {
    let dispatch = pool.dispatcher();
    command(&dispatch, 0, 9);
    let mut tickets = Vec::new();
    for _ in 0..4 {
        tickets.push(
            dispatch
                .submit_scheme_service_backpressured_on(0, batch(true), GATE)
                .await
                .unwrap(),
        );
    }
    let mut closing_stream = closing_supply(
        &dispatch,
        1,
        gerbil_scheme::instances::SchemeServiceSupplyAdmission::Queued,
    )
    .await;
    let mut local_stream = closing_supply(
        &dispatch,
        3,
        gerbil_scheme::instances::SchemeServiceSupplyAdmission::ProgressFair,
    )
    .await;
    let partial_tickets = stream::partial_wait(&dispatch).await;
    let rejected = dispatch
        .submit_scheme_service_backpressured_on(0, batch(false), Duration::from_millis(1))
        .await;
    assert!(matches!(rejected, Err(InstanceError::Timeout)));
    let router = gerbil_scheme::instances::SchemeServiceRouter::with_admission(
        dispatch.clone(),
        gerbil_scheme::instances::SchemeRoutingConfig {
            owners: vec![0],
            short_slots: 1,
            batch_slots: 3,
            short_max_work: 1,
            batch_max_work: 8,
        },
        gerbil_scheme::instances::SchemeRoutingAdmission::Reselect,
    )
    .unwrap();
    let mut waiter = Box::pin(router.submit_short(short::Short(vec![0, 0, 1, 1, 1]), GATE));
    reselection::registered(waiter.as_mut()).await;
    let start = Instant::now();
    let closed = pool.shutdown(GATE);
    assert!(matches!(waiter.await, Err(InstanceError::Closed)));
    reselection::empty(&router);
    assert!(closed.is_clean(), "{closed:?}");
    assert_eq!(closed.joined, 4);
    for stream in [&mut closing_stream, &mut local_stream] {
        for _ in 0..2 {
            assert_eq!(stream.next(GATE).await.unwrap().unwrap().0.len(), 8);
        }
        for _ in 0..2 {
            assert_eq!(
                stream.next(GATE).await.unwrap().unwrap_err(),
                InstanceError::Closed
            );
        }
    }
    for ticket in partial_tickets {
        assert!(!ticket.wait_completed(GATE).await.unwrap().0.is_empty());
    }
    for ticket in tickets {
        let (values, _) = ticket.wait_completed(GATE).await.unwrap();
        assert_eq!(values.len(), 8);
        corpus::peers(&values);
    }
    assert!(matches!(
        dispatch
            .submit_scheme_service_backpressured_on(0, batch(false), GATE)
            .await,
        Err(InstanceError::Closed)
    ));
    assert!(closed.owners.iter().all(|o| o.roots_before_cleanup == 0
        && o.finalizer_status == Some(0)
        && o.image_file_removed));
    println!(
        "SCHEME-SERVICE-DRAIN joined=4 roots=0 files-removed=true native-close={:?}",
        start.elapsed()
    );
    progress
        .send("close releases application latch and drains accepted graphs")
        .unwrap();
}
#[test]
fn scheme_owns_waits_roots_continuations_and_close() {
    if !isolation::isolate_fd_case("scheme_owns_waits_roots_continuations_and_close") {
        qualify_service();
    }
}
fn qualify_service() {
    let (progress, events) = mpsc::channel();
    let watchdog = std::thread::spawn(move || watchdog::observe(&events));
    progress.send("PHASE initialization controls").unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let before = std::fs::read_dir("/dev/fd").unwrap().count();
    // Protocol export cannot be exposed as an ordinary rooted application call.
    let exposed = unsafe { descriptor(false).with_exports(&[c"gerbil_owner_service_call"]) };
    assert!(matches!(
        InstancePool::start(&exposed, config()),
        Err(InstanceError::InvalidConfig(_))
    ));
    let mut small = config();
    small.owners = 1;
    small.queue_capacity = 1;
    let partial = InstancePool::start(&descriptor(false), small).unwrap_err();
    let InstanceError::Startup { shutdown, .. } = partial else {
        panic!("expected partial application-open failure")
    };
    assert!(shutdown.is_clean(), "{shutdown:?}");
    progress
        .send("partial Scheme application-open failure drained")
        .unwrap();
    let failed = InstancePool::start(&descriptor(true), config()).unwrap_err();
    let InstanceError::Startup { shutdown, .. } = failed else {
        panic!("expected service initialization failure")
    };
    assert!(shutdown.is_clean(), "{shutdown:?}");
    progress
        .send("Scheme service initialization failure drained")
        .unwrap();
    progress.send("PHASE auxiliary native contracts").unwrap();
    additional_contracts(&runtime, &progress);
    progress.send("PHASE four-owner main pool startup").unwrap();
    let mut pool = InstancePool::start_observed(&descriptor(false), config(), |_| {
        progress.send("service owner initialized").unwrap();
    })
    .unwrap();
    let states: std::collections::BTreeSet<_> = pool.owners().iter().map(|r| r.state).collect();
    assert_eq!(states.len(), 4);
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    for owner in 0..4 {
        dispatch.configure_scheme_service_feed_on(owner, 4).unwrap();
    }
    budget::configure(&dispatch, &progress);
    for owner in [1, 3] {
        dispatch
            .configure_scheme_service_completion_on(owner, native_cycle::policy(owner))
            .unwrap()
            .wait(GATE)
            .unwrap();
    }
    dispatch
        .configure_scheme_service_native_entry_on(1, true)
        .unwrap()
        .wait(GATE)
        .unwrap();
    let hidden = dispatch
        .submit_native_on(0, |session| {
            let header = session.numeric_vector(&[5_i64])?;
            let input = session.vector(&[&header])?;
            Ok(matches!(
                session.call_export(c"gerbil_owner_service_call", &input),
                Err(InstanceError::Loader(_))
            ))
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert!(
        hidden,
        "service lifecycle/roots must not be exposed by ordinary export lookup"
    );
    let finished_stream = runtime.block_on(async {
        service_progress::capabilities(&dispatch, &progress).await;
        wake_and_timeout(&dispatch, &progress).await;
        short::completion_ready(&dispatch, &progress).await;
        routing::contract(&dispatch, &progress).await;
        reselection::contract(&dispatch, &progress).await;
        reselection_contract::contract(&dispatch).await;
        let finished_stream = stream::contract(&dispatch, &progress).await;
        close_full(&mut pool, &progress).await;
        finished_stream
    });
    drop(pool);
    drop(dispatch);
    assert_eq!(
        std::fs::read_dir("/dev/fd").unwrap().count(),
        before,
        "service pipe/port descriptors leaked"
    );
    drop(finished_stream);
    println!(
        "SCHEME-SERVICE-PASS native-types GC continuation-affinity exceptions backpressure wake drain FD-return"
    );
    drop(progress);
    watchdog.join().unwrap();
}

fn additional_contracts(runtime: &tokio::runtime::Runtime, progress: &mpsc::Sender<&'static str>) {
    progress.send("PHASE deferred projection contract").unwrap();
    deferred::contract(runtime, progress);
    progress
        .send("PHASE single-worker completion contract")
        .unwrap();
    priority::contract(runtime, progress);
    native_cycle::contract(runtime, progress);
    prepared::contract(runtime, progress);
    pipeline::contract(runtime, progress);
    native_entry::contract(runtime, progress);
}

#[path = "scheme_service/owner_claim.rs"]
mod owner_claim;
