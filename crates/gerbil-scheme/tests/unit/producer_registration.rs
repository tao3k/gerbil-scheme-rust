use super::acquire;
use crate::instances::InstanceError;
use std::{
    future::Future,
    sync::Arc,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;
fn run(f: impl Future<Output = ()> + Send + 'static) {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            tokio::spawn(f).await.unwrap();
        });
}
#[test]
fn available_registration_does_not_wait_for_exhausted_client_budget() {
    run(async {
        let semaphore = Arc::new(Semaphore::new(1));
        let mut cx = Context::from_waker(Waker::noop());
        while tokio::task::coop::has_budget_remaining() {
            assert!(
                std::pin::pin!(tokio::task::consume_budget())
                    .as_mut()
                    .poll(&mut cx)
                    .is_ready()
            );
        }
        let mut old = Box::pin(Arc::clone(&semaphore).acquire_owned());
        assert!(old.as_mut().poll(&mut cx).is_pending());
        assert_eq!(semaphore.available_permits(), 1);
        drop(old);
        let mut ready = Box::pin(acquire(&semaphore, Instant::now() + Duration::from_secs(1)));
        let Poll::Ready(Ok(permit)) = ready.as_mut().poll(&mut cx) else {
            panic!("available native registration deferred by client budget")
        };
        assert!(!tokio::task::coop::has_budget_remaining());
        drop(permit);
        assert_eq!(semaphore.available_permits(), 1);
    });
}
#[test]
fn contention_preserves_fifo_and_cancellation_returns_granted_permit() {
    run(async {
        let semaphore = Arc::new(Semaphore::new(1));
        let deadline = Instant::now() + Duration::from_secs(1);
        let held = Arc::clone(&semaphore).acquire_owned().await.unwrap();
        let mut a = Box::pin(acquire(&semaphore, deadline));
        let mut b = Box::pin(acquire(&semaphore, deadline));
        {
            let mut cx = Context::from_waker(Waker::noop());
            assert!(a.as_mut().poll(&mut cx).is_pending());
            drop(held);
            assert!(b.as_mut().poll(&mut cx).is_pending());
        }
        drop(a);
        let permit = b.await.unwrap();
        drop(permit);
        assert_eq!(semaphore.available_permits(), 1);
    });
}
#[test]
fn expired_closed_and_waiting_timeout_never_leak_registration() {
    run(async {
        let semaphore = Arc::new(Semaphore::new(1));
        assert!(matches!(
            acquire(&semaphore, Instant::now()).await,
            Err(InstanceError::Timeout)
        ));
        assert_eq!(semaphore.available_permits(), 1);
        let held = Arc::clone(&semaphore).acquire_owned().await.unwrap();
        assert!(matches!(
            acquire(&semaphore, Instant::now() + Duration::from_millis(1)).await,
            Err(InstanceError::Timeout)
        ));
        drop(held);
        assert_eq!(semaphore.available_permits(), 1);
        semaphore.close();
        assert!(matches!(
            acquire(&semaphore, Instant::now() + Duration::from_secs(1)).await,
            Err(InstanceError::Closed)
        ));
    });
}
