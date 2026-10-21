//! Real owner work holds shared capacity through timeout, panic and pool close.
#![cfg(target_os = "macos")]
use gerbil_scheme::instances::{
    InstanceConfig, InstanceDispatcher, InstanceError, InstancePool, IsolatedProgram,
};
use gerbil_scheme_python::{
    owner_dispatch::WindowBudget,
    owner_flow::{FlowControl, owner_flow_backpressured},
};
use std::{
    future::{Future, poll_fn},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    task::Poll,
    time::{Duration, Instant},
};
const GATE: Duration = Duration::from_secs(5);
fn drained(dispatch: &InstanceDispatcher, limit: usize) {
    let started = Instant::now();
    loop {
        let owners = dispatch.owner_capacity();
        assert_eq!(owners.len(), 4);
        if owners.iter().all(|owner| {
            owner.limit == limit
                && owner.available == limit
                && owner.work_available == owner.work_limit
        }) {
            return;
        }
        assert!(
            started.elapsed() < GATE,
            "native completion did not return capacity"
        );
        std::thread::yield_now();
    }
}
async fn registered<F: Future>(future: std::pin::Pin<&mut F>) {
    let mut future = future;
    poll_fn(|context| {
        assert!(future.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
}
fn block_owner(
    dispatch: &InstanceDispatcher,
) -> (
    gerbil_scheme::instances::AsyncInstanceTicket<Vec<i64>>,
    mpsc::Sender<()>,
) {
    let (entered, ready) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let ticket = dispatch
        .submit_native_async_on(0, move |session| {
            entered.send(()).unwrap();
            blocked.recv_timeout(GATE).unwrap();
            let input = session.numeric_vector(&[0_i64, 3])?;
            let output = session.call(&input)?;
            session.copy_numeric::<i64>(&output)
        })
        .unwrap();
    ready.recv_timeout(GATE).unwrap();
    (ticket, release)
}
fn conservation(pool: &InstancePool, runtime: &tokio::runtime::Runtime) {
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_capacity(1).unwrap();
    let zero =
        runtime.block_on(dispatch.submit_native_backpressured_on(0, |_| Ok(()), Duration::ZERO));
    assert_eq!(zero.unwrap_err(), InstanceError::Timeout);
    drained(&dispatch, 1);
    let pending = runtime.block_on(owner_flow_backpressured(
        vec![(0, ())],
        FlowControl {
            budget: WindowBudget {
                released: Instant::now(),
                timeout: Duration::from_millis(1),
            },
            maximum_in_flight: 1,
        },
        |_, ()| {
            std::future::pending::<
                Result<gerbil_scheme::instances::AsyncInstanceTicket<()>, InstanceError>,
            >()
        },
    ));
    assert_eq!(pending[0].as_ref().unwrap_err(), &InstanceError::Timeout);
    drained(&dispatch, 1);
    let (ticket, release) = block_owner(&dispatch);
    assert_eq!(dispatch.owner_capacity()[0].available, 0);
    assert!(matches!(
        dispatch.clone().submit_native_on(0, |_| Ok(())),
        Err(InstanceError::QueueFull)
    ));
    assert!(matches!(
        dispatch.submit_on(0, vec![]),
        Err(InstanceError::QueueFull)
    ));
    assert_eq!(
        runtime
            .block_on(ticket.wait_completed(Duration::from_millis(1)))
            .unwrap_err(),
        InstanceError::Timeout
    );
    assert_eq!(
        dispatch.owner_capacity()[0].available,
        0,
        "awaiter timeout returned native credit early"
    );
    assert_eq!(
        dispatch.configure_owner_capacity(2).unwrap_err(),
        InstanceError::InvalidConfig("owner capacity has outstanding reservations")
    );
    let attempted = Arc::new(AtomicUsize::new(0));
    runtime.block_on(async {
        let counted = Arc::clone(&attempted);
        let abandoned = dispatch.submit_native_backpressured_on(
            0,
            move |_| {
                counted.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
            GATE,
        );
        let mut abandoned = Box::pin(abandoned);
        registered(abandoned.as_mut()).await;
        drop(abandoned);
        let timed = dispatch
            .submit_native_backpressured_on(0, |_| Ok(()), Duration::from_millis(1))
            .await;
        assert_eq!(timed.unwrap_err(), InstanceError::Timeout);
    });
    release.send(()).unwrap();
    drained(&dispatch, 1);
    assert_eq!(attempted.load(Ordering::SeqCst), 0);
    let panic = dispatch
        .submit_native_async_on::<(), _>(0, |_| panic!("injected native capacity panic"))
        .unwrap();
    assert_eq!(
        runtime.block_on(panic.wait_completed(GATE)).unwrap_err(),
        InstanceError::TaskPanicked
    );
    drained(&dispatch, 1);
}
fn work_budgets(pool: &InstancePool, runtime: &tokio::runtime::Runtime) {
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(8)).unwrap();
    for units in [0, 9] {
        let error = runtime
            .block_on(dispatch.submit_native_weighted_backpressured_on(0, units, |_| Ok(()), GATE))
            .unwrap_err();
        assert!(matches!(error, InstanceError::InvalidConfig(_)));
    }
    // Every native batch variant charges item count, including the old APIs.
    let oversized = || vec![vec![0_i64]; 9];
    assert!(matches!(
        dispatch.submit_numeric_batch_on(0, oversized(), 1),
        Err(InstanceError::InvalidConfig(_))
    ));
    assert!(matches!(
        dispatch.submit_numeric_batch_async_on(0, oversized(), 1),
        Err(InstanceError::InvalidConfig(_))
    ));
    assert!(matches!(
        dispatch.submit_numeric_batch_profiled_on(0, oversized(), 1),
        Err(InstanceError::InvalidConfig(_))
    ));
    assert!(matches!(
        dispatch.submit_numeric_batch_profiled_async_on(0, oversized(), 1),
        Err(InstanceError::InvalidConfig(_))
    ));
    let (ticket, release) = block_owner(&dispatch);
    let queued = runtime
        .block_on(dispatch.submit_native_weighted_backpressured_on(0, 7, |_| Ok(()), GATE))
        .unwrap();
    let state = &dispatch.owner_capacity()[0];
    assert_eq!((state.available, state.work_available), (2, Some(0)));
    assert!(matches!(
        dispatch.submit_native_on(0, |_| Ok(())),
        Err(InstanceError::QueueFull)
    ));
    assert!(matches!(
        dispatch.submit_on(0, vec![]),
        Err(InstanceError::QueueFull)
    ));
    // Receiver timeout returns neither slots nor work; no new work executes.
    assert_eq!(
        runtime
            .block_on(queued.wait_completed(Duration::from_millis(1)))
            .unwrap_err(),
        InstanceError::Timeout
    );
    assert_eq!(dispatch.owner_capacity()[0].work_available, Some(0));
    runtime.block_on(async {
        let waiting = dispatch.submit_native_weighted_backpressured_on(0, 8, |_| Ok(()), GATE);
        let mut waiting = Box::pin(waiting);
        registered(waiting.as_mut()).await;
        drop(waiting);
        assert_eq!(
            dispatch
                .submit_native_backpressured_on(0, |_| Ok(()), Duration::from_millis(1))
                .await
                .unwrap_err(),
            InstanceError::Timeout
        );
    });
    assert!(dispatch.configure_owner_budgets(4, Some(16)).is_err());
    release.send(()).unwrap();
    runtime.block_on(ticket.wait_completed(GATE)).unwrap();
    drained(&dispatch, 4);
    assert!(
        dispatch
            .owner_capacity()
            .iter()
            .all(|s| s.work_available == Some(8))
    );
    let panic = runtime
        .block_on(dispatch.submit_native_weighted_backpressured_on(
            0,
            8,
            |_| -> Result<(), InstanceError> { panic!("weighted job panic control") },
            GATE,
        ))
        .unwrap();
    assert_eq!(
        runtime.block_on(panic.wait_completed(GATE)).unwrap_err(),
        InstanceError::TaskPanicked
    );
    drained(&dispatch, 4);
    assert!(
        dispatch
            .owner_capacity()
            .iter()
            .all(|s| s.work_available == Some(8))
    );
    // Partial multi-unit acquisition must be returned when admission is cancelled.
    let (ticket, release) = block_owner(&dispatch);
    runtime.block_on(async {
        let mut waiting =
            Box::pin(dispatch.submit_native_weighted_backpressured_on(0, 8, |_| Ok(()), GATE));
        registered(waiting.as_mut()).await;
        drop(waiting);
    });
    assert_eq!(dispatch.owner_capacity()[0].work_available, Some(7));
    release.send(()).unwrap();
    runtime.block_on(ticket.wait_completed(GATE)).unwrap();
    drained(&dispatch, 4);
    dispatch.configure_owner_capacity(1).unwrap();
}
fn fifo(pool: &InstancePool, runtime: &tokio::runtime::Runtime) {
    let dispatch = pool.dispatcher();
    let (head, release) = block_owner(&dispatch);
    let (order, sequence) = mpsc::channel();
    runtime.block_on(async {
        let first_order = order.clone();
        let first = dispatch.submit_native_backpressured_on(
            0,
            move |_| {
                first_order.send(1).unwrap();
                Ok(1)
            },
            GATE,
        );
        let second = dispatch.submit_native_backpressured_on(
            0,
            move |_| {
                order.send(2).unwrap();
                Ok(2)
            },
            GATE,
        );
        let mut first = Box::pin(first);
        let mut second = Box::pin(second);
        registered(first.as_mut()).await;
        registered(second.as_mut()).await;
        release.send(()).unwrap();
        assert_eq!(head.wait_completed(GATE).await.unwrap().0, [16]);
        let first = first.await.unwrap();
        assert_eq!(first.wait_completed(GATE).await.unwrap().0, 1);
        let second = second.await.unwrap();
        assert_eq!(second.wait_completed(GATE).await.unwrap().0, 2);
    });
    assert_eq!(sequence.try_iter().collect::<Vec<_>>(), [1, 2]);
    drained(&dispatch, 1);
    dispatch.configure_owner_capacity(2).unwrap();
    drained(&dispatch, 2);
}
fn continuations(pool: &InstancePool, runtime: &tokio::runtime::Runtime) {
    let dispatch = pool.dispatcher();
    for operation in 1..=4 {
        let released = Instant::now();
        let jobs = (0..4)
            .map(|owner| {
                (
                    owner,
                    vec![2_i64, operation, 100 + i64::try_from(owner).unwrap()],
                )
            })
            .collect();
        let rows = runtime.block_on(owner_flow_backpressured(
            jobs,
            FlowControl {
                budget: WindowBudget {
                    released,
                    timeout: GATE,
                },
                maximum_in_flight: 8,
            },
            |owner, command| {
                let dispatch = dispatch.clone();
                async move {
                    dispatch
                        .submit_native_backpressured_on(
                            owner,
                            move |session| {
                                let input = session.numeric_vector(&command)?;
                                let output = session.call(&input)?;
                                let values = session.copy_numeric::<i64>(&output)?;
                                drop(output);
                                drop(input);
                                session.collect()?;
                                Ok(values)
                            },
                            GATE.saturating_sub(released.elapsed()),
                        )
                        .await
                }
            },
        ));
        for (owner, row) in rows.into_iter().enumerate() {
            let owner = i64::try_from(owner).unwrap();
            let expected = match operation {
                1 => 23,
                2 => 107 + owner,
                3 => 109 + owner,
                _ => 31,
            };
            assert_eq!(row.unwrap().0, [expected]);
        }
    }
    drained(&dispatch, 2);
}
fn close_waiters(pool: &mut InstancePool, runtime: &tokio::runtime::Runtime) {
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(1, Some(8)).unwrap();
    let (head, release) = block_owner(&dispatch);
    runtime.block_on(async {
        let pending = dispatch.submit_native_weighted_backpressured_on(0, 8, |_| Ok(()), GATE);
        let mut pending = Box::pin(pending);
        registered(pending.as_mut()).await;
        let closing = pool.shutdown(Duration::from_millis(1));
        assert!(!closing.is_clean());
        assert_eq!(pending.await.unwrap_err(), InstanceError::Closed);
        assert!(matches!(
            dispatch.submit_native_async_on(0, |_| Ok(())),
            Err(InstanceError::Closed)
        ));
        release.send(()).unwrap();
        assert_eq!(head.wait_completed(GATE).await.unwrap().0, [16]);
    });
    let closed = pool.shutdown(GATE);
    assert!(closed.is_clean(), "{closed:?}");
    assert_eq!(closed.joined, 4);
    assert!(
        closed
            .owners
            .iter()
            .all(|owner| owner.roots_before_cleanup == 0 && owner.image_file_removed)
    );
}
#[test]
fn shared_credits_survive_timeout_and_close_and_serialize_waiters() {
    let (progress, events) = mpsc::channel();
    let watchdog = std::thread::spawn(move || {
        while events.recv_timeout(GATE).is_ok() {}
        if matches!(events.try_recv(), Err(mpsc::TryRecvError::Disconnected)) {
            return;
        }
        eprintln!("CAPACITY-STALL cleanup=UNVERIFIED");
        std::process::exit(70);
    });
    // SAFETY: Cargo owns the unchanged native root ABI and Scheme fixture.
    let program = unsafe {
        IsolatedProgram::from_image(
            gerbil_scheme_qualification::owner_instance_image(),
            c"gerbil_owner_native_call",
        )
    }
    .unwrap();
    let mut pool = InstancePool::start_observed(
        &program,
        InstanceConfig {
            owners: 4,
            queue_capacity: 128,
            maximum_request_bytes: 16 * 1024 * 1024,
            maximum_reply_bytes: 16 * 1024 * 1024,
            startup_timeout: GATE,
        },
        |_| progress.send(()).unwrap(),
    )
    .unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    for invalid in [0, 129] {
        assert_eq!(
            pool.dispatcher()
                .configure_owner_capacity(invalid)
                .unwrap_err(),
            InstanceError::InvalidConfig("owner capacity must fit the native queue")
        );
    }
    conservation(&pool, &runtime);
    work_budgets(&pool, &runtime);
    progress.send(()).unwrap();
    fifo(&pool, &runtime);
    progress.send(()).unwrap();
    continuations(&pool, &runtime);
    progress.send(()).unwrap();
    close_waiters(&mut pool, &runtime);
    progress.send(()).unwrap();
    drop(runtime);
    drop(progress);
    watchdog.join().unwrap();
}
