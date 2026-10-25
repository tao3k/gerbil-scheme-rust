use super::{Shared, Source};
use crate::instances::{InstanceError, Request, pool::Work};
use std::{
    collections::VecDeque,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};
fn request() -> Request {
    Request {
        started: Arc::new(AtomicBool::new(false)),
        capacity: None,
        work_units: 1,
        work: Work::Wake,
    }
}
#[test]
fn temporary_empty_is_not_terminal_until_seal() {
    let shared = Shared::opening();
    let mut source = Source::Pool(Arc::clone(&shared));
    let cancelled = AtomicBool::new(false);
    assert!(!source.is_empty());
    assert!(source.ready());
    shared
        .publish(
            VecDeque::from([request()]),
            &cancelled,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
    assert!(take(&mut source, 1).is_some());
    assert!(!source.is_empty());
    shared.seal();
    assert!(source.is_empty());
    assert!(matches!(
        shared.publish(
            VecDeque::from([request()]),
            &cancelled,
            Instant::now() + Duration::from_secs(1)
        ),
        Err(InstanceError::Closed)
    ));
}
#[test]
fn rejection_seals_and_prevents_late_publication() {
    let shared = Shared::opening();
    let mut source = Source::Pool(Arc::clone(&shared));
    source.reject(&InstanceError::Timeout);
    assert!(source.is_empty());
    assert!(matches!(
        shared.publish(
            VecDeque::from([request()]),
            &AtomicBool::new(false),
            Instant::now() + Duration::from_secs(1)
        ),
        Err(InstanceError::Timeout)
    ));
    source.reject(&InstanceError::Cancelled);
    assert_eq!(shared.failure.get(), Some(&InstanceError::Timeout));
}
#[test]
fn publication_rechecks_cancel_and_deadline_without_native_credit() {
    let shared = Shared::opening();
    assert!(matches!(
        shared.publish(
            VecDeque::from([request()]),
            &AtomicBool::new(true),
            Instant::now() + Duration::from_secs(1)
        ),
        Err(InstanceError::Cancelled)
    ));
    assert!(matches!(
        shared.publish(
            VecDeque::from([request()]),
            &AtomicBool::new(false),
            Instant::now()
        ),
        Err(InstanceError::Timeout)
    ));
    assert!(shared.requests.lock().unwrap().is_empty());
}
#[test]
fn sealed_published_input_remains_claimable() {
    let shared = Shared::opening();
    shared
        .publish(
            VecDeque::from([request()]),
            &AtomicBool::new(false),
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
    shared.seal();
    let mut source = Source::Pool(shared);
    assert!(!source.is_empty());
    assert!(take(&mut source, 1).is_some());
    assert!(source.is_empty());
}

fn take(source: &mut Source, units: usize) -> Option<Request> {
    let mut wait = crate::instances::service_supply_mutex::SupplyWait::default();
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    let std::task::Poll::Ready(request) = source.poll_take(units, &mut wait, &mut cx) else {
        panic!("uncontended claim deferred");
    };
    request
}
#[test]
fn shared_source_busy_access_returns_to_owner_and_retries_after_unlock() {
    use crate::instances::{service_source_poll::Head, service_supply_mutex::SupplyWait};
    use std::task::{Context, Poll, Waker};
    let shared = Shared::opening();
    let mut source = Source::Pool(Arc::clone(&shared));
    let held = shared.requests.lock().unwrap();
    let mut wait = SupplyWait::default();
    let mut cx = Context::from_waker(Waker::noop());
    assert!(source.poll_head(&mut wait, &mut cx).is_pending());
    assert!(source.poll_take(1, &mut wait, &mut cx).is_pending());
    drop(held);
    assert!(matches!(
        source.poll_head(&mut wait, &mut cx),
        Poll::Ready(Head::Open)
    ));
    shared
        .publish(
            VecDeque::from([request()]),
            &AtomicBool::new(false),
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
    assert!(matches!(
        source.poll_head(&mut wait, &mut cx),
        Poll::Ready(Head::Work(1))
    ));
    assert!(take(&mut source, 1).is_some());
    shared.seal();
    assert!(matches!(
        source.poll_head(&mut wait, &mut cx),
        Poll::Ready(Head::Exhausted)
    ));
}
