//! Public admission cancellation/close fences with a deterministically busy owner.
use super::{GATE, InstancePool, NumericServiceBatch, config, descriptor};
use gerbil_scheme::instances::{InstanceDispatcher, InstanceError};
use std::{future::Future, sync::mpsc, task::Poll, time::Duration};

fn assert_credits(dispatch: &InstanceDispatcher, slots: usize, work: usize) {
    let row = &dispatch.owner_capacity()[0];
    assert_eq!(row.available, slots);
    assert_eq!(row.work_available, Some(work));
}

#[test]
fn pending_service_cancellation_timeout_and_close_return_credits() {
    let _case = super::native_case();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut settings = config();
    settings.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(1, Some(8)).unwrap();
    let (entered, running) = mpsc::sync_channel(1);
    let (release, held) = mpsc::sync_channel(1);
    let blocker = dispatch
        .submit_native_on(0, move |_| {
            entered.send(()).unwrap();
            held.recv_timeout(GATE).unwrap();
            Ok(())
        })
        .unwrap();
    running.recv_timeout(GATE).unwrap();
    assert_credits(&dispatch, 0, 7);
    let request = || NumericServiceBatch {
        items: vec![vec![1_i64, 2, 3]],
        yield_every: 1,
    };
    let mut pending = Box::pin(dispatch.submit_scheme_service_backpressured_on(0, request(), GATE));
    runtime.block_on(std::future::poll_fn(|cx| {
        assert!(pending.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    }));
    assert_credits(&dispatch, 0, 6);
    assert!(matches!(
        dispatch.configure_owner_budgets(2, Some(16)),
        Err(InstanceError::InvalidConfig(_))
    ));
    drop(pending);
    assert_credits(&dispatch, 0, 7);
    assert!(matches!(
        runtime.block_on(dispatch.submit_scheme_service_backpressured_on(
            0,
            request(),
            Duration::from_millis(1)
        )),
        Err(InstanceError::Timeout)
    ));
    assert_credits(&dispatch, 0, 7);
    let mut pending = Box::pin(dispatch.submit_scheme_service_backpressured_on(0, request(), GATE));
    runtime.block_on(std::future::poll_fn(|cx| {
        assert!(pending.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    }));
    // Zero is an explicit nonblocking close observation, not a widened deadline.
    assert_eq!(pool.shutdown(Duration::ZERO).pending, 1);
    assert!(matches!(
        runtime.block_on(pending),
        Err(InstanceError::Closed)
    ));
    release.send(()).unwrap();
    blocker.wait(GATE).unwrap();
    let receipt = pool.shutdown(GATE);
    assert!(receipt.is_clean(), "{receipt:?}");
    assert_eq!(receipt.joined, 1);
    assert!(
        receipt
            .owners
            .iter()
            .all(|owner| owner.roots_before_cleanup == 0)
    );
}

#[test]
fn cached_dispatchers_release_owner_ports_on_close() {
    let _case = super::native_case();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let before = std::fs::read_dir("/dev/fd").unwrap().count();
    let mut settings = config();
    settings.owners = 2;
    let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
    let cached: Vec<_> = (0..8).map(|_| pool.dispatcher()).collect();
    cached[0].configure_owner_budgets(2, Some(8)).unwrap();
    for owner in 0..2 {
        assert_eq!(
            runtime
                .block_on(super::vector(&cached[0], owner, vec![0, 1], false))
                .unwrap(),
            [10]
        );
    }
    let receipt = pool.shutdown(GATE);
    assert!(receipt.is_clean(), "{receipt:?}");
    assert_eq!(receipt.joined, 2);
    drop(pool);
    for dispatch in &cached {
        assert!(dispatch.owner_capacity().is_empty());
        assert!(matches!(
            runtime.block_on(super::vector(dispatch, 0, vec![0, 1], false)),
            Err(InstanceError::Closed)
        ));
    }
    assert!(
        std::fs::read_dir("/dev/fd").unwrap().count() <= before,
        "cached dispatchers retained native descriptors"
    );
}

#[test]
fn concurrent_owner_publication_and_whole_pool_budget_replacement() {
    let _case = super::native_case();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let mut settings = config();
    settings.owners = 2;
    let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(2, Some(16)).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let control = {
        let (dispatch, barrier) = (dispatch.clone(), std::sync::Arc::clone(&barrier));
        std::thread::spawn(move || {
            barrier.wait();
            for step in 0..128 {
                let slots = if step % 2 == 0 { 2 } else { 4 };
                match dispatch.configure_owner_budgets(slots, Some(slots * 8)) {
                    Ok(()) | Err(InstanceError::InvalidConfig(_)) => (),
                    other => panic!("budget replacement: {other:?}"),
                }
                let rows = dispatch.owner_capacity();
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].limit, rows[1].limit);
                assert_eq!(rows[0].work_limit, rows[1].work_limit);
                std::thread::yield_now();
            }
        })
    };
    let tasks: Vec<_> = (0..2)
        .map(|owner| {
            let (dispatch, barrier) = (dispatch.clone(), std::sync::Arc::clone(&barrier));
            runtime.spawn(async move {
                barrier.wait();
                for _ in 0..64 {
                    let submitted = dispatch
                        .submit_scheme_service_backpressured_on(
                            owner,
                            super::VectorRequest {
                                input: vec![0, 1],
                                panic: false,
                            },
                            GATE,
                        )
                        .await;
                    match submitted {
                        Ok(ticket) => {
                            assert_eq!(ticket.wait_completed(GATE).await.unwrap().0, [10]);
                        }
                        // A stale captured capacity is rejected before Scheme entry.
                        Err(InstanceError::Closed) => (),
                        other => {
                            panic!("unexpected publication rejection: {}", other.err().unwrap())
                        }
                    }
                }
            })
        })
        .collect();
    runtime.block_on(async {
        for task in tasks {
            task.await.unwrap();
        }
    });
    control.join().unwrap();
    runtime.block_on(super::idle(&dispatch));
    dispatch.configure_owner_budgets(3, Some(24)).unwrap();
    for row in dispatch.owner_capacity() {
        assert_eq!(
            (row.limit, row.available, row.work_available),
            (3, 3, Some(24))
        );
    }
    let receipt = pool.shutdown(GATE);
    assert!(receipt.is_clean(), "{receipt:?}");
}
