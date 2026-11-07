use gerbil_scheme::instances::{
    SchemeIngressConfig,
    SchemeRouteClass::{Batch, Short},
    SchemeServiceIngress,
};
use std::time::Duration;
fn queue() -> SchemeServiceIngress<u32> {
    SchemeServiceIngress::new(SchemeIngressConfig {
        queue_capacity: 3,
        client_capacity: 2,
        queue_reserved: [1, 1],
        client_reserved: [1, 1],
    })
    .unwrap()
}
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}
#[test]
fn ingress_queue_borrow_reclaim_and_foreign_cancel() {
    let q = queue();
    let other = queue();
    let first = q.try_enqueue(Batch, 1).unwrap();
    q.try_enqueue(Batch, 3).unwrap();
    q.try_enqueue(Batch, 5).unwrap();
    assert_eq!(q.try_enqueue(Short, 2).unwrap().reclaimed, Some(5));
    assert_eq!(q.snapshot().queued, [1, 2]);
    assert_eq!(other.cancel_pending(&first.id), None);
    assert_eq!(q.cancel_pending(&first.id), Some(1));
    assert_eq!(q.cancel_pending(&first.id), None);
}
#[test]
fn ingress_borrowed_clients_return_at_consumption_and_short_gets_next_claim() {
    runtime().block_on(async {
        let q = queue();
        q.try_enqueue(Batch, 1).unwrap();
        q.try_enqueue(Batch, 3).unwrap();
        let mut a = q.recv().await.unwrap();
        let b = q.recv().await.unwrap();
        assert_eq!(a.take(), Some(1));
        assert_eq!(q.snapshot().clients, [0, 2]);
        q.try_enqueue(Batch, 5).unwrap();
        q.try_enqueue(Short, 2).unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(2), q.recv())
                .await
                .is_err()
        );
        // Removing input is not native completion or result consumption.
        assert_eq!(q.snapshot().clients, [0, 2]);
        drop(a);
        let mut short = q.recv().await.unwrap();
        assert_eq!(short.route_class(), Short);
        assert_eq!(short.take(), Some(2));
        assert_eq!(q.snapshot().clients, [1, 1]);
        drop(short);
        drop(b);
        let mut remaining = q.recv().await.unwrap();
        assert_eq!(remaining.take(), Some(5));
        drop(remaining);
        assert_eq!(q.snapshot().clients, [0, 0]);
    });
}
#[test]
fn ingress_cancel_wait_and_close_drain_return_all_charges() {
    runtime().block_on(async {
        let q = queue();
        let pending = q.try_enqueue(Short, 2).unwrap();
        assert_eq!(q.cancel_pending(&pending.id), Some(2));
        assert!(
            tokio::time::timeout(Duration::from_millis(2), q.recv())
                .await
                .is_err()
        );
        q.try_enqueue(Short, 4).unwrap();
        q.try_enqueue(Batch, 1).unwrap();
        q.close();
        assert!(q.try_enqueue(Short, 6).is_err());
        let a = q.recv().await.unwrap();
        let b = q.recv().await.unwrap();
        assert!(q.recv().await.is_none());
        drop(a);
        drop(b);
        assert_eq!(q.snapshot().queued, [0, 0]);
        assert_eq!(q.snapshot().clients, [0, 0]);
    });
}
#[test]
fn ingress_both_classes_progress_under_surplus() {
    runtime().block_on(async {
        let q = queue();
        q.try_enqueue(Short, 2).unwrap();
        q.try_enqueue(Batch, 1).unwrap();
        let mut a = q.recv().await.unwrap();
        assert_eq!(a.take(), Some(2));
        q.try_enqueue(Short, 4).unwrap();
        let mut b = q.recv().await.unwrap();
        assert_eq!(b.take(), Some(1));
        drop(a);
        drop(b);
    });
}

#[test]
fn ingress_last_cancel_wakes_closed_receivers_without_early_active_release() {
    runtime().block_on(async {
        let q = queue();
        q.try_enqueue(Batch, 1).unwrap();
        q.try_enqueue(Batch, 3).unwrap();
        let a = q.recv().await.unwrap();
        let b = q.recv().await.unwrap();
        let queued = q.try_enqueue(Short, 2).unwrap();
        q.close();
        let waiting = q.recv();
        tokio::pin!(waiting);
        assert!(
            tokio::time::timeout(Duration::from_millis(2), &mut waiting)
                .await
                .is_err()
        );
        assert_eq!(q.cancel_pending(&queued.id), Some(2));
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut waiting)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(q.snapshot().clients, [0, 2]);
        drop(a);
        drop(b);
        assert_eq!(q.snapshot().clients, [0, 0]);
    });
}
