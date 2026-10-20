//! Fixed native owner construction, queue draining and bounded joins.
use super::{
    InstanceConfig, InstanceDispatcher, InstanceError, InstanceReply, InstanceTicket,
    IsolatedProgram, OwnerReceipt, OwnerShutdown, ShutdownReceipt, image::Image,
};
use std::os::unix::fs::DirBuilderExt;
use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

pub(super) static LIFECYCLE: Mutex<()> = Mutex::new(());
static GENERATION: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
pub(super) struct Request {
    pub(super) started: Arc<AtomicBool>,
    pub(super) work: Work,
}
pub(super) enum Work {
    Bytes {
        bytes: Vec<u8>,
        response: mpsc::Sender<super::types::Completed<InstanceReply>>,
    },
    Native(Box<dyn FnOnce(&Image) + Send>),
}
impl std::fmt::Debug for Work {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Bytes { .. } => "ByteRequest",
            Self::Native(_) => "NativeTypeJob",
        })
    }
}

/// Explicit image backend. No fallback to the process-global runtime.
#[derive(Debug)]
pub struct InstancePool {
    dispatcher: InstanceDispatcher,
    threads: Vec<Option<thread::JoinHandle<()>>>,
    done: mpsc::Receiver<OwnerShutdown>,
    stopped: Vec<OwnerShutdown>,
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
        let directory = std::env::temp_dir().join(format!(
            "gerbil-owners-{}-{}",
            std::process::id(),
            GENERATION.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .map_err(|e| InstanceError::Loader(e.to_string()))?;
        let (finished, done) = mpsc::channel();
        let mut pool = Self {
            dispatcher: InstanceDispatcher::new(
                config.maximum_request_bytes,
                config.maximum_reply_bytes,
            ),
            threads: Vec::new(),
            done,
            stopped: Vec::new(),
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
        }
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
        let path = self.image_directory.join(format!("owner-{owner}.dylib"));
        fs::copy(&program.path, &path).map_err(|e| {
            let _ = fs::remove_file(&path);
            InstanceError::Loader(e.to_string())
        })?;
        let (sender, requests) = mpsc::sync_channel::<Request>(config.queue_capacity);
        let (ready, initialized) = mpsc::channel();
        let program = program.clone();
        let finished = finished.clone();
        let failed_copy = path.clone();
        let worker = thread::Builder::new()
            .name(format!("gerbil-owner-{owner}"))
            .spawn(move || {
                let loaded = {
                    let _guard = LIFECYCLE
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    Image::load(&path, &program, owner)
                };
                let image = match loaded {
                    Ok(image) => image,
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                let _ = ready.send(Ok(image.receipt.clone()));
                for request in requests {
                    request.started.store(true, Ordering::Release);
                    match request.work {
                        Work::Native(job) => job(&image),
                        Work::Bytes { bytes, response } => {
                            let value = if crate::instance_backend::images_ready() {
                                image
                                    .invoke(&bytes, config.maximum_reply_bytes)
                                    .map(|bytes| InstanceReply { owner, bytes })
                            } else {
                                Err(InstanceError::Closed)
                            };
                            let _ = response.send((value, Instant::now()));
                        }
                    }
                }
                let receipt = {
                    let _guard = LIFECYCLE
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    image.finish()
                };
                let _ = finished.send(receipt);
            })
            .map_err(|e| {
                let _ = fs::remove_file(&failed_copy);
                InstanceError::Loader(e.to_string())
            })?;
        self.dispatcher.add_sender(sender);
        self.threads.push(Some(worker));
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

    fn collect_finished_owners(&mut self) -> usize {
        self.stopped.extend(self.done.try_iter());
        for worker in &mut self.threads {
            if !worker.as_ref().is_some_and(thread::JoinHandle::is_finished) {
                continue;
            }
            self.panicked += usize::from(worker.take().expect("finished owner").join().is_err());
            self.joined += 1;
        }
        self.threads
            .iter()
            .filter(|worker| worker.is_some())
            .count()
    }

    /// Close admission and drain accepted work within one shared deadline.
    /// Pending workers are not cancelled, destroyed or reported as clean. Call
    /// again to observe eventual completion. Image code is never unloaded.
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
    }
}
