//! Admission ordering and retirement controls without a native VM claim.
use crate::instances::{
    InstanceError, SchemeServiceSupplyAdmission,
    capacity::{Capacity, CapacityPermit},
    service_stream_acquire::Acquisition,
};
use std::{
    sync::{Arc, OnceLock},
    task::{Context, Poll, Waker},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, watch};
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
fn completed(
    value: Poll<Result<(OwnedSemaphorePermit, CapacityPermit), InstanceError>>,
) -> (OwnedSemaphorePermit, CapacityPermit) {
    match value {
        Poll::Ready(Ok(pair)) => pair,
        other => panic!("reservation: {other:?}"),
    }
}
#[test]
fn ready_credits_remain_charged_through_guard_lifetime() {
    let (capacity, changes) = capacity();
    let active = Arc::new(Semaphore::new(1));
    let mut request = Acquisition::new(SchemeServiceSupplyAdmission::OwnerLocal);
    let mut context = Context::from_waker(Waker::noop());
    let (active_credit, native_credit) =
        completed(request.poll(&active, &capacity, 8, &mut context));
    let native_credit = native_credit.retain_active(active_credit);
    assert_eq!(capacity.slots.available_permits(), 0);
    assert_eq!(capacity.work.as_ref().unwrap().available_permits(), 0);
    assert_eq!(active.available_permits(), 0);
    drop(native_credit);
    assert_eq!(capacity.slots.available_permits(), 1);
    assert_eq!(capacity.work.as_ref().unwrap().available_permits(), 8);
    assert_eq!(active.available_permits(), 1);
    assert!(changes.has_changed().unwrap());
}
#[test]
fn ready_attempt_does_not_steal_from_an_older_work_waiter() {
    let (capacity, _) = capacity();
    let work = Arc::clone(capacity.work.as_ref().unwrap())
        .try_acquire_many_owned(8)
        .unwrap();
    let active = Arc::new(Semaphore::new(2));
    let mut older = Acquisition::new(SchemeServiceSupplyAdmission::Queued);
    let mut younger = Acquisition::new(SchemeServiceSupplyAdmission::OwnerLocal);
    let mut context = Context::from_waker(Waker::noop());
    assert!(older.poll(&active, &capacity, 8, &mut context).is_pending());
    assert!(
        younger
            .poll(&active, &capacity, 1, &mut context)
            .is_pending()
    );
    drop(work);
    assert!(
        younger
            .poll(&active, &capacity, 1, &mut context)
            .is_pending()
    );
    let pair = completed(older.poll(&active, &capacity, 8, &mut context));
    assert!(
        younger
            .poll(&active, &capacity, 1, &mut context)
            .is_pending()
    );
    drop(pair);
    drop(completed(younger.poll(&active, &capacity, 1, &mut context)));
    assert_eq!(active.available_permits(), 2);
    assert_eq!(capacity.work.as_ref().unwrap().available_permits(), 8);
}
#[test]
fn changed_head_and_cancellation_return_partial_credits() {
    let (capacity, changes) = capacity();
    let slot = Arc::clone(&capacity.slots).try_acquire_owned().unwrap();
    let active = Arc::new(Semaphore::new(1));
    let mut request = Acquisition::new(SchemeServiceSupplyAdmission::OwnerLocal);
    let mut context = Context::from_waker(Waker::noop());
    assert!(
        request
            .poll(&active, &capacity, 8, &mut context)
            .is_pending()
    );
    assert_eq!(capacity.work.as_ref().unwrap().available_permits(), 0);
    assert!(
        request
            .poll(&active, &capacity, 1, &mut context)
            .is_pending()
    );
    assert_eq!(capacity.work.as_ref().unwrap().available_permits(), 7);
    request.clear();
    assert_eq!(capacity.work.as_ref().unwrap().available_permits(), 8);
    assert_eq!(active.available_permits(), 1);
    assert!(changes.has_changed().unwrap());
    drop(slot);
    drop(completed(request.poll(&active, &capacity, 8, &mut context)));
}
#[test]
fn closed_capacity_releases_ready_active_credit() {
    let (capacity, _) = capacity();
    let active = Arc::new(Semaphore::new(1));
    let mut request = Acquisition::new(SchemeServiceSupplyAdmission::OwnerLocal);
    let mut context = Context::from_waker(Waker::noop());
    capacity.close();
    assert!(matches!(
        request.poll(&active, &capacity, 8, &mut context),
        Poll::Ready(Err(InstanceError::Closed))
    ));
    assert_eq!(active.available_permits(), 1);
}

#[test]
fn closing_a_partial_wait_returns_its_active_and_work_credits() {
    let (capacity, changes) = capacity();
    let held = Arc::clone(capacity.work.as_ref().unwrap())
        .try_acquire_many_owned(4)
        .unwrap();
    let active = Arc::new(Semaphore::new(1));
    let mut request = Acquisition::new(SchemeServiceSupplyAdmission::OwnerLocal);
    let mut context = Context::from_waker(Waker::noop());
    assert!(
        request
            .poll(&active, &capacity, 8, &mut context)
            .is_pending()
    );
    assert_eq!(active.available_permits(), 0);
    assert_eq!(capacity.work.as_ref().unwrap().available_permits(), 0);
    capacity.close();
    assert!(matches!(
        request.poll(&active, &capacity, 8, &mut context),
        Poll::Ready(Err(InstanceError::Closed))
    ));
    assert_eq!(active.available_permits(), 1);
    assert_eq!(capacity.work.as_ref().unwrap().available_permits(), 4);
    assert!(changes.has_changed().unwrap());
    drop(held);
}
