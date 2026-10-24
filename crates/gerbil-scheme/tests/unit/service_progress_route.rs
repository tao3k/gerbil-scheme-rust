//! Real bounded ports, pipes and credits: Rust offer state is not native state.
use super::deliver;
use crate::instances::{
    Fair, InstanceDispatcher, NumericServiceBatch, Offer, PoolSource, Request, RouteTarget,
    pool::Work,
    service::TypedJob,
    service_stream_runner::Signal,
    service_stream_source::Source,
    service_wake::{Reader, Wake},
};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use tokio::sync::{Semaphore, oneshot};
struct Fixture {
    dispatch: InstanceDispatcher,
    source: Arc<PoolSource>,
    target: RouteTarget,
    inbox: mpsc::Receiver<Request>,
    _reader: Reader,
}
impl Fixture {
    fn new() -> Self {
        Self::bounded(4, 32)
    }
    fn bounded(slots: usize, work: usize) -> Self {
        assert!(crate::instance_backend::claim_images());
        let dispatch = InstanceDispatcher::new(1024, 1024, 4);
        let (reader, wake) = Wake::create().unwrap();
        let (send, inbox) = mpsc::sync_channel(1);
        dispatch.add_sender(send, Some(Arc::clone(&wake)));
        dispatch.configure_owner_budgets(slots, Some(work)).unwrap();
        let capacity = Arc::clone(
            dispatch.admission.senders.lock().unwrap().owners[0]
                .capacity
                .as_ref()
                .unwrap(),
        );
        let source = PoolSource::new(VecDeque::from([request(8)]));
        source.fair.set(Fair::default()).ok().unwrap();
        let target = RouteTarget {
            wake,
            capacity,
            active: Arc::new(Semaphore::new(3)),
            staged: Arc::default(),
            mailbox: Arc::downgrade(&dispatch.admission.senders.lock().unwrap().owners[0].sender),
        };
        Self {
            dispatch,
            source,
            target,
            inbox,
            _reader: reader,
        }
    }
    fn deliver(&self, cancelled: bool, expired: bool) -> bool {
        deliver(
            &self.source,
            &self.target,
            8,
            &self.dispatch.admission.accepting,
            &Arc::new(AtomicBool::new(cancelled)),
            if expired {
                Instant::now()
            } else {
                Instant::now() + Duration::from_secs(1)
            },
        )
    }
    fn offer(&self) -> Arc<Offer> {
        self.target.wake.offers.peek().unwrap()
    }
    fn free(&self) {
        assert_eq!(self.target.active.available_permits(), 3);
        assert_eq!(
            self.target.capacity.slots.available_permits(),
            self.target.capacity.slot_limit
        );
        assert_eq!(
            self.target
                .capacity
                .work
                .as_ref()
                .unwrap()
                .available_permits(),
            self.target.capacity.work_limit
        );
    }
}
fn request(units: usize) -> Request {
    let (send, _receive) = oneshot::channel();
    Request {
        started: Arc::new(AtomicBool::new(false)),
        capacity: None,
        work_units: units,
        work: Work::Service(Box::new(TypedJob {
            message: NumericServiceBatch {
                items: vec![vec![1_i64]; units],
                yield_every: 1,
            },
            send: send.into(),
            submitted: false.into(),
            lane: None,
            short: false,
            trace: None,
        })),
    }
}
#[test]
fn offer_holds_no_native_credit_and_entry_keeps_credit_through_projection() {
    let f = Fixture::new();
    assert!(f.deliver(false, false));
    f.free();
    assert!(f.inbox.try_recv().is_err());
    assert_eq!(f.target.staged.load(Ordering::Acquire), 8);
    let offer = f.offer();
    let mut received = offer.claim(&f.target.wake).enter().unwrap();
    assert!(!f.target.wake.offers.pending());
    assert!(offer.claim(&f.target.wake).enter().is_none());
    assert_eq!(f.target.active.available_permits(), 2);
    assert_eq!(
        f.target.capacity.work.as_ref().unwrap().available_permits(),
        24
    );
    received.capacity.as_mut().unwrap().entered();
    assert_eq!(f.target.staged.load(Ordering::Acquire), 0);
    assert_eq!(f.target.active.available_permits(), 2);
    drop(received);
    f.free();
}
#[test]
fn ordinary_full_inbox_does_not_block_offer_and_withdrawal_reuses_slot_immediately() {
    let f = Fixture::new();
    f.dispatch.admission.senders.lock().unwrap().owners[0]
        .sender
        .try_send(request(1))
        .unwrap();
    assert!(f.deliver(false, false));
    assert_eq!(f.inbox.try_recv().unwrap().work_units, 1);
    let old = f.offer();
    let claim = old.claim(&f.target.wake);
    let mut source = Source::Pool(Arc::clone(&f.source));
    let withdrawn = source.take(8).unwrap();
    assert!(!f.target.wake.offers.pending());
    f.free();
    f.source.requests.lock().unwrap().push_back(withdrawn);
    assert!(f.deliver(false, false));
    let new = f.offer();
    assert!(!Arc::ptr_eq(&old, &new));
    assert!(claim.enter().is_none());
    assert!(old.withdraw().is_none());
    assert!(
        Arc::ptr_eq(&f.offer(), &new),
        "stale removal cannot clear replacement"
    );
    drop(new.withdraw());
    f.free();
}
#[test]
fn occupied_offer_port_preserves_source_head_without_reserving_native_credits() {
    let f = Fixture::new();
    assert!(f.deliver(false, false));
    f.source.requests.lock().unwrap().push_back(request(8));
    assert!(!f.deliver(false, false));
    f.free();
    assert_eq!(f.source.requests.lock().unwrap().len(), 1);
    assert_eq!(f.target.staged.load(Ordering::Acquire), 8);
}
#[test]
fn cancellation_expiry_closed_peer_and_retired_registration_do_not_offer() {
    let f = Fixture::new();
    assert!(!f.deliver(true, false));
    assert!(!f.deliver(false, true));
    f.free();
    f.dispatch.admission.senders.lock().unwrap().owners.clear();
    assert!(!f.deliver(false, false));
    f.free();
    assert_eq!(f.source.requests.lock().unwrap().len(), 1);
}
#[test]
fn short_credit_is_available_while_bulk_offer_waits_and_bulk_retries_after_short() {
    let f = Fixture::bounded(1, 8);
    assert!(f.deliver(false, false));
    f.free();
    let short = f.target.capacity.try_pair(1).unwrap();
    let offer = f.offer();
    assert!(offer.claim(&f.target.wake).enter().is_none());
    assert!(offer.pending() && f.target.wake.offers.pending());
    assert_eq!(f.target.active.available_permits(), 3);
    assert_eq!(
        f.target.capacity.work.as_ref().unwrap().available_permits(),
        7
    );
    drop(short);
    drop(offer.claim(&f.target.wake).enter().unwrap());
    f.free();
}
#[test]
fn unentered_cancel_expire_close_and_reject_clear_the_slot_and_all_hints() {
    for case in 0..4 {
        let f = Fixture::new();
        let cancelled = Arc::new(AtomicBool::new(false));
        let deadline = Instant::now() + Duration::from_millis(100);
        assert!(deliver(
            &f.source,
            &f.target,
            8,
            &f.dispatch.admission.accepting,
            &cancelled,
            deadline
        ));
        let offer = f.offer();
        let claim = offer.claim(&f.target.wake);
        match case {
            0 => cancelled.store(true, Ordering::Release),
            1 => std::thread::sleep(deadline.saturating_duration_since(Instant::now())),
            2 => f.target.wake.close(),
            _ => Source::Pool(Arc::clone(&f.source))
                .reject(&crate::instances::InstanceError::Cancelled),
        }
        assert!(claim.enter().is_none());
        f.free();
        assert!(!f.target.wake.offers.pending());
        assert_eq!(f.target.staged.load(Ordering::Acquire), 0);
    }
}
#[test]
fn entry_and_withdrawal_have_one_payload_winner_without_stale_occupancy() {
    for _ in 0..32 {
        let f = Fixture::new();
        assert!(f.deliver(false, false));
        let events = f
            .target
            .capacity
            .changed
            .get_or_init(crate::instances::capacity_event::CapacityEvents::new);
        let changed = events.subscribe();
        let offer = f.offer();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let other = Arc::clone(&offer);
        let ready = Arc::clone(&barrier);
        let withdrawal = std::thread::spawn(move || {
            ready.wait();
            other.withdraw()
        });
        barrier.wait();
        let entered = offer.claim(&f.target.wake).enter();
        let withdrawn = withdrawal.join().unwrap();
        assert_ne!(entered.is_some(), withdrawn.is_some());
        let entered_native = entered.is_some();
        drop((entered, withdrawn));
        assert_eq!(
            changed.has_changed().unwrap(),
            entered_native,
            "withdrawal winner must not cause ghost native-credit publication"
        );
        f.free();
        assert!(!f.target.wake.offers.pending());
        assert_eq!(f.target.staged.load(Ordering::Acquire), 0);
    }
}
struct Count(AtomicUsize);
impl std::task::Wake for Count {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::AcqRel);
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::AcqRel);
    }
}
#[test]
fn same_price_entry_has_no_source_fanout_but_price_change_and_eof_signal() {
    for next in [1, 8] {
        let f = Fixture::new();
        f.source.requests.lock().unwrap().push_back(request(next));
        let count = Arc::new(Count(AtomicUsize::new(0)));
        let signal = Arc::new(Signal::new(
            std::task::Waker::from(Arc::clone(&count)),
            Arc::downgrade(&f.target.wake),
        ));
        f.source
            .signals
            .lock()
            .unwrap()
            .push(Arc::downgrade(&signal));
        assert!(f.deliver(false, false));
        assert_eq!(count.0.load(Ordering::Acquire), 0);
        let offer = f.offer();
        drop(offer.claim(&f.target.wake).enter().unwrap());
        offer.wake_source();
        assert_eq!(count.0.load(Ordering::Acquire), usize::from(next != 8));
        drop(Source::Pool(Arc::clone(&f.source)).take(next));
        assert_eq!(count.0.load(Ordering::Acquire), usize::from(next != 8) + 1);
    }
}

