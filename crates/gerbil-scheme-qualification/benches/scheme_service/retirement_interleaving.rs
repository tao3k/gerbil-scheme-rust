//! Controlled projection/registration/clock interleaving, no delayed release.
use super::{Comparison, GATE, corpus, measurement, settings, stall_forensics};
use gerbil_scheme::instances::{
    InstanceArrivalClock, InstanceClientProgressDriver, InstanceDispatcher, InstanceError,
    InstancePool, NativeRoot, NativeSession, NumericServiceBatch, SchemeServiceInput,
    SchemeServicePoolStreamConfig, SchemeServiceReadyStream, SchemeServiceRequest,
    SchemeServiceSubmission, SchemeServiceSupplyAdmission,
};
use std::{
    future::Future,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    task::Poll,
    time::Instant,
};
use tokio::sync::oneshot;
type Rows = Vec<Vec<i64>>;
struct ProjectionGate {
    entered: oneshot::Sender<()>,
    release: mpsc::Receiver<()>,
}
struct FinalProjection {
    remaining: AtomicUsize,
    gate: Mutex<Option<ProjectionGate>>,
}
struct Request {
    message: NumericServiceBatch<i64>,
    gate: Option<Arc<FinalProjection>>,
}
impl SchemeServiceRequest for Request {
    type Output = Rows;
    fn work_units(&self) -> usize {
        self.message.work_units()
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.message.construct(session)
    }
    fn enter_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &SchemeServiceInput<'_>,
    ) -> Result<SchemeServiceSubmission<'vm>, InstanceError> {
        self.message.enter_service(session, input)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        root: &NativeRoot<'_>,
    ) -> Result<Rows, InstanceError> {
        let value = self.message.project(session, root)?;
        if let Some(gate) = self
            .gate
            .filter(|g| g.remaining.fetch_sub(1, Ordering::AcqRel) == 1)
        {
            let gate = gate.gate.lock().unwrap().take().unwrap();
            let _ = gate.entered.send(());
            // Released immediately after observing a real registration wait.
            // Timeout/drop is a fixture error, never a reproduced native stall.
            if gate.release.recv_timeout(GATE).is_err() {
                eprintln!("INTERLEAVING-FIXTURE release=missing native-reproduction=false");
                return Err(InstanceError::Timeout);
            }
        }
        Ok(value)
    }
}
fn message(id: usize, gate: Option<Arc<FinalProjection>>) -> Request {
    Request {
        message: NumericServiceBatch {
            items: (id..id + 8).map(|id| corpus::order(id, true)).collect(),
            yield_every: 1,
        },
        gate,
    }
}
fn config(owner: usize) -> SchemeServicePoolStreamConfig {
    SchemeServicePoolStreamConfig {
        owners: vec![owner],
        maximum_active_per_owner: 3,
        maximum_staged_work: 1024,
        timeout: GATE,
    }
}
fn verify(id: usize, rows: &Rows) {
    assert_eq!(rows.len(), 8);
    for (offset, row) in rows.iter().enumerate() {
        corpus::verify(&corpus::order(id + offset, true), row);
    }
}
async fn registration_race(
    dispatch: InstanceDispatcher,
    owner: usize,
) -> SchemeServiceReadyStream<Rows> {
    let (entered, at_projection) = oneshot::channel();
    let (release, wait) = mpsc::channel();
    let gate = Arc::new(FinalProjection {
        remaining: AtomicUsize::new(3),
        gate: Mutex::new(Some(ProjectionGate {
            entered,
            release: wait,
        })),
    });
    stall_forensics::phase(13);
    let old = dispatch
        .submit_scheme_service_pool_ready_stream(
            [0, 8, 16]
                .into_iter()
                .map(|id| message(id, Some(Arc::clone(&gate))))
                .collect(),
            config(owner),
            SchemeServiceSupplyAdmission::ProgressFair,
        )
        .await
        .unwrap();
    drop(gate); // Native jobs alone retain the signal; failures cannot strand the fixture.
    at_projection.await.unwrap();
    assert_eq!(dispatch.owner_capacity()[owner].producer_available, 0);
    let mut next = Box::pin(dispatch.submit_scheme_service_pool_ready_stream(
        vec![message(24, None)],
        config(owner),
        SchemeServiceSupplyAdmission::ProgressFair,
    ));
    stall_forensics::phase(14);
    std::future::poll_fn(|cx| {
        assert!(
            next.as_mut().poll(cx).is_pending(),
            "old producer is still live"
        );
        Poll::Ready(())
    })
    .await;
    release.send(()).unwrap();
    stall_forensics::phase(15);
    let mut clock = InstanceArrivalClock::new().unwrap();
    let mut driver = InstanceClientProgressDriver::new(clock.wait_until(Instant::now()));
    stall_forensics::phase(16);
    let mut next = driver.wait(next).await.unwrap();
    stall_forensics::phase(17);
    let row = driver.wait(next.next(GATE)).await.unwrap().unwrap();
    verify(24, &row.result.unwrap());
    assert!(driver.wait(next.next(GATE)).await.is_none());
    driver.finish().await.unwrap();
    old // Intentionally unread: result consumption cannot release registration.
}
pub(super) fn run(pool: &InstancePool, runtime: &tokio::runtime::Runtime) {
    for owner in 0..4 {
        eprintln!("INTERLEAVING owner={owner} fixture=projection-registration-clock");
        let dispatch = pool.dispatcher();
        let mut old = runtime
            .block_on(runtime.spawn(registration_race(dispatch, owner)))
            .unwrap();
        stall_forensics::phase(18);
        measurement::idle(pool); // Includes producer retirement, not just outputs.
        stall_forensics::phase(19);
        settings::configure(pool, Comparison::ProgressFair, false);
        // Match the historical control-reply/native-retirement fence exactly.
        measurement::idle(pool);
        settings::short_wake(pool, Comparison::ProgressFair, false);
        measurement::idle(pool);
        stall_forensics::phase(20);
        let dispatch = pool.dispatcher();
        runtime.block_on(async move {
            let reply = dispatch
                .submit_scheme_service_backpressured_on(owner, message(32, None), GATE)
                .await
                .unwrap()
                .wait_completed(GATE)
                .await
                .unwrap();
            verify(32, &reply.0);
        });
        stall_forensics::phase(21);
        let mut seen = Vec::new();
        for _ in 0..3 {
            let row = runtime.block_on(old.next(GATE)).unwrap().unwrap();
            assert!(row.sequence < 3);
            verify(row.sequence * 8, &row.result.unwrap());
            seen.push(row.sequence);
        }
        seen.sort_unstable();
        assert_eq!(seen, [0, 1, 2]);
        assert!(runtime.block_on(old.next(GATE)).is_none());
        measurement::idle(pool);
        eprintln!("INTERLEAVING owner={owner} result=validated historical-cause-closed=false");
    }
}
