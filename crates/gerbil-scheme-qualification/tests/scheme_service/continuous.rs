//! Persistent producer: publication, projection and observation are independent.
use super::*;
use gerbil_scheme::instances::{SchemeServiceContinuous, SchemeServiceStreamConfig};

#[test]
fn continuous_window_bounds_unread_results_and_drains_after_seal() {
    let _case = super::native_case();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut settings = config();
    settings.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    runtime.block_on(async {
        let feed: SchemeServiceContinuous<short::Short> = dispatch
            .open_scheme_service_continuous(SchemeServiceStreamConfig {
                owner: 0,
                maximum_active: 3,
                maximum_staged_work: 2,
                timeout: GATE,
            })
            .await
            .unwrap();
        let input = |id| {
            let mut x = corpus::order(id, false);
            x.truncate(14);
            x
        };
        let a = feed
            .try_submit_with(Instant::now() + GATE, || Ok(short::Short(input(1))))
            .unwrap();
        let b = feed
            .try_submit_with(Instant::now() + GATE, || Ok(short::Short(input(2))))
            .unwrap();
        let before = Instant::now();
        // Native retirement must not require observing either Rust result.
        while !a.is_finished() || !b.is_finished() || dispatch.owner_capacity()[0].available != 4 {
            assert!(before.elapsed() < GATE);
            tokio::task::yield_now().await;
        }
        assert!(matches!(
            feed.try_submit_with(Instant::now() + GATE, || panic!("full must not prepare")),
            Err(InstanceError::QueueFull)
        ));
        corpus::verify(&input(1), &a.wait_completed(GATE).await.unwrap().0[0]);
        let c = feed
            .try_submit_with(Instant::now() + GATE, || Ok(short::Short(input(3))))
            .unwrap();
        feed.seal();
        assert!(matches!(
            feed.try_submit_with(Instant::now() + GATE, || panic!("sealed must not prepare")),
            Err(InstanceError::Closed)
        ));
        corpus::verify(&input(2), &b.wait_completed(GATE).await.unwrap().0[0]);
        corpus::verify(&input(3), &c.wait_completed(GATE).await.unwrap().0[0]);
        drop(feed);
        idle(&dispatch).await;
    });
    assert!(pool.shutdown(GATE).is_clean());
}
#[test]
fn continuous_preparation_failure_expiry_and_drop_return_credits() {
    let _case = super::native_case();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut settings = config();
    settings.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    runtime.block_on(async {
        let feed: SchemeServiceContinuous<short::Short> = dispatch
            .open_scheme_service_continuous(SchemeServiceStreamConfig {
                owner: 0,
                maximum_active: 3,
                maximum_staged_work: 2,
                timeout: GATE,
            })
            .await
            .unwrap();
        assert!(matches!(
            feed.try_submit_with(Instant::now() + GATE, || Err(InstanceError::TaskPanicked)),
            Err(InstanceError::TaskPanicked)
        ));
        assert!(matches!(
            feed.try_submit_with(Instant::now(), || panic!("expired must not prepare")),
            Err(InstanceError::Timeout)
        ));
        for id in 0..2 {
            let mut x = corpus::order(id, false);
            x.truncate(14);
            let ticket = feed
                .try_submit_with(Instant::now() + GATE, || Ok(short::Short(x)))
                .unwrap();
            drop(ticket);
        }
        feed.cancel();
        drop(feed);
        idle(&dispatch).await;
    });
    assert!(pool.shutdown(GATE).is_clean());
}
struct Counted {
    input: short::Short,
    constructions: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    fail: bool,
    minimum_before_projection: usize,
}
impl SchemeServiceRequest for Counted {
    type Output = Vec<Vec<i64>>;
    fn short_service(&self) -> bool {
        true
    }
    fn work_units(&self) -> usize {
        1
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.constructions
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.fail {
            Err(InstanceError::TaskPanicked)
        } else {
            self.input.construct(session)
        }
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        assert!(
            self.constructions.load(std::sync::atomic::Ordering::SeqCst)
                >= self.minimum_before_projection
        );
        self.input.project(session, output)
    }
}
#[test]
fn continuous_expired_and_abandoned_inputs_do_not_construct_and_failure_does_not_poison_source() {
    let _case = super::native_case();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut settings = config();
    settings.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    let feed: SchemeServiceContinuous<Counted> = runtime
        .block_on(
            dispatch.open_scheme_service_continuous(SchemeServiceStreamConfig {
                owner: 0,
                maximum_active: 3,
                maximum_staged_work: 2,
                timeout: GATE,
            }),
        )
        .unwrap();
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
    let constructions = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let prepare = |fail| {
        let mut x = corpus::order(3, false);
        x.truncate(14);
        Ok(Counted {
            input: short::Short(x),
            constructions: constructions.clone(),
            fail,
            minimum_before_projection: 0,
        })
    };
    let deadline = Instant::now() + Duration::from_millis(20);
    let expired = feed.try_submit_with(deadline, || prepare(false)).unwrap();
    let abandoned = feed
        .try_submit_with(Instant::now() + GATE, || prepare(false))
        .unwrap();
    drop(abandoned);
    runtime.block_on(async {
        while Instant::now() < deadline {
            tokio::task::yield_now().await;
        }
    });
    release.send(()).unwrap();
    blocker.wait(GATE).unwrap();
    runtime.block_on(async {
        assert!(matches!(
            expired.wait_completed(GATE).await,
            Err(InstanceError::Timeout)
        ));
        // Native FIFO barrier ensures both staged inputs have passed their entry checks.
        dispatch
            .submit_native_on(0, |_| Ok(()))
            .unwrap()
            .wait(GATE)
            .unwrap();
        assert_eq!(constructions.load(std::sync::atomic::Ordering::SeqCst), 0);
        let failed = feed
            .try_submit_with(Instant::now() + GATE, || prepare(true))
            .unwrap();
        assert!(matches!(
            failed.wait_completed(GATE).await,
            Err(InstanceError::TaskPanicked)
        ));
        let valid = feed
            .try_submit_with(Instant::now() + GATE, || prepare(false))
            .unwrap();
        valid.wait_completed(GATE).await.unwrap();
        feed.seal();
        drop(feed);
        idle(&dispatch).await;
    });
    assert!(pool.shutdown(GATE).is_clean());
}

