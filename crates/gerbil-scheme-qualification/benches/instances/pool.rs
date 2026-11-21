//! Diagnostic Tokio admission and dedicated, owner-affine Scheme workers.

use super::image::Image;
use std::{
    collections::BTreeSet,
    ffi::c_void,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
struct EntryGate {
    entered: AtomicUsize,
    failed: AtomicBool,
    workers: usize,
}

#[repr(C)]
struct Context {
    entered: extern "C" fn(*mut c_void),
    data: *mut c_void,
}

extern "C" fn scheme_entered(data: *mut c_void) {
    // SAFETY: run_batch holds the Arc alive until every response is drained.
    let gate = unsafe { &*data.cast::<EntryGate>() };
    gate.entered.fetch_add(1, Ordering::AcqRel);
    let deadline = Instant::now() + Duration::from_secs(5);
    while gate.entered.load(Ordering::Acquire) != gate.workers {
        if Instant::now() >= deadline {
            gate.failed.store(true, Ordering::Release);
            return;
        }
        thread::yield_now();
    }
}

#[derive(Debug)]
struct Request {
    jobs: u32,
    rounds: u32,
    gate: Option<Arc<EntryGate>>,
    response: oneshot::Sender<(i64, u32)>,
}

#[derive(Debug)]
pub(super) struct Pool {
    senders: Vec<mpsc::Sender<Request>>,
    threads: Vec<thread::JoinHandle<()>>,
    host: tokio::runtime::Runtime,
}

impl Pool {
    pub(super) fn new(paths: Vec<PathBuf>) -> Self {
        let workers = paths.len();
        let init = Arc::new(Mutex::new(()));
        let mut senders = Vec::new();
        let mut threads = Vec::new();
        let (ready, identities) = std::sync::mpsc::channel();
        for (index, path) in paths.into_iter().enumerate() {
            let (sender, mut receiver) = mpsc::channel::<Request>(workers);
            senders.push(sender);
            let init = Arc::clone(&init);
            let ready = ready.clone();
            threads.push(thread::spawn(move || {
                // Setup/cleanup install process-wide OS hooks. Serialize only
                // lifecycle transitions; no lock surrounds Scheme execution.
                let image = {
                    let _guard = init.lock().expect("lifecycle lock");
                    Image::load(&path)
                };
                ready.send(image.state).expect("owner identity");
                drop(ready);
                let id = u32::try_from(index + 1).expect("worker id");
                while let Some(request) = receiver.blocking_recv() {
                    let mut context = request.gate.as_ref().map(|gate| Context {
                        entered: scheme_entered,
                        data: Arc::as_ptr(gate).cast_mut().cast(),
                    });
                    let context = context
                        .as_mut()
                        .map_or(std::ptr::null_mut(), |ctx| std::ptr::from_mut(ctx).cast());
                    let result = image.run(id, request.jobs, request.rounds, context);
                    let _ = request.response.send((result, image.marker()));
                }
                let _guard = init.lock().expect("cleanup lock");
                drop(image);
            }));
        }
        drop(ready);
        let mut states = BTreeSet::new();
        for _ in 0..workers {
            states.insert(identities.recv_timeout(Duration::from_secs(5)).expect(
                "VM initialization must complete; do not widen the five-second startup gate",
            ));
        }
        assert_eq!(
            states.len(),
            workers,
            "images must not share Gambit global state"
        );
        eprintln!(
            "INSTANCE-IDENTITIES workers={workers} distinct-gstates={}",
            states.len()
        );
        Self {
            senders,
            threads,
            host: tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .expect("Tokio admission"),
        }
    }

    pub(super) fn batch(&self, jobs: u32, rounds: u32, prove_entry: bool) -> i64 {
        let workers = self.senders.len();
        let gate = prove_entry.then(|| {
            Arc::new(EntryGate {
                entered: AtomicUsize::new(0),
                failed: AtomicBool::new(false),
                workers,
            })
        });
        let sum = self.host.block_on(async {
            let mut responses = Vec::new();
            for (index, sender) in self.senders.iter().enumerate() {
                let jobs = jobs / u32::try_from(workers).expect("worker count")
                    + u32::from(index < usize::try_from(jobs).expect("job count") % workers);
                let (response, result) = oneshot::channel();
                sender
                    .send(Request {
                        jobs,
                        rounds,
                        gate: gate.clone(),
                        response,
                    })
                    .await
                    .expect("worker admission");
                responses.push((index, result));
            }
            let mut sum = 0;
            for (index, response) in responses {
                let (result, marker) = if let Ok(response) = tokio::time::timeout(Duration::from_secs(5), response).await {
                    response.expect("worker response")
                } else {
                        // This diagnostic must fail immediately, not hang in
                        // Drop joining an unresponsive foreign call. No clean
                        // drain or successful execution is reported on timeout.
                        eprintln!("INSTANCE-STALL owner={index} five-second-response-gate=FAILED cleanup=UNVERIFIED");
                        std::process::exit(70);
                };
                assert_eq!(
                    marker,
                    u32::try_from(index + 1).expect("worker marker"),
                    "Scheme globals must be isolated across GC"
                );
                assert!(result >= 0, "Scheme exception is not a successful result");
                sum += result;
            }
            sum
        });
        if let Some(gate) = gate {
            assert!(
                !gate.failed.load(Ordering::Acquire),
                "simultaneous Scheme entry timed out; never label queue callers as parallel VMs"
            );
            assert_eq!(gate.entered.load(Ordering::Acquire), workers);
            eprintln!("INSTANCE-ENTRY inside-scheme={workers} gc-and-global-isolation=OK");
        }
        sum
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.senders.clear();
        for thread in self.threads.drain(..) {
            thread.join().expect("VM owner shutdown");
        }
        eprintln!("INSTANCE-DRAIN owners-joined=OK images-retained-until-process-exit=true");
    }
}
