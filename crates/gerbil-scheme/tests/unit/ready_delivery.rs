use super::{Channels, Reply, SchemeServiceReadyStream, channels};
use crate::instances::{InstanceError, SchemeServiceStream};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Wake, Waker},
};
use std::{
    task::{Context, Poll},
    time::{Duration, Instant},
};
#[derive(Default)]
struct Count(AtomicUsize);
impl Wake for Count {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}
fn publish(send: &Arc<Channels<usize>>, sequence: usize, owner: usize) {
    assert!(
        Reply::Ready {
            channels: Arc::clone(send),
            sequence
        }
        .send((Ok(sequence), Instant::now()), Some(owner))
    );
}
#[test]
fn ready_owner_can_bypass_unfinished_earlier_input() {
    let (send, mut recv) = channels(&[3, 7], 3);
    let w = Waker::noop();
    let mut cx = Context::from_waker(w);
    publish(&send, 1, 7);
    let Poll::Ready(Some(row)) = recv.poll(&mut cx) else {
        panic!("ready later input must not wait for zero");
    };
    assert_eq!((row.sequence, row.owner), (1, Some(7)));
    publish(&send, 0, 3);
    publish(&send, 2, 7);
    assert!(matches!(recv.poll(&mut cx), Poll::Ready(Some(_))));
    assert!(matches!(recv.poll(&mut cx), Poll::Ready(Some(_))));
    assert!(matches!(recv.poll(&mut cx), Poll::Ready(None)));
}
#[test]
fn ready_owners_rotate_even_with_one_owner_backlog() {
    let (send, mut recv) = channels(&[3, 7], 4);
    publish(&send, 0, 3);
    publish(&send, 1, 3);
    publish(&send, 2, 3);
    publish(&send, 3, 7);
    let mut cx = Context::from_waker(Waker::noop());
    let mut seen = Vec::new();
    while let Poll::Ready(Some(row)) = recv.poll(&mut cx) {
        seen.push(row.sequence);
    }
    assert_eq!(seen, vec![0, 3, 1, 2]);
}
#[test]
fn latest_observer_waker_covers_every_owner_queue() {
    let (send, mut recv) = channels(&[3, 7], 2);
    let a = Arc::new(Count::default());
    let b = Arc::new(Count::default());
    let aw = Waker::from(Arc::clone(&a));
    let bw = Waker::from(Arc::clone(&b));
    assert!(recv.poll(&mut Context::from_waker(&aw)).is_pending());
    assert!(recv.poll(&mut Context::from_waker(&bw)).is_pending());
    publish(&send, 0, 3);
    publish(&send, 1, 7);
    assert_eq!(a.0.load(Ordering::Relaxed), 0);
    assert!(b.0.load(Ordering::Relaxed) > 0);
}
#[test]
fn timeout_does_not_consume_ready_identity() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (send, receive) = channels(&[0], 1);
            let mut stream = SchemeServiceReadyStream(SchemeServiceStream {
                pending: std::collections::VecDeque::default(),
                ready: Some(receive),
                cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                signals: Vec::new(),
            });
            assert!(matches!(
                stream.next(Duration::ZERO).await,
                Some(Err(InstanceError::Timeout))
            ));
            publish(&send, 0, 0);
            assert_eq!(
                stream
                    .next(Duration::from_secs(1))
                    .await
                    .unwrap()
                    .unwrap()
                    .sequence,
                0
            );
            assert!(stream.next(Duration::ZERO).await.is_none());
        });
}
#[test]
fn dropped_sender_reports_missing_output_and_drop_closes_publish() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (send, receive) = channels::<usize>(&[0], 1);
            let mut stream = SchemeServiceReadyStream(SchemeServiceStream {
                pending: std::collections::VecDeque::default(),
                ready: Some(receive),
                cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                signals: Vec::new(),
            });
            drop(send);
            assert!(matches!(
                stream.next(Duration::from_secs(1)).await,
                Some(Err(InstanceError::Closed))
            ));
            assert!(stream.next(Duration::ZERO).await.is_none());
            let (send, recv) = channels::<usize>(&[0], 1);
            drop(recv);
            assert!(
                !Reply::Ready {
                    channels: send,
                    sequence: 0
                }
                .send((Ok(1), Instant::now()), Some(0))
            );
        });
}

#[test]
fn invalid_owner_cannot_publish_under_another_owner_identity() {
    let (send, _) = channels::<usize>(&[3, 7], 1);
    assert!(
        !Reply::Ready {
            channels: send,
            sequence: 0
        }
        .send((Ok(1), Instant::now()), Some(8))
    );
}