#[test]
fn actor_supply_publishes_three_inputs_before_first_projection() {
    let _case = super::native_case();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut settings = config();
    settings.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    dispatch.configure_scheme_service_feed_on(0, 3).unwrap();
    dispatch
        .configure_scheme_service_completion_on(
            0,
            gerbil_scheme::instances::SchemeServiceCompletion::NativeCycle,
        )
        .unwrap()
        .wait(GATE)
        .unwrap();
    let feed: SchemeServiceContinuous<Counted> = runtime
        .block_on(
            dispatch.open_scheme_service_continuous(SchemeServiceStreamConfig {
                owner: 0,
                maximum_active: 3,
                maximum_staged_work: 3,
                timeout: GATE,
            }),
        )
        .unwrap();
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
    let constructions = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut tickets = Vec::new();
    for id in 0..3 {
        let mut x = corpus::order(id, false);
        x.truncate(14);
        tickets.push(
            feed.try_submit_with(Instant::now() + GATE, || {
                Ok(Counted {
                    input: short::Short(x),
                    constructions: constructions.clone(),
                    fail: false,
                    minimum_before_projection: 3,
                })
            })
            .unwrap(),
        );
    }
    release.send(()).unwrap();
    blocker.wait(GATE).unwrap();
    feed.seal();
    runtime.block_on(async {
        for ticket in tickets {
            ticket.wait_completed(GATE).await.unwrap();
        }
        drop(feed);
        idle(&dispatch).await;
    });
    assert!(pool.shutdown(GATE).is_clean());
}

