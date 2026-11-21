//! Real nonblocking pipes, no Scheme/OS latency claims. Notification is not claim.
use super::{Destination, Port, Queue, Queued, Source};
use crate::instances::{
    InstanceDispatcher, InstanceError, Request,
    capacity_event::CapacityEvents,
    pool::Work,
    service_wake::{Reader, Wake},
};
use std::{
    os::fd::AsRawFd,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use tokio::sync::{Semaphore, oneshot};
struct Fixture {
    _dispatch: InstanceDispatcher,
    _inboxes: Vec<mpsc::Receiver<Request>>,
    source: Arc<Source>,
    wakes: Vec<Arc<Wake>>,
    readers: Vec<Reader>,
}
impl Fixture {
    fn new() -> Self {
        let dispatch = InstanceDispatcher::new(1024, 1024, 4);
        let mut readers = Vec::new();
        let mut wakes = Vec::new();
        let inboxes = (0..3)
            .map(|_| {
                let (reader, wake) = Wake::create().unwrap();
                let (send, receive) = mpsc::sync_channel(4);
                dispatch.add_sender(send, Some(Arc::clone(&wake)));
                readers.push(reader);
                wakes.push(wake);
                receive
            })
            .collect();
        dispatch.configure_owner_budgets(4, Some(32)).unwrap();
        let destinations = dispatch
            .admission
            .senders
            .lock()
            .unwrap()
            .owners
            .iter()
            .enumerate()
            .map(|(owner, d)| Destination {
                owner,
                capacity: Arc::clone(d.capacity.as_ref().unwrap()),
                active: Arc::new(Semaphore::new(1)),
                wake: Arc::downgrade(&wakes[owner]),
                waiting: AtomicBool::new(false),
            })
            .collect();
        let source = Arc::new(Source {
            admission: Arc::downgrade(&dispatch.admission),
            destinations,
            queue: Mutex::new(Queue::default()),
            selective: AtomicBool::new(true),
            progress: AtomicBool::new(false),
            pending_limit: 4,
            maximum_active_per_owner: 1,
            pending: Arc::new(Semaphore::new(4)),
            length: AtomicUsize::new(0),
            next: AtomicUsize::new(0),
        });
        dispatch
            .admission
            .capacity_changed
            .get_or_init(CapacityEvents::new)
            .short_pool
            .set(Arc::downgrade(&source))
            .unwrap();
        for (owner, wake) in wakes.iter().enumerate() {
            wake.short_pool
                .set(Port {
                    owner,
                    index: owner,
                    source: Arc::clone(&source),
                })
                .unwrap();
        }
        Self {
            _dispatch: dispatch,
            _inboxes: inboxes,
            source,
            wakes,
            readers,
        }
    }
    fn port(&self, owner: usize) -> &Port {
        self.wakes[owner].short_pool.get().unwrap()
    }
    fn offer(&self, id: usize) -> oneshot::Receiver<Result<usize, InstanceError>> {
        let (assigned, receiver) = oneshot::channel();
        self.source
            .publish(Queued {
                id,
                deadline: Instant::now() + Duration::from_secs(1),
                assigned: Some(assigned),
                _pending: Arc::clone(&self.source.pending)
                    .try_acquire_owned()
                    .unwrap(),
                request: Request {
                    work: Work::Wake,
                    capacity: None,
                    work_units: 1,
                    started: Arc::new(AtomicBool::new(false)),
                },
            })
            .unwrap();
        receiver
    }
    fn readable(&self, owner: usize) -> bool {
        let mut fd = libc::pollfd {
            fd: self.readers[owner].as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one valid borrowed descriptor; zero-time observation only.
        unsafe { libc::poll(&raw mut fd, 1, 0) > 0 }
    }
}
#[test]
fn fifo_notification_is_bounded_and_does_not_bind_the_payload() {
    let f = Fixture::new();
    for owner in [2, 0, 1] {
        f.port(owner).arm_wait();
    }
    let _first = f.offer(0);
    assert!(f.readable(2));
    assert!(!f.readable(0));
    assert!(!f.readable(1));
    assert_eq!(f.source.queue.lock().unwrap().notified.len(), 1);
    assert!(
        f.source
            .destinations
            .iter()
            .all(|d| d.capacity.slots.available_permits() == 4)
    );
    // An already-running owner can take it; the wake target is not affinity.
    let claimed = f.port(1).claim().unwrap();
    assert_eq!(f.source.length.load(Ordering::Acquire), 0);
    drop(claimed);
    let _second = f.offer(1);
    assert_eq!(
        f.source
            .queue
            .lock()
            .unwrap()
            .notified
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![0]
    );
    f.source.cancel(1);
    assert_eq!(f.source.pending.available_permits(), 4);
    assert!(f.source.queue.lock().unwrap().notified.is_empty());
}
#[test]
fn publication_before_registration_and_interruption_relay_are_lossless() {
    let f = Fixture::new();
    let _waiting = f.offer(0);
    assert!(!f.readable(0));
    f.port(0).arm_wait();
    f.port(1).arm_wait();
    assert!(f.readable(0));
    assert!(!f.readable(1));
    f.port(0).interrupted();
    assert!(f.readable(1));
    assert_eq!(
        f.source
            .queue
            .lock()
            .unwrap()
            .notified
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert!(f.port(2).claim().is_some());
}
#[test]
fn capacity_return_and_failed_target_transfer_only_wake_duty() {
    let f = Fixture::new();
    let held: Vec<_> = (0..4)
        .map(|_| f.source.destinations[0].capacity.try_pair(1).unwrap())
        .collect();
    f.port(0).arm_wait();
    let _waiting = f.offer(0);
    assert!(!f.readable(0));
    drop(held);
    assert!(f.readable(0));
    f.port(1).arm_wait();
    f.wakes[0].close();
    f.port(0).arm_wait();
    assert!(!f.source.destinations[0].waiting.load(Ordering::Acquire));
    assert!(f.readable(1));
    assert!(f.port(1).claim().is_some());
    f.source.close();
    assert!(f.source.queue.lock().unwrap().waiters.is_empty());
    assert!(f.source.queue.lock().unwrap().notified.is_empty());
}

#[test]
fn burst_notification_is_bounded_by_unassigned_work_not_waiter_count() {
    let f = Fixture::new();
    for owner in 0..3 {
        f.port(owner).arm_wait();
    }
    let _a = f.offer(0);
    let _b = f.offer(1);
    assert!(f.readable(0));
    assert!(f.readable(1));
    assert!(!f.readable(2));
    assert_eq!(f.source.queue.lock().unwrap().notified.len(), 2);
    drop(f.port(2).claim().unwrap());
    f.source.cancel(1);
    assert_eq!(f.source.pending.available_permits(), 4);
    assert!(f.source.queue.lock().unwrap().notified.is_empty());
}

#[test]
fn visible_payload_can_be_claimed_without_a_pipe_round_trip() {
    let f = Fixture::new();
    let _waiting = f.offer(0);
    assert!(f.port(0).claim().is_some());
    assert!((0..3).all(|owner| !f.readable(owner)));
    f.port(0).arm_wait();
    assert_eq!(f.source.length.load(Ordering::Acquire), 0);
    assert!(f.source.queue.lock().unwrap().notified.is_empty());
}

#[test]
fn progress_ranking_promotes_an_eligible_oldest_waiter() {
    let f = Fixture::new();
    f.source.progress.store(true, Ordering::Release);
    for (i, w) in f.wakes.iter().enumerate() {
        w.progress.enabled.store(true, Ordering::Release);
        if i == 0 {
            w.progress.charge(24, false);
        }
    }
    // The old expensive owner must be promoted after two lower-service grants.
    for owner in [0, 1, 2] {
        f.port(owner).arm_wait();
    }
    for (id, expected) in [(0, 1), (1, 2), (2, 0)] {
        let assigned = f.offer(id);
        assert!(f.source.queue.lock().unwrap().notified.contains(&expected));
        let request = f.port(expected).claim().unwrap();
        assert_eq!(assigned.blocking_recv().unwrap().unwrap(), expected);
        drop(request);
        if id < 2 {
            f.port(expected).arm_wait();
        }
    }
}

#[test]
fn expired_guard_cannot_publish_a_new_peers_capacity_event() {
    let fixture = Fixture::new();
    let _placement = fixture.offer(0);
    let running = fixture.port(0).claim().unwrap();
    let active = Arc::clone(&fixture.source.destinations[1].active)
        .try_acquire_owned()
        .unwrap();
    fixture.port(1).arm_wait();
    let _next = fixture.offer(1);
    assert!(!fixture.readable(1));
    // Native credit return and shared event publication are separate boundaries.
    drop(active);
    fixture.source.cancel(0); // this client no longer owns any queued input
    assert!(!fixture.readable(1));
    assert_eq!(fixture.source.length.load(Ordering::Acquire), 1);
    fixture.source.capacity_available(); // the actual eligibility event
    assert!(fixture.readable(1));
    assert!(fixture.port(1).claim().is_some());
    drop(running);
}
