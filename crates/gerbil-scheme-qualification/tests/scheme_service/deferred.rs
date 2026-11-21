//! Actual late binding, cancellation, native retirement and close drain.
#[cfg(feature = "arrival-clock")]
#[path = "arrival_clock.rs"]
mod arrival_clock;
#[path = "claim_progress.rs"]
mod claim_progress;
use super::{GATE, command, config, corpus, descriptor, idle, reselection, short};
use gerbil_scheme::instances::{
    InstanceDispatcher, InstanceError, InstancePool, InstanceTicket, NativeRoot, NativeSession,
    SchemeServiceRequest, SchemeShortPool, SchemeShortPoolConfig,
};
use std::{sync::mpsc, time::Duration};
fn probe() -> short::Short {
    short::Short(vec![0, 0, 1, 1, 1])
}
pub(super) fn block(
    dispatch: &InstanceDispatcher,
    owner: usize,
) -> (mpsc::Sender<()>, InstanceTicket<()>) {
    let (entered, ready) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let ticket = dispatch
        .submit_native_on(owner, move |_| {
            entered.send(()).unwrap();
            wait.recv_timeout(GATE).unwrap();
            Ok(())
        })
        .unwrap();
    ready.recv_timeout(GATE).unwrap();
    (release, ticket)
}
pub(super) fn unblock(blocked: Vec<(mpsc::Sender<()>, InstanceTicket<()>)>) {
    for (release, ticket) in blocked {
        release.send(()).unwrap();
        ticket.wait(GATE).unwrap();
    }
}
async fn cancel_and_timeout(dispatch: &InstanceDispatcher, supply: &SchemeShortPool) {
    let blocked: Vec<_> = (0..4).map(|owner| block(dispatch, owner)).collect();
    let before: Vec<_> = dispatch
        .owner_capacity()
        .iter()
        .map(|c| (c.available, c.work_available))
        .collect();
    let mut cancelled = Box::pin(supply.submit(probe(), GATE));
    reselection::registered(cancelled.as_mut()).await;
    assert_eq!(
        before,
        dispatch
            .owner_capacity()
            .iter()
            .map(|c| (c.available, c.work_available))
            .collect::<Vec<_>>()
    );
    assert!(
        supply
            .configure_wake(gerbil_scheme::instances::SchemeShortWake::Selective)
            .is_err()
    );
    drop(cancelled);
    // maximum_pending is one: each repetition needs the cancelled/expired slot.
    for _ in 0..3 {
        assert!(matches!(
            supply.submit(probe(), Duration::from_millis(1)).await,
            Err(InstanceError::Timeout)
        ));
    }
    queued_cancellation(dispatch, supply, &before).await;
    unblock(blocked);
    idle(dispatch).await;
}
struct HeldProjection {
    entered: mpsc::Sender<usize>,
    release: mpsc::Receiver<()>,
}
impl SchemeServiceRequest for HeldProjection {
    type Output = usize;
    fn work_units(&self) -> usize {
        1
    }
    fn short_service(&self) -> bool {
        true
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        probe().construct(session)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<usize, InstanceError> {
        let _ = session.copy_numeric::<i64>(output)?;
        self.entered.send(session.owner_index()).unwrap();
        self.release.recv_timeout(GATE).unwrap();
        Ok(session.owner_index())
    }
}
async fn accepted_timeout(dispatch: &InstanceDispatcher, supply: &SchemeShortPool) {
    let (entered, ready) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let accepted = supply
        .submit(
            HeldProjection {
                entered,
                release: wait,
            },
            GATE,
        )
        .await
        .unwrap();
    let owner = ready.recv_timeout(GATE).unwrap();
    assert_eq!(owner, accepted.owner);
    assert!(matches!(
        accepted
            .ticket
            .wait_completed(Duration::from_millis(1))
            .await,
        Err(InstanceError::Timeout)
    ));
    assert_eq!(
        dispatch.owner_capacity()[owner].available,
        3,
        "accepted root/credit must survive observation timeout"
    );
    release.send(()).unwrap();
    idle(dispatch).await;
    // Existing root-balanced probe forces GC on the same fixed native owner.
    command(dispatch, owner, 9);
}
async fn close_queued(pool: &mut InstancePool, supply: &SchemeShortPool) {
    let dispatch = pool.dispatcher();
    let (entered, ready) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let accepted = supply
        .submit(
            HeldProjection {
                entered,
                release: wait,
            },
            GATE,
        )
        .await
        .unwrap();
    let owner = ready.recv_timeout(GATE).unwrap();
    let blocked: Vec<_> = (0..4)
        .filter(|o| *o != owner)
        .map(|owner| block(&dispatch, owner))
        .collect();
    let queued = supply.submit_progress(probe(), GATE).await.unwrap();
    let mut capacity_waiter = Box::pin(supply.submit(probe(), GATE));
    reselection::registered(capacity_waiter.as_mut()).await;
    let pending = pool.shutdown(Duration::ZERO);
    assert_eq!(pending.pending, 4);
    assert_eq!(
        supply
            .configure_wake(gerbil_scheme::instances::SchemeShortWake::Selective)
            .unwrap_err(),
        InstanceError::Closed
    );
    assert!(matches!(
        queued.wait_completed(GATE).await,
        Err(InstanceError::Closed)
    ));
    assert!(matches!(capacity_waiter.await, Err(InstanceError::Closed)));
    release.send(()).unwrap();
    assert_eq!(accepted.ticket.wait_completed(GATE).await.unwrap().0, owner);
    unblock(blocked);
    let closed = pool.shutdown(GATE);
    assert!(closed.is_clean(), "{closed:?}");
    assert_eq!(closed.joined, 4);
    assert!(
        closed
            .owners
            .iter()
            .all(|o| o.roots_before_cleanup == 0 && o.image_file_removed)
    );
}
async fn burst(dispatch: &InstanceDispatcher, supply: &SchemeShortPool) {
    let tasks: Vec<_> = (0..12)
        .map(|id| {
            let supply = supply.clone();
            tokio::spawn(async move {
                let mut input = corpus::order(900 + id, false);
                input.truncate(14);
                let accepted = supply
                    .submit(short::Short(input.clone()), GATE)
                    .await
                    .unwrap();
                let values = accepted.ticket.wait_completed(GATE).await.unwrap().0;
                corpus::verify(&input, &values[0]);
            })
        })
        .collect();
    for task in tasks {
        task.await.unwrap();
    }
    idle(dispatch).await;
}
pub(super) fn contract(runtime: &tokio::runtime::Runtime, progress: &mpsc::Sender<&'static str>) {
    for policy in [
        gerbil_scheme::instances::SchemeShortWake::Broadcast,
        gerbil_scheme::instances::SchemeShortWake::Selective,
        gerbil_scheme::instances::SchemeShortWake::ProgressFair,
    ] {
        policy_contract(runtime, progress, policy);
    }
}
fn policy_contract(
    runtime: &tokio::runtime::Runtime,
    progress: &mpsc::Sender<&'static str>,
    policy: gerbil_scheme::instances::SchemeShortWake,
) {
    let mut pool = InstancePool::start_observed(&descriptor(false), config(), |_| {
        progress.send("late-binding owner initialized").unwrap();
    })
    .unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    for owner in 0..4 {
        dispatch
            .configure_scheme_service_completion_on(
                owner,
                gerbil_scheme::instances::SchemeServiceCompletion::NativeCycle,
            )
            .unwrap()
            .wait(GATE)
            .unwrap();
    }
    if policy == gerbil_scheme::instances::SchemeShortWake::ProgressFair {
        runtime.block_on(super::service_progress::enable(&dispatch));
    }
    let supply = dispatch
        .configure_scheme_short_pool(SchemeShortPoolConfig {
            owners: vec![3, 1, 0, 2],
            maximum_pending: 1,
            maximum_active_per_owner: 1,
        })
        .unwrap();
    assert_eq!(
        supply.wake_policy(),
        gerbil_scheme::instances::SchemeShortWake::Broadcast
    );
    supply.configure_wake(policy).unwrap();
    assert_eq!(supply.wake_policy(), policy);
    assert!(dispatch.configure_owner_budgets(4, Some(32)).is_err());
    runtime.block_on(async {
        for owner in 0..4 {
            dispatch.configure_scheme_completion_publication_on(owner,
                gerbil_scheme::instances::SchemeCompletionPublication::AfterRetirement).unwrap();
        }
        assert!(supply.submit(super::batch(false), GATE).await.is_err());
        cancel_and_timeout(&dispatch, &supply).await;
        progress.send("unassigned cancellation and timeout refund shared capacity without native credits").unwrap();
        let busy = block(&dispatch, 0);
        let mut input = corpus::order(123, false); input.truncate(14);
        let accepted = supply.submit(short::Short(input.clone()), GATE).await.unwrap();
        assert_ne!(accepted.owner, 0, "busy owner cannot claim a short payload");
        let values = accepted.ticket.wait_completed(GATE).await.unwrap().0;
        corpus::verify(&input, &values[0]);
        unblock(vec![busy]);
        burst(&dispatch, &supply).await;
        accepted_timeout(&dispatch, &supply).await;
        queued_accepted_timeout(&dispatch, &supply).await;
        claim_progress::contract(&dispatch, &supply).await;
        #[cfg(feature = "arrival-clock")]
        arrival_clock::contract(&dispatch, &supply).await;
        progress.send("owner claims typed short work; observation timeout retains native root and credit until retirement").unwrap();
        close_queued(&mut pool, &supply).await;
    });
    drop(supply);
    drop(dispatch);
    drop(pool);
    progress.send("late-binding close wakes queued and capacity waiters; four owners drained with zero roots and removed files").unwrap();
}

async fn queued_cancellation(
    dispatch: &InstanceDispatcher,
    supply: &SchemeShortPool,
    before: &[(usize, Option<usize>)],
) {
    for _ in 0..3 {
        let queued = supply.submit_queued(probe(), GATE).await.unwrap();
        assert_eq!(
            before,
            dispatch
                .owner_capacity()
                .iter()
                .map(|c| (c.available, c.work_available))
                .collect::<Vec<_>>()
        );
        assert!(matches!(
            supply
                .submit_queued(probe(), Duration::from_millis(1))
                .await,
            Err(InstanceError::Timeout)
        ));
        drop(queued);
        let expired = supply.submit_queued(probe(), GATE).await.unwrap();
        assert!(matches!(
            expired.wait_completed(Duration::ZERO).await,
            Err(InstanceError::Timeout)
        ));
    }
}
async fn queued_accepted_timeout(dispatch: &InstanceDispatcher, supply: &SchemeShortPool) {
    let (entered, ready) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let accepted = supply
        .submit_queued(
            HeldProjection {
                entered,
                release: wait,
            },
            GATE,
        )
        .await
        .unwrap();
    let owner = ready.recv_timeout(GATE).unwrap();
    assert!(matches!(
        accepted.wait_completed(Duration::ZERO).await,
        Err(InstanceError::Timeout)
    ));
    assert_eq!(
        dispatch.owner_capacity()[owner].available,
        3,
        "a queued ticket cannot retract claimed native ownership"
    );
    release.send(()).unwrap();
    idle(dispatch).await;
    let mut input = corpus::order(923, false);
    input.truncate(14);
    let issue = gerbil_scheme::instances::InstanceArrival::new(std::time::Instant::now(), None)
        .unwrap()
        .issue()
        .unwrap();
    let observed = supply
        .submit_queued(short::Short(input.clone()), GATE)
        .await
        .unwrap()
        .wait_observed(issue, GATE)
        .await
        .unwrap();
    corpus::verify(&input, &observed.result.unwrap()[0]);
    assert!(observed.progress.is_ok());
    assert!(matches!(
        supply
            .submit_queued(short::Short(corpus::order(924, false)), GATE)
            .await
            .unwrap()
            .wait_completed(GATE)
            .await,
        Err(InstanceError::Scheme(_))
    ));
    idle(dispatch).await;
}
