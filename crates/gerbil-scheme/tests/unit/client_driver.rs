use super::InstanceClientProgressDriver;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};
struct Probe {
    ready: Arc<AtomicBool>,
    polls: Arc<AtomicUsize>,
    wake: Arc<Mutex<Option<Waker>>>,
}
impl Future for Probe {
    type Output = usize;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<usize> {
        self.polls.fetch_add(1, Ordering::Relaxed);
        *self.wake.lock().unwrap() = Some(cx.waker().clone());
        if self.ready.load(Ordering::Acquire) {
            Poll::Ready(7)
        } else {
            Poll::Pending
        }
    }
}
fn probe() -> Probe {
    Probe {
        ready: Arc::new(AtomicBool::new(false)),
        polls: Arc::new(AtomicUsize::new(0)),
        wake: Arc::new(Mutex::new(None)),
    }
}
fn run(f: impl Future<Output = ()>) {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f);
}
#[test]
fn no_repeated_progress_poll_without_event_then_priority_over_ready_work() {
    run(async {
        let p = probe();
        let ready = Arc::clone(&p.ready);
        let polls = Arc::clone(&p.polls);
        let wake = Arc::clone(&p.wake);
        let mut driver = InstanceClientProgressDriver::new(p);
        for _ in 0..8 {
            driver.wait(std::future::ready(())).await;
        }
        assert_eq!(polls.load(Ordering::Relaxed), 1);
        ready.store(true, Ordering::Release);
        wake.lock().unwrap().as_ref().unwrap().wake_by_ref();
        driver
            .wait(std::future::poll_fn(|_| {
                assert_eq!(polls.load(Ordering::Relaxed), 2);
                Poll::Ready(())
            }))
            .await;
        for _ in 0..8 {
            driver.wait(std::future::ready(())).await;
        }
        assert_eq!(polls.load(Ordering::Relaxed), 2);
        assert_eq!(driver.finish().await, 7);
    });
}
#[test]
fn wake_during_ready_bulk_turn_remains_due_at_next_boundary() {
    run(async {
        let p = probe();
        let ready = Arc::clone(&p.ready);
        let polls = Arc::clone(&p.polls);
        let wake = Arc::clone(&p.wake);
        let mut driver = InstanceClientProgressDriver::new(p);
        driver
            .wait(std::future::poll_fn(|_| {
                ready.store(true, Ordering::Release);
                wake.lock().unwrap().as_ref().unwrap().wake_by_ref();
                Poll::Ready(())
            }))
            .await;
        assert_eq!(polls.load(Ordering::Relaxed), 1);
        driver.wait(std::future::ready(())).await;
        assert_eq!(polls.load(Ordering::Relaxed), 2);
        assert_eq!(driver.finish().await, 7);
    });
}
#[derive(Default)]
struct Count(AtomicUsize);
impl Wake for Count {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}
#[test]
fn latest_parent_waker_and_drop_retirement_are_independent_of_progress() {
    let p = probe();
    let wake = Arc::clone(&p.wake);
    let mut driver = InstanceClientProgressDriver::new(p);
    let a = Arc::new(Count::default());
    let b = Arc::new(Count::default());
    let aw = Waker::from(Arc::clone(&a));
    let bw = Waker::from(Arc::clone(&b));
    let mut wait = Box::pin(driver.wait(std::future::pending::<()>()));
    assert!(
        wait.as_mut()
            .poll(&mut Context::from_waker(&aw))
            .is_pending()
    );
    assert!(
        wait.as_mut()
            .poll(&mut Context::from_waker(&bw))
            .is_pending()
    );
    let signal = wake.lock().unwrap().as_ref().unwrap().clone();
    signal.wake_by_ref();
    assert_eq!(a.0.load(Ordering::Relaxed), 0);
    assert_eq!(b.0.load(Ordering::Relaxed), 1);
    drop(wait);
    drop(driver);
    signal.wake_by_ref();
    assert_eq!(b.0.load(Ordering::Relaxed), 1);
}

#[test]
fn preparation_services_event_before_next_item_and_after_final_item() {
    run(async {
        for trigger in [0, 2] {
            let p = probe();
            let ready = Arc::clone(&p.ready);
            let polls = Arc::clone(&p.polls);
            let wake = Arc::clone(&p.wake);
            let mut driver = InstanceClientProgressDriver::new(p);
            let source = (0..3).map(|i| {
                assert_eq!(
                    polls.load(Ordering::Relaxed),
                    if i > trigger { 2 } else { 1 }
                );
                if i == trigger {
                    ready.store(true, Ordering::Release);
                    wake.lock().unwrap().as_ref().unwrap().wake_by_ref();
                }
                Ok(i)
            });
            let values = driver
                .prepare_bounded(
                    source,
                    3,
                    std::time::Instant::now() + std::time::Duration::from_secs(1),
                )
                .await
                .unwrap();
            assert_eq!(values, vec![0, 1, 2]);
            assert_eq!(polls.load(Ordering::Relaxed), 2);
            assert_eq!(driver.finish().await, 7);
        }
    });
}
struct DropValue(Arc<AtomicUsize>);
impl Drop for DropValue {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}
#[test]
fn preparation_rejects_count_before_iterator_and_drops_partial_error() {
    run(async {
        let calls = Arc::new(AtomicUsize::new(0));
        let drops = Arc::new(AtomicUsize::new(0));
        let mut driver = InstanceClientProgressDriver::new(std::future::pending::<()>());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        let oversized = (0..3).map(|_| {
            calls.fetch_add(1, Ordering::Relaxed);
            Ok(())
        });
        assert!(
            driver
                .prepare_bounded(oversized, 2, deadline)
                .await
                .is_err()
        );
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        let source = (0..4).map(|i| {
            calls.fetch_add(1, Ordering::Relaxed);
            if i == 2 {
                Err(crate::instances::InstanceError::Closed)
            } else {
                Ok(DropValue(Arc::clone(&drops)))
            }
        });
        assert!(driver.prepare_bounded(source, 4, deadline).await.is_err());
        assert_eq!(calls.load(Ordering::Relaxed), 3);
        assert_eq!(drops.load(Ordering::Relaxed), 2);
        let source = (0..1).map(|_| {
            calls.fetch_add(1, Ordering::Relaxed);
            Ok(())
        });
        assert!(matches!(
            driver
                .prepare_bounded(source, 1, std::time::Instant::now())
                .await,
            Err(crate::instances::InstanceError::Timeout)
        ));
        assert_eq!(calls.load(Ordering::Relaxed), 3);
    });
}
