//! Open-source progress while later Rust preparation and result consumption pause.
use super::{Comparison, GATE, corpus, measurement, settings, stall_forensics};
use gerbil_scheme::instances::{
    InstanceClientProgressDriver, InstanceDispatcher, InstanceError, InstancePool, NativeRoot,
    NativeSession, NumericServiceBatch, SchemeServiceInput, SchemeServicePoolStreamConfig,
    SchemeServiceRequest, SchemeServiceSubmission, SchemeServiceSupplyAdmission,
};
use std::{
    collections::BTreeSet,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Instant,
};
struct Gate {
    entered: mpsc::Sender<usize>,
    release: mpsc::Receiver<()>,
    broken: Arc<AtomicBool>,
}
struct Request {
    id: usize,
    message: NumericServiceBatch<i64>,
    gate: Option<Gate>,
    projected: mpsc::Sender<usize>,
}
impl SchemeServiceRequest for Request {
    type Output = Vec<Vec<i64>>;
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
        let submission = self.message.enter_service(session, input)?;
        if let Some(gate) = self.gate.take() {
            let _ = gate.entered.send(session.owner_index());
            if gate.release.recv_timeout(GATE).is_err() {
                gate.broken.store(true, Ordering::Release);
                eprintln!("SOURCE-FIXTURE entry-release=missing native-reproduction=false");
                return Err(InstanceError::Timeout);
            }
        }
        Ok(submission)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        root: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        let rows = self.message.project(session, root)?;
        verify(self.id, &rows);
        let _ = self.projected.send(session.owner_index());
        Ok(rows)
    }
}
fn verify(id: usize, rows: &[Vec<i64>]) {
    assert_eq!(rows.len(), 8);
    for (offset, row) in rows.iter().enumerate() {
        corpus::verify(&corpus::order(id * 8 + offset, false), row);
    }
}
struct Input {
    messages: std::vec::IntoIter<Request>,
    cursor: usize,
    paused: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
    broken: Arc<AtomicBool>,
}
impl Iterator for Input {
    type Item = Result<Request, InstanceError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor == 4 {
            let _ = self.paused.send(());
            if self.release.recv_timeout(GATE).is_err() {
                self.broken.store(true, Ordering::Release);
                eprintln!("SOURCE-FIXTURE preparation-release=missing native-reproduction=false");
                return Some(Err(InstanceError::Timeout));
            }
        }
        self.cursor += 1;
        self.messages.next().map(Ok)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.messages.size_hint()
    }
}
impl ExactSizeIterator for Input {}
struct Control {
    entered: mpsc::Receiver<usize>,
    projected: mpsc::Receiver<usize>,
    paused: mpsc::Receiver<()>,
    entries: Vec<mpsc::Sender<()>>,
    preparation: mpsc::Sender<()>,
    broken: Arc<AtomicBool>,
}
fn input() -> (Input, Control) {
    let (entered, entries) = mpsc::channel();
    let (projected, projections) = mpsc::channel();
    let (paused, pause) = mpsc::channel();
    let (preparation, release) = mpsc::channel();
    let broken = Arc::new(AtomicBool::new(false));
    let mut releases = Vec::new();
    let messages = (0..8)
        .map(|id| {
            let gate = (id < 4).then(|| {
                let (release, wait) = mpsc::channel();
                releases.push(release);
                Gate {
                    entered: entered.clone(),
                    release: wait,
                    broken: Arc::clone(&broken),
                }
            });
            Request {
                id,
                message: NumericServiceBatch {
                    items: (id * 8..id * 8 + 8)
                        .map(|id| corpus::order(id, false))
                        .collect(),
                    yield_every: 1,
                },
                gate,
                projected: projected.clone(),
            }
        })
        .collect::<Vec<_>>()
        .into_iter();
    (
        Input {
            messages,
            cursor: 0,
            paused,
            release,
            broken: Arc::clone(&broken),
        },
        Control {
            entered: entries,
            projected: projections,
            paused: pause,
            entries: releases,
            preparation,
            broken,
        },
    )
}
async fn produce(dispatch: InstanceDispatcher, input: Input) {
    let mut driver = InstanceClientProgressDriver::new(std::future::pending::<()>());
    let mut stream = dispatch
        .pipeline_scheme_service_pool_ready_stream(
            input,
            SchemeServicePoolStreamConfig {
                owners: vec![0, 1, 2, 3],
                maximum_active_per_owner: 1,
                maximum_staged_work: 64,
                timeout: GATE,
            },
            SchemeServiceSupplyAdmission::ProgressFair,
            &mut driver,
        )
        .await
        .unwrap();
    stall_forensics::phase(31);
    let mut seen = BTreeSet::new();
    while let Some(row) = stream.next(GATE).await {
        let row = row.unwrap();
        assert!(seen.insert(row.sequence));
        verify(row.sequence, &row.result.unwrap());
    }
    assert_eq!(seen, (0..8).collect());
}
fn four_owners(events: &mpsc::Receiver<usize>) {
    let owners: BTreeSet<_> = (0..4).map(|_| events.recv_timeout(GATE).unwrap()).collect();
    assert_eq!(owners, (0..4).collect());
}
fn retired_while_open(pool: &InstancePool) {
    let start = Instant::now();
    loop {
        let states = pool.dispatcher().owner_capacity();
        assert!(states.iter().all(|c| c.producer_available == 0));
        if states
            .iter()
            .all(|c| c.available == 4 && c.work_available == Some(32))
        {
            return;
        }
        assert!(
            start.elapsed() < GATE,
            "native credits depend on paused publisher"
        );
        std::thread::yield_now();
    }
}
pub(super) fn run(pool: &InstancePool, runtime: &tokio::runtime::Runtime) {
    measurement::idle(pool);
    settings::configure(pool, Comparison::ProgressFair, false);
    measurement::idle(pool);
    settings::short_wake(pool, Comparison::ProgressFair, false);
    measurement::idle(pool);
    let (input, control) = input();
    let producer = runtime.spawn(produce(pool.dispatcher(), input));
    stall_forensics::phase(26);
    control.paused.recv_timeout(GATE).unwrap();
    four_owners(&control.entered);
    stall_forensics::phase(27);
    for release in control.entries {
        release.send(()).unwrap();
    }
    stall_forensics::phase(28);
    four_owners(&control.projected);
    assert!(!control.broken.load(Ordering::Acquire));
    stall_forensics::phase(29);
    retired_while_open(pool);
    stall_forensics::phase(30);
    control.preparation.send(()).unwrap();
    runtime.block_on(producer).unwrap();
    assert!(!control.broken.load(Ordering::Acquire));
    measurement::idle(pool);
    eprintln!(
        "SOURCE-HANDOFF owners=4 projected-before-publisher-resume=4 credits-returned=true result=validated historical-cause-closed=false"
    );
}
