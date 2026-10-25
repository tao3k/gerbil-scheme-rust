//! Transport controls only; no Scheme execution or OS scheduling claims.
use super::Wait;
use crate::instances::{AsyncInstanceTicket, InstanceError};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
    time::Instant,
};
use tokio::sync::oneshot;
#[derive(Default)]
struct Signals(AtomicUsize);
impl Wake for Signals {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
fn poll(wait: &mut Wait<i64>, signals: &Arc<Signals>) -> Poll<<Wait<i64> as Future>::Output> {
    let waker = Waker::from(Arc::clone(signals));
    Pin::new(wait).poll(&mut Context::from_waker(&waker))
}
#[test]
fn early_claim_is_pending_until_final_result_and_keeps_the_same_waker() {
    let (assigned, placement) = oneshot::channel();
    let (send, response) = oneshot::channel();
    let mut wait = Wait::new(AsyncInstanceTicket { response }, placement);
    let signals = Arc::new(Signals::default());
    assert!(poll(&mut wait, &signals).is_pending());
    assigned.send(Ok(2)).unwrap();
    assert!(signals.0.load(Ordering::SeqCst) > 0);
    assert!(poll(&mut wait, &signals).is_pending());
    assert_eq!(wait.progress.owner, Some(2));
    let claimed_polls = wait.progress.claim_polls;
    assert!(poll(&mut wait, &signals).is_pending());
    assert_eq!(wait.progress.claim_polls, claimed_polls);
    send.send((Ok(9), Instant::now())).unwrap();
    assert!(matches!(
        poll(&mut wait, &signals),
        Poll::Ready(Ok((Ok(9), _)))
    ));
    assert!(!wait.progress.final_without_claim);
}
#[test]
fn already_ready_final_result_coalesces_the_unconsumed_claim_event() {
    let (assigned, placement) = oneshot::channel();
    let (send, response) = oneshot::channel();
    let mut wait = Wait::new(AsyncInstanceTicket { response }, placement);
    assigned.send(Ok(1)).unwrap();
    send.send((Ok(7), Instant::now())).unwrap();
    assert!(matches!(
        poll(&mut wait, &Arc::new(Signals::default())),
        Poll::Ready(Ok((Ok(7), _)))
    ));
    assert_eq!(wait.progress.claim_polls, 0);
    assert!(wait.progress.observed.is_none());
    assert!(wait.progress.final_without_claim);
}
#[test]
fn failed_claim_never_fabricates_a_final_value() {
    let (assigned, placement) = oneshot::channel();
    let (send, response) = oneshot::channel();
    let mut wait = Wait::new(AsyncInstanceTicket { response }, placement);
    let signals = Arc::new(Signals::default());
    assigned.send(Err(InstanceError::Closed)).unwrap();
    assert!(poll(&mut wait, &signals).is_pending());
    assert!(wait.progress.owner.is_none());
    send.send((Err(InstanceError::Closed), Instant::now()))
        .unwrap();
    assert!(matches!(
        poll(&mut wait, &signals),
        Poll::Ready(Ok((Err(InstanceError::Closed), _)))
    ));
}
#[test]
fn repoll_moves_both_channel_registrations_to_the_current_task_waker() {
    let (assigned, placement) = oneshot::channel();
    let (send, response) = oneshot::channel();
    let mut wait = Wait::new(AsyncInstanceTicket { response }, placement);
    let old = Arc::new(Signals::default());
    let current = Arc::new(Signals::default());
    assert!(poll(&mut wait, &old).is_pending());
    assert!(poll(&mut wait, &current).is_pending());
    assigned.send(Ok(0)).unwrap();
    assert_eq!(old.0.load(Ordering::SeqCst), 0);
    assert!(current.0.load(Ordering::SeqCst) > 0);
    assert!(poll(&mut wait, &current).is_pending());
    send.send((Ok(4), Instant::now())).unwrap();
    assert!(matches!(
        poll(&mut wait, &current),
        Poll::Ready(Ok((Ok(4), _)))
    ));
}
#[test]
fn closed_transport_finishes_without_a_claim_receipt() {
    let (assigned, placement) = oneshot::channel();
    let (send, response) = oneshot::channel();
    let mut wait = Wait::new(AsyncInstanceTicket { response }, placement);
    drop(assigned);
    drop(send);
    assert!(matches!(
        poll(&mut wait, &Arc::new(Signals::default())),
        Poll::Ready(Err(_))
    ));
}

#[test]
fn one_published_result_survives_exhausted_bulk_budget_without_serving_pending_work() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (assigned, placement) = oneshot::channel();
            let (send, response) = oneshot::channel();
            let mut ready =
                Wait::<i64, true, true>::new(AsyncInstanceTicket { response }, placement);
            let (_other_claim, placement) = oneshot::channel();
            let (other_send, response) = oneshot::channel();
            let mut control = Wait::<i64>::new(AsyncInstanceTicket { response }, placement);
            let (pending_claim, placement) = oneshot::channel();
            let (pending_send, response) = oneshot::channel();
            let mut pending =
                Wait::<i64, true, true>::new(AsyncInstanceTicket { response }, placement);
            let signals = Arc::new(Signals::default());
            let waker = Waker::from(Arc::clone(&signals));
            let mut cx = Context::from_waker(&waker);
            assert!(Pin::new(&mut ready).poll(&mut cx).is_pending());
            assigned.send(Ok(1)).unwrap();
            send.send((Ok(7), Instant::now())).unwrap();
            other_send.send((Ok(9), Instant::now())).unwrap();
            while tokio::task::coop::has_budget_remaining() {
                assert!(
                    std::pin::pin!(tokio::task::consume_budget())
                        .as_mut()
                        .poll(&mut cx)
                        .is_ready()
                );
            }
            assert!(Pin::new(&mut control).poll(&mut cx).is_pending());
            assert!(Pin::new(&mut pending).poll(&mut cx).is_pending());
            assert!(matches!(
                Pin::new(&mut ready).poll(&mut cx),
                Poll::Ready(Ok((Ok(7), _)))
            ));
            assert!(ready.progress.final_without_claim);
            assert!(ready.progress.ready_without_budget);
            assert!(!tokio::task::coop::has_budget_remaining());
            // Recover a normal task poll before registering the empty receiver.
            tokio::task::yield_now().await;
            assert!(Pin::new(&mut pending).poll(&mut cx).is_pending());
            let before = signals.0.load(Ordering::SeqCst);
            pending_claim.send(Ok(2)).unwrap();
            assert!(signals.0.load(Ordering::SeqCst) > before);
            assert!(Pin::new(&mut pending).poll(&mut cx).is_pending());
            pending_send
                .send((Err(InstanceError::Closed), Instant::now()))
                .unwrap();
            assert!(matches!(
                Pin::new(&mut pending).poll(&mut cx),
                Poll::Ready(Ok((Err(InstanceError::Closed), _)))
            ));
        });
}
