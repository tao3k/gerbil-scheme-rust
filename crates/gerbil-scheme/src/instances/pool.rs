//! Fixed native owner construction, queue draining and bounded joins.
use super::{
    InstanceConfig, InstanceDispatcher, InstanceError, InstanceReply, InstanceTicket,
    IsolatedProgram, OwnerReceipt, OwnerShutdown, ShutdownReceipt,
    image::Image,
    mapping::{ImageMappingState, Mapping},
};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::{
    collections::BTreeSet,
    ffi::{CString, OsString},
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

pub(super) static LIFECYCLE: Mutex<()> = Mutex::new(());
fn create_owner_directory(root: &Path) -> Result<PathBuf, InstanceError> {
    // A PID/counter can recur after process exit while a quarantined directory
    // still exists. Atomically reserve a new private namespace; never delete or
    // reuse files whose native owner lifetime belongs to another pool/process.
    let template = root.join(format!("gerbil-owners-{}-XXXXXX", std::process::id()));
    let mut path = CString::new(template.as_os_str().as_bytes())
        .map_err(|error| InstanceError::Loader(error.to_string()))?
        .into_bytes_with_nul();
    // SAFETY: path is a uniquely borrowed, writable, NUL-terminated template
    // ending in six X bytes. mkdtemp changes only that buffer and creates 0700.
    if unsafe { libc::mkdtemp(path.as_mut_ptr().cast()) }.is_null() {
        return Err(InstanceError::Loader(
            std::io::Error::last_os_error().to_string(),
        ));
    }
    path.pop(); // Remove only the CString terminator; preserve native path bytes.
    Ok(OsString::from_vec(path).into())
}
#[derive(Debug)]
struct OwnerThread {
    worker: thread::JoinHandle<()>,
    mapping: Arc<Mapping>,
    resources: Vec<Arc<dyn std::fmt::Debug + Send + Sync>>,
}

#[derive(Debug)]
pub(super) struct Request {
    pub(super) started: Arc<AtomicBool>,
    pub(super) work: Work,
    #[cfg(feature = "tokio-dispatch")]
    pub(super) capacity: Option<super::capacity::CapacityPermit>,
    #[cfg(feature = "tokio-dispatch")]
    pub(super) work_units: usize,
}
pub(super) enum Work {
    Bytes {
        bytes: Vec<u8>,
        response: mpsc::Sender<super::types::Completed<InstanceReply>>,
    },
    Native(Box<dyn FnOnce(&Image) + Send>),
    #[cfg(feature = "tokio-dispatch")]
    Control(Box<dyn FnOnce(&Image) + Send>),
    #[cfg(feature = "tokio-dispatch")]
    Service(Box<dyn super::service::Job>),
    #[cfg(feature = "tokio-dispatch")]
    Producer(Box<dyn super::batch_stream_runner::Producer>),
    #[cfg(feature = "tokio-dispatch")]
    Wake,
}
impl std::fmt::Debug for Work {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Bytes { .. } => "ByteRequest",
            Self::Native(_) => "NativeTypeJob",
            #[cfg(feature = "tokio-dispatch")]
            Self::Control(_) => "NativeControl",
            #[cfg(feature = "tokio-dispatch")]
            Self::Service(_) => "SchemeServiceJob",
            #[cfg(feature = "tokio-dispatch")]
            Self::Producer(_) => "BoundedBatchProducer",
            #[cfg(feature = "tokio-dispatch")]
            Self::Wake => "OwnerWake",
        })
    }
}

/// Explicit image backend. No fallback to the process-global runtime.
#[derive(Debug)]
pub struct InstancePool {
    dispatcher: InstanceDispatcher,
    threads: Vec<Option<OwnerThread>>,
    done: mpsc::Receiver<OwnerShutdown>,
    joins: Option<mpsc::Receiver<bool>>,
    stopped: Vec<OwnerShutdown>,
    mappings: Vec<Arc<super::mapping::Mapping>>,
    receipts: Vec<OwnerReceipt>,

    joined: usize,
    panicked: usize,
    /// Process-owned copied images. Known clean shutdown removes these paths.
    pub image_directory: PathBuf,
}

