//! Root-free preparation versus native construction at a held ready frontier.
use super::*;
use gerbil_scheme::instances::{SchemeServiceCompletion, SchemeServiceStreamConfig};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
struct Message {
    value: NumericServiceBatch<i64>,
    last: bool,
    projected: Arc<AtomicUsize>,
    constructed: Arc<AtomicUsize>,
    dispatch: InstanceDispatcher,
    reached: mpsc::Sender<()>,
    release: Arc<std::sync::Mutex<mpsc::Receiver<()>>>,
}
impl Message {
    fn mark_construction(&self) {
        if self.last {
            assert_eq!(
                self.projected.load(Ordering::Acquire),
                3,
                "ready results precede new native construction"
            );
        }
        self.constructed.fetch_add(1, Ordering::AcqRel);
    }
}
impl SchemeServiceRequest for Message {
    type Output = Vec<Vec<i64>>;
    fn work_units(&self) -> usize {
        8
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.mark_construction();
        self.value.construct(session)
    }
    fn construct_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &gerbil_scheme::instances::SchemeServiceInput,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.mark_construction();
        self.value.construct_service(session, input)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        let index = self.projected.fetch_add(1, Ordering::AcqRel);
        if !self.last {
            assert_eq!(self.constructed.load(Ordering::Acquire), 3);
            if index == 1 {
                // Two native jobs plus one prepared input hold three credits.
                assert_eq!(self.dispatch.owner_capacity()[0].available, 1);
                assert!((session.diagnostics().unwrap()[4] - 2.0).abs() < f64::EPSILON);
                session.collect()?;
                self.reached.send(()).unwrap();
                self.release.lock().unwrap().recv_timeout(GATE).unwrap();
            }
        }
        self.value.project(session, output)
    }
}
pub(super) fn contract(runtime: &tokio::runtime::Runtime, progress: &mpsc::Sender<&'static str>) {
    for policy in [
        SchemeServiceCompletion::PreparedCycle,
        SchemeServiceCompletion::PipelineCycle,
    ] {
        for action in ["continue", "cancel", "expire", "close"] {
            progress
                .send(match action {
                    "continue" => "PHASE prepared continue",
                    "cancel" => "PHASE prepared cancel",
                    "expire" => "PHASE prepared expire",
                    _ => "PHASE prepared close",
                })
                .unwrap();
            run(runtime, progress, action, policy);
        }
    }
}
fn run(
    runtime: &tokio::runtime::Runtime,
    progress: &mpsc::Sender<&'static str>,
    action: &str,
    policy: SchemeServiceCompletion,
) {
    let mut cfg = config();
    cfg.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), cfg).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    dispatch
        .configure_scheme_service_completion_on(0, policy)
        .unwrap()
        .wait(GATE)
        .unwrap();
    let projected = Arc::new(AtomicUsize::new(0));
    let constructed = Arc::new(AtomicUsize::new(0));
    let (reached, ready) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let wait = Arc::new(std::sync::Mutex::new(wait));
    let messages = (0..4)
        .map(|index| Message {
            value: batch(index < 3),
            last: index == 3,
            projected: Arc::clone(&projected),
            constructed: Arc::clone(&constructed),
            dispatch: dispatch.clone(),
            reached: reached.clone(),
            release: Arc::clone(&wait),
        })
        .collect();
    runtime.block_on(async {
        let timeout = if action == "expire" {
            Duration::from_millis(200)
        } else {
            GATE
        };
        let start = Instant::now();
        let mut stream = dispatch
            .submit_scheme_service_stream_on(
                messages,
                SchemeServiceStreamConfig {
                    owner: 0,
                    maximum_active: 3,
                    maximum_staged_work: 32,
                    timeout,
                },
            )
            .await
            .unwrap();
        while constructed.load(Ordering::Acquire) < 3 {
            command(&dispatch, 0, 10);
            assert!(start.elapsed() < GATE);
            tokio::task::yield_now().await;
        }
        command(&dispatch, 0, 18);
        ready.recv_timeout(GATE).unwrap();
        match action {
            "cancel" => stream.cancel(),
            "expire" => std::thread::sleep(
                timeout.saturating_sub(start.elapsed()) + Duration::from_millis(1),
            ),
            "close" => {
                assert_eq!(pool.shutdown(Duration::ZERO).pending, 1);
            }
            _ => {}
        }
        release.send(()).unwrap();
        for _ in 0..3 {
            corpus::peers(&stream.next(GATE).await.unwrap().unwrap().0);
        }
        let last = stream.next(GATE).await.unwrap();
        if action == "continue" {
            corpus::peers(&last.unwrap().0);
            assert_eq!(constructed.load(Ordering::Acquire), 4);
        } else {
            let expected = match action {
                "cancel" => InstanceError::Cancelled,
                "expire" => InstanceError::Timeout,
                _ => InstanceError::Closed,
            };
            assert_eq!(last.unwrap_err(), expected);
            assert_eq!(
                constructed.load(Ordering::Acquire),
                3,
                "rejected prepared input never constructs a Scheme graph"
            );
        }
        assert!(stream.next(GATE).await.is_none());
        if action != "close" {
            credits_restored(&dispatch).await;
        }
    });
    let report = pool.shutdown(GATE);
    assert_eq!(report.pending, 0);
    assert_eq!(report.joined, 1);
    assert_eq!(report.owners[0].roots_before_cleanup, 0);
    assert!(report.owners[0].image_file_removed);
    progress.send("bounded root-free prepared input: ready retirement precedence and cancellation/expiry/close frontier passed").unwrap();
}

async fn credits_restored(dispatch: &InstanceDispatcher) {
    idle(dispatch).await;
    let start = Instant::now();
    while dispatch.owner_capacity()[0].producer_available != 1 {
        assert!(
            start.elapsed() < GATE,
            "prepared producer registration did not return"
        );
        tokio::task::yield_now().await;
    }
}