#[test]
fn continuous_backpressure_waits_for_consumption_and_preserves_expiry() {
    let _case = super::native_case();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut settings = config();
    settings.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    runtime.block_on(async {
        let feed: SchemeServiceContinuous<short::Short> = dispatch
            .open_scheme_service_continuous(SchemeServiceStreamConfig {
                owner: 0,
                maximum_active: 3,
                maximum_staged_work: 1,
                timeout: GATE,
            })
            .await
            .unwrap();
        let input = |id| {
            let mut x = corpus::order(id, false);
            x.truncate(14);
            short::Short(x)
        };
        let first = feed
            .submit_with(Instant::now() + GATE, || Ok(input(1)))
            .await
            .unwrap();
        let prepared = std::sync::atomic::AtomicBool::new(false);
        let mut waiting = Box::pin(feed.submit_with(Instant::now() + GATE, || {
            prepared.store(true, std::sync::atomic::Ordering::Release);
            Ok(input(2))
        }));
        std::future::poll_fn(|cx| {
            assert!(std::future::Future::poll(waiting.as_mut(), cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        assert!(!prepared.load(std::sync::atomic::Ordering::Acquire));
        let expired = feed
            .submit_with(Instant::now() + Duration::from_millis(10), || {
                panic!("expired waiter prepared input")
            })
            .await;
        assert!(matches!(expired, Err(InstanceError::Timeout)));
        first.wait_completed(GATE).await.unwrap();
        let second = waiting.await.unwrap();
        assert!(prepared.load(std::sync::atomic::Ordering::Acquire));
        let mut cancelled = Box::pin(feed.submit_with(Instant::now() + GATE, || {
            panic!("cancelled waiter prepared input")
        }));
        std::future::poll_fn(|cx| {
            assert!(std::future::Future::poll(cancelled.as_mut(), cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        drop(cancelled); // No staged input or native roots were acquired.
        second.wait_completed(GATE).await.unwrap();
        let third = feed
            .submit_with(Instant::now() + GATE, || Ok(input(3)))
            .await
            .unwrap();
        let closed = feed.submit_with(Instant::now() + GATE, || {
            panic!("sealed waiter prepared input")
        });
        let mut closed = Box::pin(closed);
        std::future::poll_fn(|cx| {
            assert!(std::future::Future::poll(closed.as_mut(), cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        feed.seal();
        assert!(matches!(closed.await, Err(InstanceError::Closed)));

        third.wait_completed(GATE).await.unwrap();
        drop(feed);
        idle(&dispatch).await;
    });
    assert!(pool.shutdown(GATE).is_clean());
}

#[test]
fn canceled_staged_input_is_withdrawn_before_credit_returns() {
    let _case = super::native_case();
    let runtime = native_runtime();
    for timeout in [false, true] {
        let mut settings = config();
        settings.owners = 1;
        let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
        let dispatch = pool.dispatcher();
        dispatch.configure_owner_budgets(4, Some(32)).unwrap();
        let feed: SchemeServiceContinuous<short::Short> = runtime
            .block_on(
                dispatch.open_scheme_service_continuous(SchemeServiceStreamConfig {
                    owner: 0,
                    maximum_active: 3,
                    maximum_staged_work: 1,
                    timeout: GATE,
                }),
            )
            .unwrap();
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
        let input = || {
            let mut x = corpus::order(7, false);
            x.truncate(14);
            short::Short(x)
        };
        let first = feed
            .try_submit_with(Instant::now() + GATE, || Ok(input()))
            .unwrap();
        runtime.block_on(async {
            if timeout {
                assert!(matches!(
                    first.wait_completed(Duration::from_millis(10)).await,
                    Err(InstanceError::Timeout)
                ));
            } else {
                drop(first);
            }
            let ticket = feed
                .try_submit_with(Instant::now() + GATE, || Ok(input()))
                .expect("canceled queued input must be withdrawn before returning its credit");
            assert!(matches!(
                feed.try_submit_with(Instant::now() + GATE, || panic!("window still bounded")),
                Err(InstanceError::QueueFull)
            ));
            release.send(()).unwrap();
            blocker.wait(GATE).unwrap();
            let value = ticket.wait_completed(GATE).await.unwrap().0;
            corpus::verify(&input().0, &value[0]);
            feed.seal();
            idle(&dispatch).await;
        });
        drop(feed);
        assert!(pool.shutdown(GATE).is_clean());
    }
}

#[test]
fn continuous_publication_rechecks_original_deadline_after_preparation() {
    let _case = super::native_case();
    let runtime = native_runtime();
    let mut settings = config();
    settings.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    runtime.block_on(async {
        let feed: SchemeServiceContinuous<short::Short> = dispatch
            .open_scheme_service_continuous(SchemeServiceStreamConfig {
                owner: 0,
                maximum_active: 3,
                maximum_staged_work: 1,
                timeout: GATE,
            })
            .await
            .unwrap();
        for backpressured in [false, true] {
            let deadline = Instant::now() + Duration::from_millis(5);
            let prepare = || {
                std::thread::sleep(Duration::from_millis(10));
                let mut input = corpus::order(9, false);
                input.truncate(14);
                Ok(short::Short(input))
            };
            let result = if backpressured {
                feed.submit_with(deadline, prepare).await
            } else {
                feed.try_submit_with(deadline, prepare)
            };
            assert!(
                matches!(result, Err(InstanceError::Timeout)),
                "preparation cannot extend publication eligibility"
            );
        }
        let mut input = corpus::order(10, false);
        input.truncate(14);
        let reply = feed
            .submit_with(Instant::now() + GATE, || Ok(short::Short(input.clone())))
            .await
            .unwrap()
            .wait_completed(GATE)
            .await
            .unwrap()
            .0;
        corpus::verify(&input, &reply[0]);
        feed.seal();
        idle(&dispatch).await;
    });
    assert!(pool.shutdown(GATE).is_clean());
}
