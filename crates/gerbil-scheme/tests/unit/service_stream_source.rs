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
    source.take(units)
}
#[test]
fn shared_source_busy_access_waits_then_observes_open_work_and_seal() {
    let shared = Shared::opening();
    let mut source = std::thread::scope(|scope| {
        let held = shared.requests.lock().unwrap();
        let (started, ready) = std::sync::mpsc::channel();
        let (completed, done) = std::sync::mpsc::channel();
        let shared = Arc::clone(&shared);
        let worker = scope.spawn(move || {
            started.send(()).unwrap();
            let mut source = Source::Pool(shared);
            assert_eq!(source.units(), None);
            assert!(source.take(1).is_none());
            completed.send(()).unwrap();
            source
        });
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(
            done.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        drop(held);
        done.recv_timeout(Duration::from_secs(5)).unwrap();
        worker.join().unwrap()
    });
    assert!(!source.is_empty(), "open empty input is not terminal");
    shared
        .publish(
            VecDeque::from([request()]),
            &AtomicBool::new(false),
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
    assert_eq!(source.units(), Some(1));
    assert!(take(&mut source, 1).is_some());
    shared.seal();
    assert_eq!(source.units(), None);
    assert!(source.is_empty());
}
