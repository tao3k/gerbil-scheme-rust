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
    for reclaim in [false, true] {
        case(dispatch, progress, reclaim).await;
    }
}
async fn case(dispatch: &InstanceDispatcher, progress: &mpsc::Sender<&'static str>, reclaim: bool) {
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
    only_entered_credit_is_reserved(dispatch);
    // The dedicated slot holds only Rust input. Native capacity remains available
    // while the target is blocked inside its earlier native constructor.
    if reclaim {
        let before = dispatch
            .scheme_service_progress_on(1)
            .unwrap()
            .peer_withdrawals;
        command(dispatch, 0, 8);
        let start = Instant::now();
        while dispatch
            .scheme_service_progress_on(1)
            .unwrap()
            .peer_withdrawals
            == before
        {
            assert!(
                start.elapsed() < GATE,
                "ready native owner did not reclaim peer offer"
            );
            tokio::task::yield_now().await;
        }
    }
    stream.cancel();
    slot_cleared_before_release(dispatch).await;
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
    assert!(accepted >= if reclaim { 4 } else { 2 });
    producer_free(dispatch, &[0, 1]).await;
    idle(dispatch).await;
    assert_eq!(
        dispatch
            .scheme_service_progress_on(1)
            .unwrap()
            .inbox_remaining,
        0
    );
    progress.send("dedicated peer slot: blocked native owner retains all unentered credits; cancel/reclaim clears slot before release; entered Scheme work drains").unwrap();
}

async fn slot_cleared_before_release(dispatch: &InstanceDispatcher) {
    let start = Instant::now();
    loop {
        let p = dispatch.scheme_service_progress_on(1).unwrap();
        let capacity = dispatch.owner_capacity()[1].clone();
        if p.inbox_remaining == 0
            && !p.pending_offer
            && capacity.available == capacity.limit - 1
            && capacity.work_available == capacity.work_limit.map(|n| n - 8)
        {
            break;
        }
        assert!(
            start.elapsed() < GATE,
            "unentered offer credits were not fully returned"
        );
        tokio::task::yield_now().await;
    }
}

fn only_entered_credit_is_reserved(dispatch: &InstanceDispatcher) {
    assert!(
        dispatch
            .scheme_service_progress_on(1)
            .unwrap()
            .pending_offer
    );
    let occupied = dispatch.owner_capacity()[1].clone();
    assert_eq!(occupied.available, occupied.limit - 1);
    assert_eq!(occupied.work_available, occupied.work_limit.map(|n| n - 8));
}
