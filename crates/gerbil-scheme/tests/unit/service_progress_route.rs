//! Transaction controls use real channels/pipes, without executing Scheme.
use super::{deliver, reserve};
use crate::instances::{
    Fair, InstanceDispatcher, NumericServiceBatch, Request,
    pool::Work,
    service::TypedJob,
    service_wake::{Reader, Wake},
};
use crate::instances::{PoolSource, RouteTarget};
use std::{collections::VecDeque, sync::mpsc, time::Duration};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
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
        assert!(crate::instance_backend::claim_images());
        let dispatch = InstanceDispatcher::new(1024, 1024, 4);
        let (reader, wake) = Wake::create().unwrap();
        let (send, inbox) = mpsc::sync_channel(1);
        dispatch.add_sender(send, Some(Arc::clone(&wake)));
        dispatch.configure_owner_budgets(4, Some(32)).unwrap();
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
        let mut permit = Some(reserve(&self.target, 8).unwrap());
        let result = deliver(
            &self.source,
            &self.target,
            &self.dispatch.admission.accepting,
            &mut permit,
            &AtomicBool::new(cancelled),
            if expired {
                Instant::now()
            } else {
                Instant::now() + Duration::from_secs(1)
            },
        );
        drop(permit);
        result
    }
    fn refunded(&self) {
        assert_eq!(self.target.active.available_permits(), 3);
        assert_eq!(self.target.capacity.slots.available_permits(), 4);
        assert_eq!(
            self.target
                .capacity
                .work
                .as_ref()
                .unwrap()
                .available_permits(),
            32
        );
        assert_eq!(self.target.staged.load(Ordering::Acquire), 0);
        assert_eq!(
            self.source
                .requests
                .lock()
                .unwrap()
                .front()
                .unwrap()
                .work_units,
            8
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
            send,
            submitted: false,
            lane: None,
            short: false,
            trace: None,
        })),
    }
}
#[test]
fn actual_delivery_keeps_native_credits_after_inbox_hint_moves() {
    let f = Fixture::new();
    assert!(f.deliver(false, false));
    assert!(f.source.requests.lock().unwrap().is_empty());
    assert_eq!(f.target.staged.load(Ordering::Acquire), 8);
    let mut received = f.inbox.try_recv().unwrap();
    let permit = received.capacity.as_mut().unwrap();
    permit.entered();
    assert_eq!(f.target.staged.load(Ordering::Acquire), 0);
    assert_eq!(f.target.active.available_permits(), 2);
    assert_eq!(
        f.target.capacity.work.as_ref().unwrap().available_permits(),
        24
    );
    drop(received);
    assert_eq!(f.target.active.available_permits(), 3);
    assert_eq!(f.target.capacity.slots.available_permits(), 4);
    assert_eq!(
        f.target.capacity.work.as_ref().unwrap().available_permits(),
        32
    );
}
#[test]
fn full_or_disconnected_inbox_restores_fifo_and_all_credits() {
    let f = Fixture::new();
    f.dispatch.admission.senders.lock().unwrap().owners[0]
        .sender
        .try_send(request(1))
        .unwrap();
    assert!(!f.deliver(false, false));
    f.refunded();
    drop(f.inbox.try_recv().unwrap());
    let Fixture {
        dispatch,
        source,
        target,
        inbox,
        _reader,
    } = f;
    drop(inbox);
    let mut permit = Some(reserve(&target, 8).unwrap());
    assert!(!deliver(
        &source,
        &target,
        &dispatch.admission.accepting,
        &mut permit,
        &AtomicBool::new(false),
        Instant::now() + Duration::from_secs(1)
    ));
    drop(permit);
    assert_eq!(source.requests.lock().unwrap().len(), 1);
    assert_eq!(target.staged.load(Ordering::Acquire), 0);
    assert_eq!(target.active.available_permits(), 3);
    assert_eq!(
        target.capacity.work.as_ref().unwrap().available_permits(),
        32
    );
}
#[test]
fn cancellation_expiry_and_closed_peer_do_not_bind_work() {
    let f = Fixture::new();
    assert!(!f.deliver(true, false));
    f.refunded();
    assert!(!f.deliver(false, true));
    f.refunded();
    f.target.wake.close();
    assert!(!f.deliver(false, false));
    f.refunded();
}

#[test]
fn expired_registration_mailbox_cannot_bind_a_replacement_owner() {
    let f = Fixture::new();
    f.dispatch.admission.senders.lock().unwrap().owners.clear();
    assert!(f.target.mailbox.upgrade().is_none());
    assert!(!f.deliver(false, false));
    f.refunded();
}
