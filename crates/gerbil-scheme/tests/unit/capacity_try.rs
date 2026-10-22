//! Partial cancellation and durable version controls; no native runtime claims.
use crate::instances::capacity::{Capacity, acquire_pair};
use std::{
    future::{Future, poll_fn},
    sync::{Arc, OnceLock},
    task::Poll,
};
use tokio::sync::{Semaphore, watch};
fn capacity() -> (Arc<Capacity>, watch::Receiver<()>) {
    let changed = crate::instances::capacity_event::CapacityEvents::new();
    let receiver = changed.subscribe();
    (
        Arc::new(Capacity {
            changed: Arc::new(OnceLock::from(changed)),
            slots: Arc::new(Semaphore::new(1)),
            slot_limit: 1,
            work: Some(Arc::new(Semaphore::new(8))),
            work_limit: 8,
            producer: Arc::new(Semaphore::new(1)),
        }),
        receiver,
    )
}
#[test]
fn failed_try_releases_partial_work_without_retry_event() {
    let (capacity, changes) = capacity();
    let slot = Arc::clone(&capacity.slots).try_acquire_owned().unwrap();
    assert!(capacity.try_pair(8).is_err());
    assert_eq!(capacity.work.as_ref().unwrap().available_permits(), 8);
    assert!(
        !changes.has_changed().unwrap(),
        "failed scan must not wake itself forever"
    );
    drop(slot);
}
#[test]
fn cancelled_slot_wait_returns_work_and_notifies() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (capacity, mut changes) = capacity();
            let slot = Arc::clone(&capacity.slots).try_acquire_owned().unwrap();
            let mut waiting = Box::pin(acquire_pair(Arc::clone(&capacity), 8));
            poll_fn(|cx| {
                assert!(waiting.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
            assert_eq!(capacity.work.as_ref().unwrap().available_permits(), 0);
            drop(waiting);
            assert_eq!(capacity.work.as_ref().unwrap().available_permits(), 8);
            assert!(changes.has_changed().unwrap());
            changes.mark_unchanged();
            drop(slot);
            let reserved = capacity.try_pair(8).unwrap();
            drop(reserved); // delivery precedes registration of changed(): version persists
            assert!(changes.has_changed().unwrap());
            tokio::time::timeout(std::time::Duration::from_secs(1), changes.changed())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(capacity.slots.available_permits(), 1);
        });
}

#[test]
fn cancelled_partial_work_acquisition_notifies_reselection() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (capacity, changes) = capacity();
            let work = Arc::clone(capacity.work.as_ref().unwrap())
                .try_acquire_many_owned(4)
                .unwrap();
            let mut waiting = Box::pin(acquire_pair(Arc::clone(&capacity), 8));
            poll_fn(|cx| {
                assert!(waiting.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
            assert_eq!(capacity.work.as_ref().unwrap().available_permits(), 0);
            drop(waiting); // Tokio's internal partial acquisition, no full work permit yet
            assert_eq!(capacity.work.as_ref().unwrap().available_permits(), 4);
            assert_eq!(capacity.slots.available_permits(), 1);
            assert!(changes.has_changed().unwrap());
            drop(work);
            assert!(capacity.try_pair(8).is_ok());
        });
}

#[test]
fn version_mark_before_scan_preserves_concurrent_return() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (capacity, mut changes) = capacity();
            let held = capacity.try_pair(8).unwrap();
            // An earlier notification can be discarded only before the live scan.
            capacity.changed.get().unwrap().publish();
            changes.mark_unchanged();
            assert!(capacity.try_pair(8).is_err());
            drop(held); // after scan, before registration of changed()
            tokio::time::timeout(std::time::Duration::from_secs(1), changes.changed())
                .await
                .unwrap()
                .unwrap();
            let returned = capacity.try_pair(8).unwrap();
            drop(returned);
            changes.mark_unchanged(); // return before mark: scan observes real credits
            assert!(capacity.try_pair(8).is_ok());
        });
}

#[test]
fn cancellation_publishes_after_internal_partial_work_is_returned() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::{Context, Wake, Waker};
    struct Probe {
        work: Arc<Semaphore>,
        visible: AtomicUsize,
    }
    impl Wake for Probe {
        fn wake(self: Arc<Self>) {
            self.wake_by_ref();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.visible
                .store(self.work.available_permits(), Ordering::Release);
        }
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (capacity, mut changes) = capacity();
            let work = Arc::clone(capacity.work.as_ref().unwrap());
            let held = Arc::clone(&work).try_acquire_many_owned(4).unwrap();
            let mut acquiring = Box::pin(acquire_pair(Arc::clone(&capacity), 8));
            poll_fn(|cx| {
                assert!(acquiring.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
            let probe = Arc::new(Probe {
                work,
                visible: AtomicUsize::new(usize::MAX),
            });
            let waker = Waker::from(Arc::clone(&probe));
            let mut changed = Box::pin(changes.changed());
            assert!(
                changed
                    .as_mut()
                    .poll(&mut Context::from_waker(&waker))
                    .is_pending()
            );
            drop(acquiring);
            assert_eq!(
                probe.visible.load(Ordering::Acquire),
                4,
                "foreign waker must see restored partial credits during publication"
            );
            drop(held);
        });
}
