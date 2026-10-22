//! Selection hints cannot donate a native turn or release resource credit.
use crate::instances::{Fair, StagedWork, capacity::Capacity, service_wake::Wake};
use std::sync::{Arc, OnceLock, atomic::Ordering};
use tokio::sync::Semaphore;
pub(super) fn capacity() -> Arc<Capacity> {
    Arc::new(Capacity {
        slots: Arc::new(Semaphore::new(4)),
        slot_limit: 4,
        work: Some(Arc::new(Semaphore::new(32))),
        work_limit: 32,
        producer: Arc::new(Semaphore::new(1)),
        changed: Arc::new(OnceLock::new()),
    })
}
#[test]
fn actual_peer_capacity_and_queued_work_replace_wait_marker() {
    let fair = Fair::default();
    let (_r0, w0) = Wake::create().unwrap();
    let (_r1, w1) = Wake::create().unwrap();
    let a0 = Arc::new(Semaphore::new(3));
    let a1 = Arc::new(Semaphore::new(3));
    for (i, w, a) in [(0, &w0, &a0), (1, &w1, &a1)] {
        w.progress.enabled.store(true, Ordering::Release);
        fair.register(i, Arc::downgrade(w), capacity(), Arc::downgrade(a));
    }
    assert!(fair.target(0, 8).is_none());
    w0.progress.charge(24, false);
    let target = fair.target(0, 8).unwrap();
    assert_eq!(target.owner, 1);
    assert!(!w1.progress.waiting_for_claim.load(Ordering::Acquire));
    let held = Arc::clone(&a1).try_acquire_many_owned(3).unwrap();
    assert!(fair.target(0, 8).is_none());
    drop(held);
    let staged = StagedWork::new(Arc::clone(&target.staged), 16);
    assert!(fair.target(0, 8).is_none());
    drop(staged);
    assert!(fair.target(0, 8).is_some());
    w1.closed.store(true, Ordering::Release);
    assert!(fair.target(0, 8).is_none());
}
