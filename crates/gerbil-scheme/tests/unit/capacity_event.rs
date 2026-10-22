//! Notification protocol controls; no native runtime or OS timing claims.
use crate::instances::capacity_event::CapacityEvents;
use std::future::{Future, poll_fn};
use std::task::Poll;
#[test]
fn arm_scan_publish_wait_and_cancellation() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let events = CapacityEvents::new();
            let first = events.notify.notified();
            let second = events.notify.notified();
            tokio::pin!(first, second);
            first.as_mut().enable();
            second.as_mut().enable();
            // Both classes scanned unavailable credits. A return before await wakes both.
            events.publish();
            events.publish();
            tokio::time::timeout(std::time::Duration::from_secs(1), async {
                first.await;
                second.await;
            })
            .await
            .unwrap();
            let mut cancelled = Box::pin(events.notify.notified());
            cancelled.as_mut().enable();
            drop(cancelled);
            events.publish();
            let next = events.notify.notified();
            tokio::pin!(next);
            next.as_mut().enable();
            poll_fn(|cx| {
                assert!(next.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
            events.publish(); // close/failure uses the same event, followed by a live-state scan
            next.await;
        });
}
#[test]
fn watch_consumer_retains_version_while_notify_waiter_is_armed() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let events = CapacityEvents::new();
            let mut watch = events.subscribe();
            let waiting = events.notify.notified();
            tokio::pin!(waiting);
            waiting.as_mut().enable();
            watch.mark_unchanged();
            events.publish();
            assert!(watch.has_changed().unwrap());
            waiting.await;
            watch.changed().await.unwrap();
        });
}
