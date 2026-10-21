//! Real producer turns share credits, preserve Scheme recovery/continuations and close without host repoll.
#![cfg(target_os = "macos")]
use gerbil_scheme::instances::{
    BatchAdmission, InstanceConfig, InstanceDispatcher, InstanceError, InstancePool,
    IsolatedProgram,
};
use std::{
    future::{Future, poll_fn},
    sync::mpsc,
    task::Poll,
    time::{Duration, Instant},
};
const GATE: Duration = Duration::from_secs(5);
fn admission(timeout: Duration) -> BatchAdmission {
    BatchAdmission {
        owner: 0,
        yield_every: 1,
        timeout,
    }
}
fn batch(seed: i64) -> Vec<Vec<i64>> {
    (0..8).map(|i| vec![seed + i, 0, 0, 2, 3]).collect()
}
fn verify(rows: &[Vec<i64>], seed: i64) {
    assert_eq!(rows.len(), 8);
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row.len(), 21);
        assert_eq!(row[0], seed + i64::try_from(i).unwrap());
        assert_eq!(&row[1..3], &[2, 6]);
        assert_eq!(row[5], 6);
        assert!(row[6..].iter().all(|n| *n == 0));
    }
}
fn idle(dispatch: &InstanceDispatcher) {
    let started = Instant::now();
    loop {
        if dispatch.owner_capacity().iter().all(|s| {
            s.available == s.limit && s.work_available == s.work_limit && s.producer_available == 1
        }) {
            return;
        }
        assert!(started.elapsed() < GATE, "producer credits did not return");
        std::thread::yield_now();
    }
}
async fn registered<F: Future>(future: std::pin::Pin<&mut F>) {
    let mut future = future;
    poll_fn(|cx| {
        assert!(future.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
}
fn blocked(
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
            let result = session.call(&input)?;
            session.copy_numeric::<i64>(&result)
        })
        .unwrap();
    ready.recv_timeout(GATE).unwrap();
    (ticket, release)
}
fn controls(dispatch: &InstanceDispatcher, runtime: &tokio::runtime::Runtime) {
    for turns in [Vec::new(), vec![batch(1); 129], vec![vec![vec![1_i64]; 9]]] {
        assert!(matches!(
            runtime.block_on(
                dispatch.submit_numeric_batch_stream_backpressured(turns, admission(GATE))
            ),
            Err(InstanceError::InvalidConfig(_))
        ));
    }
    assert!(matches!(
        runtime.block_on(
            dispatch
                .submit_numeric_batch_stream_backpressured(vec![batch(1); 128], admission(GATE))
        ),
        Err(InstanceError::InvalidConfig(
            "producer aggregate input bound"
        ))
    ));
    assert!(matches!(
        runtime.block_on(
            dispatch.submit_numeric_batch_stream_backpressured(
                vec![batch(1)],
                admission(Duration::ZERO)
            )
        ),
        Err(InstanceError::Timeout)
    ));
    idle(dispatch);
}
fn recovery_and_gc(dispatch: &InstanceDispatcher, runtime: &tokio::runtime::Runtime) {
    let (head, release) = blocked(dispatch);
    let mut bad = batch(20);
    bad[0][3] = -1;
    let stream = runtime
        .block_on(dispatch.submit_numeric_batch_stream_profiled_backpressured(
            vec![batch(10), bad, batch(30)],
            admission(GATE),
        ))
        .unwrap();
    assert_eq!(dispatch.owner_capacity()[0].producer_available, 0);
    assert!(dispatch.configure_owner_budgets(4, Some(16)).is_err());
    let short = runtime
        .block_on(dispatch.submit_native_backpressured_on(
            0,
            |session| {
                let input = session.numeric_vector(&[2_i64, 1, 100])?;
                let output = session.call(&input)?;
                let value = session.copy_numeric::<i64>(&output)?;
                drop(output);
                drop(input);
                session.collect()?;
                Ok(value)
            },
            GATE,
        ))
        .unwrap();
    let state = &dispatch.owner_capacity()[0];
    assert_eq!((state.available, state.work_available), (2, Some(6)));
    release.send(()).unwrap();
    assert_eq!(runtime.block_on(head.wait_completed(GATE)).unwrap().0, [16]);
    let (value, short_completed) = runtime.block_on(short.wait_completed(GATE)).unwrap();
    assert_eq!(value, [23]);
    let (rows, _) = runtime.block_on(stream.wait_completed(GATE)).unwrap();
    assert_eq!(rows.len(), 3);
    assert!(short_completed <= rows[0].completed.unwrap());
    verify(&rows[0].value.as_ref().unwrap().values, 10);
    assert!(
        matches!(&rows[1].value,Err(InstanceError::Scheme(message)) if message.contains("Scheme actor batch handler failed"))
    );
    assert!(rows[1].completed.is_some());
    verify(&rows[2].value.as_ref().unwrap().values, 30);
    for (operation, expected) in [(2, 107), (3, 109), (4, 31)] {
        let value = dispatch
            .submit_native_on(0, move |session| {
                let input = session.numeric_vector(&[2_i64, operation, 100])?;
                let output = session.call(&input)?;
                let value = session.copy_numeric::<i64>(&output)?;
                drop(output);
                drop(input);
                session.collect()?;
                Ok(value)
            })
            .unwrap()
            .wait(GATE)
            .unwrap();
        assert_eq!(value, [expected]);
    }
    idle(dispatch);
}
fn deadline_and_abandonment(dispatch: &InstanceDispatcher, runtime: &tokio::runtime::Runtime) {
    let (head, release) = blocked(dispatch);
    let stream = runtime
        .block_on(dispatch.submit_numeric_batch_stream_backpressured(
            vec![batch(40)],
            admission(Duration::from_millis(20)),
        ))
        .unwrap();
    std::thread::sleep(Duration::from_millis(25));
    release.send(()).unwrap();
    runtime.block_on(head.wait_completed(GATE)).unwrap();
    let (rows, _) = runtime.block_on(stream.wait_completed(GATE)).unwrap();
    assert_eq!(rows[0].value, Err(InstanceError::Timeout));
    assert!(rows[0].completed.is_none());
    idle(dispatch);
    let stream = runtime
        .block_on(dispatch.submit_numeric_batch_stream_backpressured(
            vec![batch(50), batch(60), batch(70)],
            admission(GATE),
        ))
        .unwrap();
    drop(stream);
    idle(dispatch);
    let stream = runtime
        .block_on(dispatch.submit_numeric_batch_stream_backpressured(
            vec![batch(80), batch(90), batch(100), batch(110)],
            admission(GATE),
        ))
        .unwrap();
    let (rows, _) = runtime.block_on(stream.wait_completed(GATE)).unwrap();
    for (row, seed) in rows[..3].iter().zip([80, 90, 100]) {
        verify(row.value.as_ref().unwrap(), seed);
    }
    assert_eq!(
        rows[3].value,
        Err(InstanceError::InvalidConfig(
            "producer aggregate output bound"
        ))
    );
    assert!(rows[3].completed.is_some());
    idle(dispatch);
}
fn close_without_host_repoll(pool: &mut InstancePool, runtime: &tokio::runtime::Runtime) {
    let dispatch = pool.dispatcher();
    let (head, release) = blocked(&dispatch);
    let stream = runtime
        .block_on(
            dispatch
                .submit_numeric_batch_stream_backpressured(vec![batch(120); 3], admission(GATE)),
        )
        .unwrap();
    runtime.block_on(async {
        let mut waiting = Box::pin(
            dispatch.submit_numeric_batch_stream_backpressured(vec![batch(130)], admission(GATE)),
        );
        registered(waiting.as_mut()).await;
        assert!(!pool.shutdown(Duration::from_millis(1)).is_clean());
        release.send(()).unwrap();
        head.wait_completed(GATE).await.unwrap();
        // Waiting registration remains unpolled while actual owners join.
        let closed = pool.shutdown(GATE);
        assert!(closed.is_clean(), "{closed:?}");
        assert_eq!(closed.joined, 4);
        assert!(
            closed
                .owners
                .iter()
                .all(|o| o.roots_before_cleanup == 0 && o.image_file_removed)
        );
        let (rows, _) = stream.wait_completed(GATE).await.unwrap();
        assert!(
            rows.iter()
                .all(|row| row.completed.is_none()
                    && matches!(row.value, Err(InstanceError::Closed)))
        );
        assert_eq!(waiting.await.unwrap_err(), InstanceError::Closed);
    });
}
#[test]
fn bounded_native_producer_preserves_fences_and_scheme_capabilities() {
    let (progress, events) = mpsc::channel();
    let watchdog = std::thread::spawn(move || {
        while events.recv_timeout(GATE).is_ok() {}
        if matches!(events.try_recv(), Err(mpsc::TryRecvError::Disconnected)) {
            return;
        }
        eprintln!("PRODUCER-STALL cleanup=UNVERIFIED");
        std::process::exit(70);
    });
    // SAFETY: Cargo owns the unchanged application/root ABI and native image.
    let program = unsafe {
        IsolatedProgram::from_image(
            gerbil_scheme_qualification::owner_instance_image(),
            c"gerbil_owner_orders_call",
        )
    }
    .unwrap();
    let mut pool = InstancePool::start_observed(
        &program,
        InstanceConfig {
            owners: 4,
            queue_capacity: 128,
            maximum_request_bytes: 16384,
            maximum_reply_bytes: 4096,
            startup_timeout: GATE,
        },
        |_| progress.send(()).unwrap(),
    )
    .unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(8)).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    controls(&dispatch, &runtime);
    recovery_and_gc(&dispatch, &runtime);
    progress.send(()).unwrap();
    deadline_and_abandonment(&dispatch, &runtime);
    progress.send(()).unwrap();
    close_without_host_repoll(&mut pool, &runtime);
    drop(progress);
    watchdog.join().unwrap();
}