impl InstancePool {
    /// Observe whether this process attempted any private-image native setup.
    #[must_use]
    pub fn has_been_initialized() -> bool {
        super::image::INITIALIZATION_ATTEMPTED.load(Ordering::Acquire)
    }

    /// Initialize fixed native owners. Failure closes/drains owners already started.
    /// # Errors
    /// Rejects invalid bounds, incompatible process backend, image/ABI failures,
    /// startup timeout or non-distinct runtime states. Failed setup is never retried.
    pub fn start(program: &IsolatedProgram, config: InstanceConfig) -> Result<Self, InstanceError> {
        Self::start_observed(program, config, |_| {})
    }

    /// Initialize owners and report each successful native admission on the caller.
    /// The callback receives validated metadata, never a Scheme reference. It runs
    /// once per owner, after SDK setup and before starting the next owner.
    /// # Errors
    /// Preserves every `start` admission, native setup and per-owner timeout error.
    /// # Panics
    /// If the observer panics. Pool drop starts draining admitted owners.
    pub fn start_observed(
        program: &IsolatedProgram,
        config: InstanceConfig,
        mut ready: impl FnMut(&OwnerReceipt),
    ) -> Result<Self, InstanceError> {
        #[cfg(not(feature = "tokio-dispatch"))]
        if program.service.is_some() {
            return Err(InstanceError::InvalidConfig(
                "Scheme service requires tokio-dispatch",
            ));
        }
        if program.service.as_ref().is_some_and(|name| {
            name == &program.entry
                || program.exports.contains(name)
                || program.finalizer.as_ref() == Some(name)
        }) {
            return Err(InstanceError::InvalidConfig(
                "Scheme service protocol entry must remain private",
            ));
        }
        if program.service.is_some()
            && (config.maximum_request_bytes < 128
                || config.maximum_reply_bytes < 8
                || config.queue_capacity > 65536)
        {
            return Err(InstanceError::InvalidConfig(
                "Scheme service protocol bounds",
            ));
        }
        if config.owners == 0
            || config.owners > 64
            || config.queue_capacity == 0
            || config.maximum_request_bytes == 0
            || config.maximum_reply_bytes == 0
            || config.maximum_reply_bytes > isize::MAX as usize
            || config.maximum_request_bytes > isize::MAX as usize
            || config.startup_timeout.is_zero()
        {
            return Err(InstanceError::InvalidConfig(
                "nonzero bounds and 1..=64 owners required",
            ));
        }
        if !crate::instance_backend::claim_images() {
            return Err(InstanceError::BackendConflict);
        }
        let directory = create_owner_directory(&std::env::temp_dir())?;
        let (finished, done) = mpsc::channel();
        let mut pool = Self {
            dispatcher: InstanceDispatcher::new(
                config.maximum_request_bytes,
                config.maximum_reply_bytes,
                config.queue_capacity,
            ),
            threads: Vec::new(),
            done,
            joins: None,
            stopped: Vec::new(),
            mappings: Vec::new(),
            receipts: Vec::new(),

            joined: 0,
            panicked: 0,
            image_directory: directory,
        };
        let mut states = BTreeSet::new();
        for owner in 0..config.owners {
            if let Err(error) = pool.start_owner(program, config, owner, &finished, &mut states) {
                let shutdown = pool.shutdown(config.startup_timeout);
                if shutdown.pending == 0 {
                    let _ = fs::remove_dir(&pool.image_directory);
                }
                return Err(InstanceError::Startup {
                    cause: Box::new(error),
                    shutdown,
                    image_directory: pool.image_directory.clone(),
                });
            }
            ready(pool.receipts.last().expect("validated owner admission"));
        }
        #[cfg(feature = "tokio-dispatch")]
        pool.dispatcher.seal_owner_ports();
        Ok(pool)
    }

