//! Pooled supply moves only independent Rust inputs, never accepted Scheme state.
#[path = "peer_delivery.rs"]
mod peer_delivery;
use super::{GATE, batch, command, idle, reselection, short};
use gerbil_scheme::instances::{
    InstanceDispatcher, InstanceError, NativeRoot, NativeSession, NumericServiceBatch,
    SchemeRoutingAdmission, SchemeServicePoolStreamConfig, SchemeServiceRequest,
    SchemeServiceStreamConfig, SchemeServiceSupplyAdmission,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
fn config(owners: Vec<usize>, timeout: Duration) -> SchemeServicePoolStreamConfig {
    SchemeServicePoolStreamConfig {
        owners,
        maximum_active_per_owner: 2,
        maximum_staged_work: 256,
        timeout,
    }
}
struct Tagged {
    index: usize,
    owner: Option<usize>,
    message: NumericServiceBatch<i64>,
    entered: Arc<AtomicUsize>,
}
impl SchemeServiceRequest for Tagged {
    type Output = (usize, usize, usize);
    fn work_units(&self) -> usize {
        self.message.work_units()
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.entered.fetch_add(1, Ordering::SeqCst);
        self.owner = Some(session.owner_index());
        self.message.construct(session)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        assert_eq!(
            self.owner,
            Some(session.owner_index()),
            "accepted native state migrated"
        );
        let values = self.message.project(session, output)?;
        Ok((self.index, session.owner_index(), values.len()))
    }
}
fn messages(count: usize, wait: bool, entered: &Arc<AtomicUsize>) -> Vec<Tagged> {
    (0..count)
        .map(|index| {
            let mut message = batch(wait);
            if !wait && index % 2 == 0 {
                message.items.truncate(1);
            }
            Tagged {
                index,
                owner: None,
                message,
                entered: Arc::clone(entered),
            }
        })
        .collect()
}
async fn producer_free(dispatch: &InstanceDispatcher, owners: &[usize]) {
    let start = Instant::now();
    while owners
        .iter()
        .any(|&i| dispatch.owner_capacity()[i].producer_available != 1)
    {
        assert!(
            start.elapsed() < GATE,
            "shared producer registration retained"
        );
        tokio::task::yield_now().await;
    }
}
pub(super) async fn contract(dispatch: &InstanceDispatcher, progress: &mpsc::Sender<&'static str>) {
    super::service_progress::enable(dispatch).await;
    progress_route(dispatch).await;
    peer_delivery::contract(dispatch, progress).await;
    for mode in [
        SchemeServiceSupplyAdmission::Queued,
        SchemeServiceSupplyAdmission::OwnerLocal,
        SchemeServiceSupplyAdmission::RetirementRefill,
        SchemeServiceSupplyAdmission::ProgressFair,
    ] {
        recovery(dispatch, mode).await;
        redistribute(dispatch, mode).await;
        progress.send("pooled supply: all admission policies preserve recovery, heterogeneous charges, FIFO identity and native affinity").unwrap();
        registration(dispatch, mode).await;
        progress.send("pooled supply: all admission policies cancel and expire partial registration before native entry").unwrap();
        cancellation(dispatch, false, mode).await;
        cancellation(dispatch, true, mode).await;
        progress.send("pooled supply: all admission policies preserve active bounds, cancellation/expiry and accepted drain").unwrap();
    }
}
async fn recovery(dispatch: &InstanceDispatcher, mode: SchemeServiceSupplyAdmission) {
    let messages = [
        (vec![0, 1], false),
        (vec![-1], false),
        (vec![1], false),
        (vec![0, 1], true),
        (vec![0, 2], false),
    ]
    .into_iter()
    .map(|(input, panic)| super::stream::Independent(super::VectorRequest { input, panic }))
    .collect();
    let mut stream = dispatch
        .submit_scheme_service_pool_stream_with_admission(
            messages,
            config(vec![0, 1, 2, 3], GATE),
            mode,
        )
        .await
        .unwrap();
    assert_eq!(stream.next(GATE).await.unwrap().unwrap().0, [10]);
    assert_eq!(
        stream.next(GATE).await.unwrap().unwrap_err(),
        InstanceError::TaskPanicked
    );
    assert!(matches!(
        stream.next(GATE).await.unwrap(),
        Err(InstanceError::Scheme(_))
    ));
    assert_eq!(
        stream.next(GATE).await.unwrap().unwrap_err(),
        InstanceError::TaskPanicked
    );
    assert_eq!(stream.next(GATE).await.unwrap().unwrap().0, [13]);
    producer_free(dispatch, &[0, 1, 2, 3]).await;
    idle(dispatch).await;
}
async fn progress_route(dispatch: &InstanceDispatcher) {
    command(dispatch, 0, 9);
    let held = dispatch
        .submit_scheme_service_stream_on(
            vec![batch(true)],
            SchemeServiceStreamConfig {
                owner: 0,
                maximum_active: 1,
                maximum_staged_work: 8,
                timeout: GATE,
            },
        )
        .await
        .unwrap();
    let start = Instant::now();
    while command(dispatch, 0, 10)[0] < 4 {
        assert!(start.elapsed() < GATE);
        tokio::task::yield_now().await;
    }
    // The ordinary ledger cannot see this direct resident producer.
    let old = reselection::router(dispatch, SchemeRoutingAdmission::Reselect)
        .submit_short(short::Short(vec![0, 0, 1, 1, 1]), GATE)
        .await
        .unwrap();
    assert_eq!(old.owner, 0);
    old.ticket.wait_completed(GATE).await.unwrap();
    let new = reselection::router(dispatch, SchemeRoutingAdmission::ReselectProgress)
        .submit_short(short::Short(vec![0, 0, 1, 1, 1]), GATE)
        .await
        .unwrap();
    assert_eq!(
        new.owner, 1,
        "native producer occupancy was invisible to short routing"
    );
    new.ticket.wait_completed(GATE).await.unwrap();
    drop(held);
    command(dispatch, 0, 8);
    producer_free(dispatch, &[0]).await;
    idle(dispatch).await;
}
async fn redistribute(dispatch: &InstanceDispatcher, mode: SchemeServiceSupplyAdmission) {
    idle(dispatch).await;
    let (release, occupied) = reselection::fill(dispatch, 0);
    let entered = Arc::new(AtomicUsize::new(0));
    let mut stream = dispatch
        .submit_scheme_service_pool_stream_with_admission(
            messages(24, false, &entered),
            config(vec![0, 1, 2, 3], GATE),
            mode,
        )
        .await
        .unwrap();
    let router = reselection::router(dispatch, SchemeRoutingAdmission::ReselectProgress);
    let short = router
        .submit_short(short::Short(vec![0, 0, 1, 1, 1]), GATE)
        .await
        .unwrap();
    assert_ne!(short.owner, 0, "short route chose exhausted owner");
    short.ticket.wait_completed(GATE).await.unwrap();
    for index in 0..24 {
        let (identity, owner, len) = stream.next(GATE).await.unwrap().unwrap().0;
        assert_eq!(identity, index);
        assert_ne!(owner, 0, "uncommitted work waited for blocked owner");
        assert_eq!(len, if index % 2 == 0 { 1 } else { 8 });
    }
    assert!(stream.next(GATE).await.is_none());
    producer_free(dispatch, &[1, 2, 3]).await;
    assert_eq!(entered.load(Ordering::SeqCst), 24);
    release.send(()).unwrap();
    for ticket in occupied {
        ticket.wait(GATE).unwrap();
    }
    producer_free(dispatch, &[0]).await;
    idle(dispatch).await;
}
async fn registration(dispatch: &InstanceDispatcher, mode: SchemeServiceSupplyAdmission) {
    command(dispatch, 3, 9);
    let held = dispatch
        .submit_scheme_service_stream_on(
            vec![batch(true)],
            SchemeServiceStreamConfig {
                owner: 3,
                maximum_active: 1,
                maximum_staged_work: 8,
                timeout: GATE,
            },
        )
        .await
        .unwrap();
    let entered = Arc::new(AtomicUsize::new(0));
    let mut pending = Box::pin(dispatch.submit_scheme_service_pool_stream_with_admission(
        messages(2, false, &entered),
        config(vec![3, 0], GATE),
        mode,
    ));
    reselection::registered(pending.as_mut()).await;
    assert_eq!(dispatch.owner_capacity()[0].producer_available, 0);
    assert_eq!(
        entered.load(Ordering::SeqCst),
        0,
        "partial registration executed work"
    );
    drop(pending);
    producer_free(dispatch, &[0]).await;
    assert_eq!(entered.load(Ordering::SeqCst), 0);
    let timed = dispatch
        .submit_scheme_service_pool_stream_with_admission(
            messages(2, false, &entered),
            config(vec![0, 3], Duration::from_millis(20)),
            mode,
        )
        .await;
    assert!(matches!(timed, Err(InstanceError::Timeout)));
    producer_free(dispatch, &[0]).await;
    assert_eq!(entered.load(Ordering::SeqCst), 0);
    drop(held);
    command(dispatch, 3, 8);
    producer_free(dispatch, &[3]).await;
    idle(dispatch).await;
}
async fn cancellation(
    dispatch: &InstanceDispatcher,
    expire: bool,
    mode: SchemeServiceSupplyAdmission,
) {
    for owner in [0, 1] {
        command(dispatch, owner, 9);
    }
    let entered = Arc::new(AtomicUsize::new(0));
    let timeout = if expire {
        Duration::from_millis(300)
    } else {
        GATE
    };
    let mut settings = config(vec![0, 1], timeout);
    settings.maximum_active_per_owner = 1;
    let mut stream = dispatch
        .submit_scheme_service_pool_stream_with_admission(
            messages(6, true, &entered),
            settings,
            mode,
        )
        .await
        .unwrap();
    let start = Instant::now();
    while entered.load(Ordering::SeqCst) < 2 {
        assert!(start.elapsed() < GATE);
        tokio::task::yield_now().await;
    }
    assert_eq!(entered.load(Ordering::SeqCst), 2);
    if expire {
        tokio::time::sleep(Duration::from_millis(350)).await;
    } else {
        stream.cancel();
    }
    // Observe rejection before releasing any accepted native graph.
    let expected = if expire {
        InstanceError::Timeout
    } else {
        InstanceError::Cancelled
    };
    for owner in [0, 1] {
        command(dispatch, owner, 8);
    }
    let mut accepted = 0;
    let mut rejected = 0;
    for _ in 0..6 {
        match stream.next(GATE).await.unwrap() {
            Ok(_) => accepted += 1,
            Err(error) => {
                assert_eq!(error, expected);
                rejected += 1;
            }
        }
    }
    assert_eq!((accepted, rejected), (2, 4));
    producer_free(dispatch, &[0, 1]).await;
    idle(dispatch).await;
}
