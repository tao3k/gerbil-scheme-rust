//! Native capacity released on another owner must unblock stateless admission.
use super::{GATE, idle, short};
use gerbil_scheme::instances::{
    InstanceDispatcher, InstanceError, InstanceTicket, SchemeRouteClass, SchemeRoutingAdmission,
    SchemeRoutingConfig, SchemeServiceExecution, SchemeServiceRouter,
};
use std::{
    future::{Future, poll_fn},
    sync::mpsc,
    task::Poll,
    time::Duration,
};
pub(super) fn router(
    dispatch: &InstanceDispatcher,
    admission: SchemeRoutingAdmission,
) -> SchemeServiceRouter {
    SchemeServiceRouter::with_admission(
        dispatch.clone(),
        SchemeRoutingConfig {
            owners: vec![0, 1],
            short_slots: 1,
            batch_slots: 3,
            short_max_work: 1,
            batch_max_work: 8,
        },
        admission,
    )
    .unwrap()
}
pub(super) fn fill(
    dispatch: &InstanceDispatcher,
    owner: usize,
) -> (mpsc::Sender<()>, Vec<InstanceTicket<()>>) {
    let (entered, ready) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let mut tickets = vec![
        dispatch
            .submit_native_on(owner, move |_| {
                entered.send(()).unwrap();
                blocked.recv_timeout(GATE).unwrap();
                Ok(())
            })
            .unwrap(),
    ];
    ready.recv_timeout(GATE).unwrap();
    for _ in 0..3 {
        tickets.push(dispatch.submit_native_on(owner, |_| Ok(())).unwrap());
    }
    (release, tickets)
}
pub(super) async fn registered<F: Future>(future: std::pin::Pin<&mut F>) {
    let mut future = future;
    poll_fn(|cx| {
        assert!(future.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
}
pub(super) fn empty(router: &SchemeServiceRouter) {
    assert!(
        router
            .loads()
            .iter()
            .all(|l| l.short_requests == 0 && l.batch_requests == 0 && l.batch_work == 0)
    );
}
fn probe() -> short::Short {
    short::Short(vec![0, 0, 1, 1, 1])
}
pub(super) async fn contract(dispatch: &InstanceDispatcher, progress: &mpsc::Sender<&'static str>) {
    for admission in [
        SchemeRoutingAdmission::Reselect,
        SchemeRoutingAdmission::ReselectProgressNotified,
    ] {
        admission_contract(dispatch, progress, admission).await;
    }
}
async fn admission_contract(
    dispatch: &InstanceDispatcher,
    progress: &mpsc::Sender<&'static str>,
    admission: SchemeRoutingAdmission,
) {
    idle(dispatch).await;
    let old = router(dispatch, SchemeRoutingAdmission::Pinned);
    let new = router(dispatch, admission);
    let (release_a, a) = fill(dispatch, 0);
    let (release_b, b) = fill(dispatch, 1);
    let before: Vec<_> = dispatch
        .owner_capacity()
        .into_iter()
        .map(|c| (c.available, c.work_available))
        .collect();
    let mut candidate = Box::pin(new.submit_short(probe(), GATE));
    registered(candidate.as_mut()).await;
    assert_eq!(
        before,
        dispatch
            .owner_capacity()
            .into_iter()
            .map(|c| (c.available, c.work_available))
            .collect::<Vec<_>>(),
        "waiting reselection held partial native credits"
    );
    drop(candidate);
    empty(&new);
    assert!(matches!(
        new.submit_short(probe(), Duration::from_millis(2)).await,
        Err(InstanceError::Timeout)
    ));
    empty(&new);
    let mut candidate = Box::pin(new.submit_short(probe(), GATE));
    registered(candidate.as_mut()).await;
    let mut follower = Box::pin(new.submit_short(probe(), GATE));
    registered(follower.as_mut()).await;
    let mut cancelled = Box::pin(new.submit_short(probe(), GATE));
    registered(cancelled.as_mut()).await;
    drop(cancelled); // cancellation while waiting for the class FIFO turn
    let mut pinned = Box::pin(old.submit_short(probe(), GATE));
    registered(pinned.as_mut()).await;
    release_b.send(()).unwrap();
    let selected = candidate.await.unwrap();
    assert_eq!(selected.owner, 1, "must reselect B while A remains blocked");
    assert_eq!(selected.route_class, SchemeRouteClass::Short);
    assert_eq!(selected.execution_class, SchemeServiceExecution::Short);
    assert_eq!(
        selected.ticket.wait_completed(GATE).await.unwrap().0.len(),
        1
    );
    let second = follower.await.unwrap();
    assert_eq!(
        second.owner, 1,
        "next FIFO admission must also use released B"
    );
    second.ticket.wait_completed(GATE).await.unwrap();
    registered(pinned.as_mut()).await; // old algorithm is still stuck on A
    drop(pinned);
    empty(&old);
    release_a.send(()).unwrap();
    for ticket in a.into_iter().chain(b) {
        ticket.wait(GATE).unwrap();
    }
    idle(dispatch).await;
    empty(&new);
    progress.send("owner reselection: direct native capacity release on B unblocks; pinned A remains blocked; cancellation/timeout credits conserved").unwrap();
    closed(dispatch).await;
    reconfigured(dispatch).await;
}
async fn closed(dispatch: &InstanceDispatcher) {
    let new = router(dispatch, SchemeRoutingAdmission::Reselect);
    let (release_a, a) = fill(dispatch, 0);
    let (release_b, b) = fill(dispatch, 1);
    let mut pending = Box::pin(new.submit_short(probe(), GATE));
    registered(pending.as_mut()).await;
    let mut follower = Box::pin(new.submit_short(probe(), GATE));
    registered(follower.as_mut()).await;
    new.close();
    assert!(matches!(pending.await, Err(InstanceError::Closed)));
    assert!(matches!(follower.await, Err(InstanceError::Closed)));
    empty(&new);
    release_a.send(()).unwrap();
    release_b.send(()).unwrap();
    for ticket in a.into_iter().chain(b) {
        ticket.wait(GATE).unwrap();
    }
    idle(dispatch).await;
}

async fn reconfigured(dispatch: &InstanceDispatcher) {
    let old = router(dispatch, SchemeRoutingAdmission::Pinned);
    let new = router(dispatch, SchemeRoutingAdmission::Reselect);
    let (release_a, a) = fill(dispatch, 0);
    let (release_b, b) = fill(dispatch, 1);
    let mut pending = Box::pin(new.submit_short(probe(), GATE));
    registered(pending.as_mut()).await;
    release_a.send(()).unwrap();
    release_b.send(()).unwrap();
    for ticket in a.into_iter().chain(b) {
        ticket.wait(GATE).unwrap();
    }
    idle(dispatch).await; // Leave pending unpolled across the capacity epoch change.
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    assert!(matches!(pending.await, Err(InstanceError::Closed)));
    assert!(matches!(
        old.submit_short(probe(), GATE).await,
        Err(InstanceError::Closed)
    ));
    empty(&old);
    empty(&new);
    let current = router(dispatch, SchemeRoutingAdmission::Reselect);
    current
        .submit_short(probe(), GATE)
        .await
        .unwrap()
        .ticket
        .wait_completed(GATE)
        .await
        .unwrap();
    idle(dispatch).await;
    empty(&current);
}