    fn start_owner(
        &mut self,
        program: &IsolatedProgram,
        config: InstanceConfig,
        owner: usize,
        finished: &mpsc::Sender<OwnerShutdown>,
        states: &mut BTreeSet<usize>,
    ) -> Result<(), InstanceError> {
        let identity = super::image::next_identity()?;
        let path = self.image_directory.join(format!("owner-{owner}.dylib"));
        let (wake_reader, wake) = if program.service.is_some() {
            let (reader, signal) = super::service_wake::Wake::create()?;
            (Some(reader), Some(signal))
        } else {
            (None, None)
        };
        // Allocate wake resources before creating an image copy: failure must
        // leave neither a descriptor nor an untracked copied image behind.
        fs::copy(&program.path, &path).map_err(|e| {
            let _ = fs::remove_file(&path);
            InstanceError::Loader(e.to_string())
        })?;
        let owner_wake = wake.clone();
        let (sender, requests) = mpsc::sync_channel::<Request>(config.queue_capacity);
        let (ready, initialized) = mpsc::channel();
        let program = program.clone();
        let finished = finished.clone();
        let failed_copy = path.clone();
        let mapping = super::mapping::Mapping::new(owner, identity, program.unload_after_join);
        let owner_mapping = Arc::clone(&mapping);
        let worker = thread::Builder::new()
            .name(format!("gerbil-owner-{owner}"))
            .spawn(move || {
                super::owner_thread::run(super::owner_thread::Task {
                    path,
                    program,
                    owner,
                    mapping: owner_mapping,
                    config,
                    ready,
                    finished,
                    wake_reader,
                    wake: owner_wake,
                    requests,
                });
            })
            .map_err(|e| {
                let _ = fs::remove_file(&failed_copy);
                InstanceError::Loader(e.to_string())
            })?;
        self.dispatcher.add_sender(sender, wake);
        self.mappings.push(Arc::clone(&mapping));
        self.threads.push(Some(OwnerThread {
            worker,
            mapping,
            resources: Vec::new(),
        }));
        let receipt = initialized
            .recv_timeout(config.startup_timeout)
            .map_err(|_| InstanceError::Timeout)??;
        if !states.insert(receipt.state) {
            return Err(InstanceError::InvalidConfig(
                "runtime state aliases another owner",
            ));
        }
        self.receipts.push(receipt);
        Ok(())
    }

    /// Retain Rust-owned data until every owner completes its actual native join.
    /// Register resources before admitting work that uses them. Destructors must
    /// not re-enter the retiring VM. Failed cleanup/panic retains its resources
    /// with the quarantined owner; this never authorizes native code unload.
    /// Destructor panics are reported as retirement failures, without abandoning peers.
    /// # Errors
    /// Rejects registration after owner joining has started.
    pub fn retain_owner_resource<T>(&mut self, resource: &Arc<T>) -> Result<(), InstanceError>
    where
        T: std::fmt::Debug + Send + Sync + 'static,
    {
        if self.joins.is_some() || self.threads.iter().any(Option::is_none) {
            return Err(InstanceError::Closed);
        }
        for owner in self.threads.iter_mut().flatten() {
            owner.resources.push(resource.clone());
        }
        Ok(())
    }

    /// Read initialization receipts without entering Scheme.
    #[must_use]
    pub fn owners(&self) -> &[OwnerReceipt] {
        &self.receipts
    }

    /// Round-robin admission of a stateless request.
    /// # Errors
    /// Returns closed, queue-full or request-bound rejection; there is no fallback.
    pub fn submit(&self, bytes: Vec<u8>) -> Result<InstanceTicket, InstanceError> {
        self.dispatcher.submit(bytes)
    }

    /// Owner-affine admission for stateful requests.
    /// # Errors
    /// Rejects closed/out-of-range owners, oversized requests or a full queue.
    pub fn submit_on(&self, owner: usize, bytes: Vec<u8>) -> Result<InstanceTicket, InstanceError> {
        self.dispatcher.submit_on(owner, bytes)
    }

    /// Clone admission for concurrent Rust callers; native VM owners remain fixed.
    #[must_use]
    pub fn dispatcher(&self) -> InstanceDispatcher {
        self.dispatcher.clone()
    }

