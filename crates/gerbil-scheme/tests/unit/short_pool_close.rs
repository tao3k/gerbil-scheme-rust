//! Reproduce publication blocked behind the queue while owner failure closes it.
use super::{Queue, Queued, Source};
use crate::instances::{InstanceDispatcher, InstanceError, Request, pool::Work};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize},
    },
    time::{Duration, Instant},
};
use tokio::sync::{Semaphore, oneshot};
#[test]
fn owner_failure_close_cannot_be_overtaken_by_queued_publication() {
    let dispatch = InstanceDispatcher::new(1, 1, 1);
    let pending = Arc::new(Semaphore::new(1));
    let source = Arc::new(Source {
        admission: Arc::downgrade(&dispatch.admission),
        destinations: Vec::new(),
        queue: Mutex::new(Queue::default()),
        selective: AtomicBool::new(false),
        pending_limit: 1,
        pending: Arc::clone(&pending),
        length: AtomicUsize::new(0),
        next: AtomicUsize::new(0),
    });
    let (assigned, _receiver) = oneshot::channel();
    let queued = Queued {
        id: 0,
        deadline: Instant::now() + Duration::from_secs(1),
        assigned,
        _pending: Arc::clone(&pending).try_acquire_owned().unwrap(),
        request: Request {
            work: Work::Wake,
            started: Arc::new(AtomicBool::new(false)),
            capacity: None,
            work_units: 1,
        },
    };
    let queue = source.queue.lock().unwrap();
    let publish = Arc::clone(&source);
    let thread = std::thread::spawn(move || publish.publish(queued));
    let started = Instant::now();
    loop {
        if dispatch.admission.senders.try_lock().is_err() {
            break;
        }
        assert!(started.elapsed() < Duration::from_secs(1));
        std::thread::yield_now();
    }
    // Source::close marks the semaphore closed before acquiring this queue.
    // Publisher has already passed the first check and is blocked on the queue.
    pending.close();
    drop(queue);
    assert_eq!(thread.join().unwrap(), Err(InstanceError::Closed));
    source.close();
    assert_eq!(source.length.load(std::sync::atomic::Ordering::Acquire), 0);
    assert_eq!(pending.available_permits(), 1);
}