#[test]
fn port_publication_racing_close_cannot_leave_a_live_slot_or_hint() {
    for _ in 0..32 {
        let f = Fixture::new();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let ready = Arc::clone(&barrier);
        let wake = Arc::clone(&f.target.wake);
        let close = std::thread::spawn(move || {
            ready.wait();
            wake.close();
        });
        barrier.wait();
        let _ = f.deliver(false, false);
        close.join().unwrap();
        assert!(!f.target.wake.offers.pending());
        assert!(f.target.wake.offers.peek().is_none());
        assert_eq!(f.target.staged.load(Ordering::Acquire), 0);
        f.free();
    }
}

struct FenceProbe {
    offer: Arc<Offer>,
    free: AtomicBool,
}
impl std::task::Wake for FenceProbe {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.free
            .store(self.offer.entry_fence_is_free(), Ordering::Release);
    }
}
#[test]
fn partial_native_credit_return_calls_waiters_only_after_entry_fence_unlocks() {
    use std::future::Future;
    let f = Fixture::new();
    assert!(f.deliver(false, false));
    let busy = Arc::clone(&f.target.capacity.slots)
        .try_acquire_many_owned(4)
        .unwrap();
    let events = f
        .target
        .capacity
        .changed
        .get_or_init(crate::instances::capacity_event::CapacityEvents::new);
    let offer = f.offer();
    let probe = Arc::new(FenceProbe {
        offer: Arc::clone(&offer),
        free: AtomicBool::new(false),
    });
    let waker = std::task::Waker::from(Arc::clone(&probe));
    let mut context = std::task::Context::from_waker(&waker);
    let notified = events.notify.notified();
    let mut notified = std::pin::pin!(notified);
    assert!(notified.as_mut().poll(&mut context).is_pending());
    assert!(offer.claim(&f.target.wake).enter().is_none());
    assert!(probe.free.load(Ordering::Acquire));
    assert!(notified.as_mut().poll(&mut context).is_ready());
    assert!(offer.pending());
    drop(busy);
    drop(offer.withdraw());
    f.free();
}