    fn begin_joining(&mut self) {
        if self.joins.is_some() || self.threads.iter().all(Option::is_none) {
            return;
        }
        // is_finished() may become true before native TLS destructors return.
        // Only a real join proves owner retirement. One temporary coordinator
        // performs those blocking joins without blocking the caller's deadline.
        let (handoff, workers) = mpsc::channel::<Vec<(usize, OwnerThread)>>();
        let (joined, reports) = mpsc::channel();
        let coordinator = thread::Builder::new()
            .name("gerbil-owner-join".into())
            .spawn(move || Self::join_owners(&workers, &joined));
        if coordinator.is_err() {
            // No ownership moved on spawn failure; a later shutdown can retry.
            return;
        }
        let workers = self
            .threads
            .iter_mut()
            .enumerate()
            .filter_map(|(index, worker)| worker.take().map(|worker| (index, worker)))
            .collect();
        if let Err(mpsc::SendError(workers)) = handoff.send(workers) {
            self.restore_threads(workers);
            return;
        }
        self.joins = Some(reports);
        // Dropping this handle detaches only the coordinator. It runs no Scheme
        // work; owner completion is admitted only through actual join reports.
    }

    fn join_owners(
        workers: &mpsc::Receiver<Vec<(usize, OwnerThread)>>,
        reports: &mpsc::Sender<bool>,
    ) {
        let Ok(workers) = workers.recv() else { return };
        for (_, worker) in workers {
            let mut panicked = worker.worker.join().is_err();
            worker.mapping.joined(panicked);
            if matches!(
                worker.mapping.snapshot().state,
                ImageMappingState::Quarantined
                    | ImageMappingState::Unloading
                    | ImageMappingState::ReleaseFailed(_)
            ) {
                // Losing the host wrapper must not free data reachable by an
                // owner whose native retirement was not verified.
                std::mem::forget(worker.resources);
            } else {
                // Drop arbitrary host data outside every lifecycle/state lock,
                // and before publishing completion to the shutdown observer.
                for resource in worker.resources {
                    panicked |=
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(resource)))
                            .is_err();
                }
                if panicked {
                    worker.mapping.quarantine();
                }
            }
            let _ = reports.send(panicked);
        }
    }

    fn restore_threads(&mut self, workers: Vec<(usize, OwnerThread)>) {
        for (index, worker) in workers {
            self.threads[index] = Some(worker);
        }
    }

    fn collect_finished_owners(&mut self) -> usize {
        self.stopped.extend(self.done.try_iter());
        self.begin_joining();
        if let Some(reports) = &self.joins {
            for panicked in reports.try_iter() {
                self.panicked += usize::from(panicked);
                self.joined += 1;
            }
        }
        self.threads.len() - self.joined
    }

    /// Close admission and drain accepted work within one shared deadline.
    /// Pending workers are not cancelled, destroyed or reported as clean. Call
    /// again to observe eventual completion. Code release requires explicit
    /// provider authorization, clean native join and no outstanding code leases.
    #[must_use]
    pub fn shutdown(&mut self, timeout: Duration) -> ShutdownReceipt {
        self.dispatcher.close();
        let start = Instant::now();
        loop {
            let pending = self.collect_finished_owners();
            if pending == 0 || start.elapsed() >= timeout {
                self.stopped.extend(self.done.try_iter());
                return ShutdownReceipt {
                    owners: self.stopped.clone(),
                    joined: self.joined,
                    pending,
                    panicked: self.panicked,
                    mappings: self
                        .mappings
                        .iter()
                        .map(|mapping| mapping.snapshot())
                        .collect(),
                };
            }
            thread::park_timeout(
                Duration::from_millis(1).min(timeout.saturating_sub(start.elapsed())),
            );
        }
    }
}

impl Drop for InstancePool {
    fn drop(&mut self) {
        // Closing channels lets each owner finish accepted work and clean up.
        // Dropping JoinHandles only detaches; it never frees a running VM.
        self.dispatcher.close();
        self.begin_joining();
    }
}

#[cfg(test)]
#[path = "../../tests/unit/owner_resources.rs"]
mod resource_tests;
