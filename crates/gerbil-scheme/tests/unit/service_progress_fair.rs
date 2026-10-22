//! A low-service peer receives an opportunity, never an exclusive reservation.
use crate::instances::{Fair, capacity::Capacity, service_wake::Wake};
use std::sync::{Arc, OnceLock, atomic::Ordering};
use tokio::sync::Semaphore;
fn capacity() -> Arc<Capacity> {
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
fn peer_opportunity_is_bounded_and_never_migrates_a_request() {
    let fair = Fair::default();
    let (_r0, w0) = Wake::create().unwrap();
    let (_r1, w1) = Wake::create().unwrap();
    let a0 = Arc::new(Semaphore::new(3));
    let a1 = Arc::new(Semaphore::new(3));
    for (i, w, a) in [(0, &w0, &a0), (1, &w1, &a1)] {
        w.progress.enabled.store(true, Ordering::Release);
        fair.register(i, Arc::downgrade(w), capacity(), Arc::downgrade(a));
    }
    w0.progress.charge(24, false);
    // Free credits alone cannot identify a peer inside arbitrary native work.
    assert!(fair.allow(0, 8));
    w1.progress.waiting_for_claim.store(true, Ordering::Release);
    // Exhausted local credits are not donated opportunities; no debt is consumed.
    let held = Arc::clone(&a0).try_acquire_many_owned(3).unwrap();
    assert!(fair.allow(0, 8));
    drop(held);
    assert!(!fair.allow(0, 8));
    // The peer may never run: a second attempt must still be allowed.
    assert!(fair.allow(0, 8));
    assert!(fair.allow(1, 8));
    w1.closed.store(true, Ordering::Release);
    assert!(fair.allow(0, 8));
}
