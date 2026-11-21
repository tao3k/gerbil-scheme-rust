//! Owner choice commits at native claim; a stalled peer cannot hold it.
use super::*;
use gerbil_scheme::instances::{
    SchemeRoutingAdmission, SchemeRoutingConfig, SchemeServiceRouter, SchemeShortPoolConfig,
    SchemeShortWake,
};
fn routing_config() -> SchemeRoutingConfig {
    SchemeRoutingConfig {
        owners: vec![0, 1],
        short_slots: 1,
        batch_slots: 3,
        short_max_work: 1,
        batch_max_work: 8,
    }
}
fn router(dispatch: &InstanceDispatcher) -> SchemeServiceRouter {
    SchemeServiceRouter::with_admission(
        dispatch.clone(),
        routing_config(),
        SchemeRoutingAdmission::ReselectProgressNotified,
    )
    .unwrap()
}
fn input(id: usize) -> Vec<i64> {
    let mut v = corpus::order(id, false);
    v.truncate(14);
    v
}
struct Held {
    ticket: gerbil_scheme::instances::InstanceTicket<()>,
    release: mpsc::SyncSender<()>,
}
impl Held {
    fn start(dispatch: &InstanceDispatcher, owner: usize) -> Self {
        let (entered, waiting) = mpsc::sync_channel(1);
        let (release, gate) = mpsc::sync_channel(1);
        let ticket = dispatch
            .submit_native_on(owner, move |_| {
                entered.send(()).unwrap();
                gate.recv_timeout(GATE)
                    .map_err(|_| InstanceError::Timeout)?;
                Ok(())
            })
            .unwrap();
        waiting.recv_timeout(GATE).unwrap();
        Self { ticket, release }
    }
    fn finish(self) {
        self.release.send(()).unwrap();
        self.ticket.wait(GATE).unwrap();
    }
}
fn configure(dispatch: &InstanceDispatcher) -> SchemeServiceRouter {
    assert!(router(dispatch).with_owner_claimed_shorts().is_err());
    let pool = dispatch
        .configure_scheme_short_pool(SchemeShortPoolConfig {
            owners: vec![0, 1],
            maximum_pending: 16,
            maximum_active_per_owner: 1,
        })
        .unwrap();
    pool.configure_wake(SchemeShortWake::Selective).unwrap();
    let mut wrong = routing_config();
    wrong.owners = vec![0];
    assert!(
        SchemeServiceRouter::new(dispatch.clone(), wrong.clone())
            .unwrap()
            .with_owner_claimed_shorts()
            .is_err()
    );
    wrong.owners = vec![0, 1];
    wrong.short_slots = 2;
    wrong.batch_slots = 2;
    assert!(
        SchemeServiceRouter::new(dispatch.clone(), wrong)
            .unwrap()
            .with_owner_claimed_shorts()
            .is_err()
    );
    router(dispatch).with_owner_claimed_shorts().unwrap()
}
async fn cancel_close(dispatch: &InstanceDispatcher, claimed: &SchemeServiceRouter) {
    let held = Held::start(dispatch, 1);
    let mut cancelled = Box::pin(claimed.submit_short(short::Short(input(902)), GATE));
    assert!(
        tokio::time::timeout(Duration::from_millis(2), cancelled.as_mut())
            .await
            .is_err()
    );
    drop(cancelled);
    assert!(matches!(
        claimed
            .submit_short(short::Short(input(903)), Duration::from_millis(2))
            .await,
        Err(InstanceError::Timeout)
    ));
    let mut closing = Box::pin(claimed.submit_short(short::Short(input(904)), GATE));
    assert!(
        tokio::time::timeout(Duration::from_millis(2), closing.as_mut())
            .await
            .is_err()
    );
    claimed.close();
    assert!(matches!(closing.await, Err(InstanceError::Closed)));
    held.finish();
}
async fn claimed_timeout(dispatch: &InstanceDispatcher, claimed: &SchemeServiceRouter) {
    command(dispatch, 1, 9);
    let mut waiting = input(905);
    waiting[1] = 2;
    let selected = claimed
        .submit_short(short::Short(waiting), GATE)
        .await
        .unwrap();
    assert_eq!(selected.owner, 1);
    let start = Instant::now();
    while command(dispatch, 1, 10)[0] < 1 {
        assert!(start.elapsed() < GATE);
        tokio::task::yield_now().await;
    }
    assert!(matches!(
        selected
            .ticket
            .wait_completed(Duration::from_millis(2))
            .await,
        Err(InstanceError::Timeout)
    ));
    assert_eq!(
        claimed
            .loads()
            .iter()
            .find(|r| r.owner == 1)
            .unwrap()
            .short_requests,
        1,
        "client timeout must not return a claimed native short charge"
    );
    command(dispatch, 1, 8);
    let returned = Instant::now();
    loop {
        let state = dispatch.owner_capacity()[1].clone();
        if state.available == state.limit && state.work_available == state.work_limit {
            break;
        }
        assert!(
            returned.elapsed() < GATE,
            "claimed peer's native quota did not return"
        );
        tokio::task::yield_now().await;
    }
    assert!(claimed.loads().iter().all(|r| r.short_requests == 0));
}
async fn exercise(
    dispatch: &InstanceDispatcher,
    claimed: &SchemeServiceRouter,
    progress: &mpsc::Sender<&'static str>,
) {
    let held = Held::start(dispatch, 0);
    let legacy = SchemeServiceRouter::new(dispatch.clone(), routing_config())
        .unwrap()
        .submit_short(short::Short(input(900)), GATE)
        .await
        .unwrap();
    assert_eq!(
        legacy.owner, 0,
        "control must publish into stalled owner's inbox"
    );
    let expected = input(901);
    let result = claimed
        .submit_short(short::Short(expected.clone()), GATE)
        .await
        .unwrap();
    assert_eq!(
        result.owner, 1,
        "only a running peer can commit owner choice"
    );
    let reply = result.ticket.wait_completed(GATE).await.unwrap().0;
    corpus::verify(&expected, &reply[0]);
    assert!(claimed.submit_short(batch(false), GATE).await.is_err());
    progress
        .send("stalled owner bypassed with native verified result")
        .unwrap();
    claimed_timeout(dispatch, claimed).await;
    cancel_close(dispatch, claimed).await;
    held.finish();
    let values = legacy.ticket.wait_completed(GATE).await.unwrap().0;
    corpus::verify(&input(900), &values[0]);
    idle(dispatch).await;
    assert!(
        claimed
            .loads()
            .iter()
            .all(|r| r.short_requests == 0 && r.batch_requests == 0 && r.batch_work == 0)
    );
    progress
        .send("cancel timeout close native drain verified")
        .unwrap();
}
#[test]
fn owner_claim_bypasses_stalled_peer_and_drains_cancellation() {
    let _case = super::native_case();
    let (progress, events) = mpsc::channel();
    let watch = std::thread::spawn(move || watchdog::observe(&events));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let mut pool = InstancePool::start_observed(&descriptor(false), config(), |_| {
        progress.send("owner admitted").unwrap();
    })
    .unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    let claimed = configure(&dispatch);
    runtime.block_on(exercise(&dispatch, &claimed, &progress));
    let closed = pool.shutdown(GATE);
    assert!(closed.is_clean(), "{closed:?}");
    assert_eq!(closed.joined, 4);
    assert!(
        closed
            .owners
            .iter()
            .all(|o| o.roots_before_cleanup == 0 && o.image_file_removed)
    );
    progress.send("four owners joined with zero roots").unwrap();
    drop(progress);
    watch.join().unwrap();
    println!(
        "OWNER-CLAIM-PASS stalled-peer bypass quotas cancellation timeout close native-drain roots-zero"
    );
}
