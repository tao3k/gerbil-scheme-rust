//! Close invalidation and registered identity controls, without native VM claims.
use crate::instances::{
    InstanceDispatcher, InstanceError, capacity::Capacity, dispatch::Admission,
};
use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    task::{Context, Wake, Waker},
};
fn dispatcher() -> (InstanceDispatcher, Arc<Capacity>) {
    let dispatch = InstanceDispatcher::new(1024, 1024, 4);
    let (sender, _receiver) = mpsc::sync_channel(4);
    dispatch.add_sender(sender, None);
    dispatch.configure_owner_budgets(1, Some(8)).unwrap();
    let capacity = dispatch.producer_owner(0).unwrap();
    (dispatch, capacity)
}
struct CloseObserver {
    admission: Arc<Admission>,
    woke: AtomicBool,
}
impl Wake for CloseObserver {
    fn wake(self: Arc<Self>) {
        assert!(!self.admission.accepting.load(Ordering::Acquire));
        assert!(
            self.admission.senders.try_lock().is_ok(),
            "callback under admission fence"
        );
        self.woke.store(true, Ordering::Release);
    }
}
#[test]
fn close_invalidates_cached_admission_before_out_of_lock_callbacks() {
    let (dispatch, capacity) = dispatcher();
    let held = Arc::clone(&capacity.slots).try_acquire_owned().unwrap();
    let observer = Arc::new(CloseObserver {
        admission: Arc::clone(&dispatch.admission),
        woke: AtomicBool::new(false),
    });
    let waker = Waker::from(Arc::clone(&observer));
    let mut context = Context::from_waker(&waker);
    let mut waiting = Box::pin(Arc::clone(&capacity.slots).acquire_owned());
    assert!(waiting.as_mut().poll(&mut context).is_pending());
    dispatch.close();
    assert!(observer.woke.load(Ordering::Acquire));
    assert!(waiting.as_mut().poll(&mut context).is_ready());
    drop(held);
}
#[test]
fn a_live_registration_prevents_capacity_identity_replacement() {
    let (dispatch, capacity) = dispatcher();
    let registration = Arc::clone(&capacity.producer).try_acquire_owned().unwrap();
    assert!(matches!(
        dispatch.configure_owner_budgets(2, Some(16)),
        Err(InstanceError::InvalidConfig(_))
    ));
    assert!(Arc::ptr_eq(&capacity, &dispatch.producer_owner(0).unwrap()));
    assert!(dispatch.admission.accepting.load(Ordering::Acquire));
    drop(registration);
    dispatch.configure_owner_budgets(2, Some(16)).unwrap();
    assert!(!Arc::ptr_eq(
        &capacity,
        &dispatch.producer_owner(0).unwrap()
    ));
    assert!(dispatch.admission.accepting.load(Ordering::Acquire));
    dispatch.close();
}
#[test]
fn cached_close_flag_does_not_retain_the_dispatcher_table() {
    let (dispatch, capacity) = dispatcher();
    let table = Arc::downgrade(&dispatch.admission);
    let token = Arc::clone(&dispatch.admission.accepting);
    dispatch.close();
    drop(dispatch);
    drop(capacity);
    assert!(table.upgrade().is_none());
    assert!(!token.load(Ordering::Acquire));
}
