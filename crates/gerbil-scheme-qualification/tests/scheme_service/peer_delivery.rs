//! A peer's OS owner is deliberately occupied after registration, not emulated.
use super::{GATE, batch, command, config, idle, producer_free};
use gerbil_scheme::instances::{
    InstanceDispatcher, InstanceError, NativeRoot, NativeSession, NumericServiceBatch,
    SchemeServiceRequest, SchemeServiceSupplyAdmission,
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Instant,
};
struct Blocking {
    first: Arc<AtomicBool>,
    entered: mpsc::Sender<()>,
    release: Arc<Mutex<mpsc::Receiver<()>>>,
    native: NumericServiceBatch<i64>,
}
impl SchemeServiceRequest for Blocking {
    type Output = Vec<Vec<i64>>;
    fn work_units(&self) -> usize {
        self.native.work_units()
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        if session.owner_index() == 0 {
            let start = Instant::now();
            while !self.first.load(Ordering::Acquire) {
                assert!(
                    start.elapsed() < GATE,
                    "peer did not enter its local constructor"
                );
                std::thread::yield_now();
            }
        }
        if session.owner_index() == 1 && !self.first.swap(true, Ordering::AcqRel) {
            self.entered.send(()).unwrap();
            self.release.lock().unwrap().recv_timeout(GATE).unwrap();
        }
        self.native.construct(session)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        self.native.project(session, output)
    }
}
pub(super) async fn contract(dispatch: &InstanceDispatcher, progress: &mpsc::Sender<&'static str>) {
    idle(dispatch).await;
    for owner in [0, 1] {
        command(dispatch, owner, 9);
    }
    let first = Arc::new(AtomicBool::new(false));
    let (entered, ready) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let blocked = Arc::new(Mutex::new(blocked));
    let messages = (0..16)
        .map(|_| Blocking {
            first: Arc::clone(&first),
            entered: entered.clone(),
            release: Arc::clone(&blocked),
            native: batch(true),
        })
        .collect();
    let before = dispatch
        .scheme_service_progress_on(1)
        .unwrap()
        .peer_deliveries;
    let mut stream = dispatch
        .submit_scheme_service_pool_stream_with_admission(
            messages,
            config(vec![0, 1], GATE),
            SchemeServiceSupplyAdmission::ProgressFair,
        )
        .await
        .unwrap();
    ready.recv_timeout(GATE).unwrap();
    let start = Instant::now();
    while dispatch
        .scheme_service_progress_on(1)
        .unwrap()
        .peer_deliveries
        == before
    {
        assert!(start.elapsed() < GATE, "no real peer inbox delivery");
        tokio::task::yield_now().await;
    }
    assert!(
        !dispatch
            .scheme_service_progress_on(1)
            .unwrap()
            .waiting_for_claim
    );
    let occupied = dispatch.owner_capacity()[1].clone();
    assert_eq!(occupied.available, occupied.limit - 2);
    assert_eq!(occupied.work_available, occupied.work_limit.map(|n| n - 16));
    // Cancellation rejects only the shared source. The queued accepted peer entry
    // must keep its credits and execute after its actual native owner resumes.
    stream.cancel();
    assert_eq!(
        dispatch
            .scheme_service_progress_on(1)
            .unwrap()
            .inbox_remaining,
        8
    );
    release.send(()).unwrap();
    for owner in [0, 1] {
        command(dispatch, owner, 8);
    }
    let mut accepted = 0;
    while let Some(result) = stream.next(GATE).await {
        match result {
            Ok(_) => accepted += 1,
            Err(InstanceError::Cancelled) => {}
            Err(e) => panic!("peer drain: {e:?}"),
        }
    }
    assert!(accepted >= 3);
    producer_free(dispatch, &[0, 1]).await;
    idle(dispatch).await;
    assert_eq!(
        dispatch
            .scheme_service_progress_on(1)
            .unwrap()
            .inbox_remaining,
        0
    );
    progress.send("actual peer inbox: native owner blocked, all budgets charged, cancellation drains accepted entry and clears staged hint").unwrap();
}
