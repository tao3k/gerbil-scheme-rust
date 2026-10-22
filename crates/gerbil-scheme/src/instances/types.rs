//! Owned host values and lifecycle receipts for native instance dispatch.
use std::{
    ffi::{CStr, CString},
    fmt,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

/// Failure at an explicit admission, execution or lifecycle boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstanceError {
    /// Invalid finite resource configuration.
    InvalidConfig(&'static str),
    /// Native image/file operation failed.
    Loader(String),
    /// Native ABI operation failed; Scheme exceptions return a rejected root.
    Native { operation: &'static str, code: i64 },
    /// Caught Scheme exception from the original owner-local native call.
    Scheme(String),
    /// This process already selected the ordinary linked runtime.
    BackendConflict,
    /// Startup failed; observed drain state is retained, including pending owners.
    Startup {
        cause: Box<InstanceError>,
        shutdown: ShutdownReceipt,
        image_directory: PathBuf,
    },
    /// Initialization phase is reported without inventing partial-setup rollback.
    Initialization {
        owner: usize,
        status: i32,
        phase: u32,
        diagnostic: Option<Box<[i64; 4]>>,
        cleanup: Option<OwnerShutdown>,
    },
    /// Owner queue has reached its configured bound; request was not admitted.
    QueueFull,
    /// An unadmitted staged service request was cancelled; entered Scheme is not preempted.
    Cancelled,
    /// Admission was closed or the selected owner is no longer running.
    Closed,
    /// Wait expired. Accepted Scheme work remains live; this is not cancellation.
    Timeout,
    /// A Rust scheduling/projection job panicked; native root guards unwound.
    TaskPanicked,
}
impl fmt::Display for InstanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for InstanceError {}

/// Build-owned image and application object-export descriptor.
#[derive(Clone, Debug)]
pub struct IsolatedProgram {
    pub(super) path: PathBuf,
    pub(super) entry: CString,
    pub(super) exports: Vec<CString>,
    pub(super) finalizer: Option<CString>,
    pub(super) service: Option<CString>,
    pub(super) unload_after_join: bool,
}
impl IsolatedProgram {
    /// Opt into a Scheme-owned asynchronous service and native wake-port loop.
    /// Ordinary entry/default pool behavior is unchanged without this descriptor.
    /// # Safety
    /// The export implements the rooted service command ABI documented in
    /// `scheme/async-service.ss` and the native service protocol document: open, submit, take, ready, wait, prepare-close and drain/close.
    /// It owns task graphs until take/close and closes every child/port before
    /// returning from close. Wait wakes on host input or Scheme completion.
    /// No native pointer escapes and no continuation crosses the foreign frame.
    #[must_use]
    pub unsafe fn with_scheme_service(mut self, export: &CStr) -> Self {
        self.service = Some(export.to_owned());
        self
    }

    /// Authorize balancing this image's loader reference after clean native join.
    /// # Safety
    /// All VM-entering callbacks must be quiesced before SDK cleanup. Every
    /// code-group reference that can outlive its owner (host callbacks, foreign-thread
    /// TLS, native module hooks) must hold an `ImageCodeLease` until revoked/retired.
    /// Unregistered references are forbidden by this opt-in contract, not silently
    /// disabled by the runtime. Native unload hooks must not re-enter a dead VM.
    #[must_use]
    pub unsafe fn with_unload_after_join(mut self) -> Self {
        self.unload_after_join = true;
        self
    }

    /// Quiesce Scheme services after queue drain, before root/VM retirement.
    /// # Safety
    /// The export obeys the rooted unary ABI and returns after joining its children.
    #[must_use]
    pub unsafe fn with_finalizer(mut self, name: &CStr) -> Self {
        self.finalizer = Some(name.to_owned());
        self
    }

    /// Add rooted unary exports, resolved before any native setup.
    /// # Safety
    /// Every export obeys the same root/exception/lifetime ABI as the main entry.
    #[must_use]
    pub unsafe fn with_exports(mut self, exports: &[&CStr]) -> Self {
        self.exports = exports.iter().map(|name| (*name).to_owned()).collect();
        self
    }

    /// Select an existing native image; no runtime starts here.
    ///
    /// # Safety
    /// The trusted image must implement the instance ABI in `ffi/instance.h`,
    /// hide its Gambit symbols and link SDK dependencies statically. `entry`
    /// if present, must borrow one input root, catch Scheme exceptions, return zero on
    /// rejection or transfer one fresh object root, and retain no Rust
    /// pointer. It must not finalize the VM, move roots, escape foreign frames,
    /// or enter an unqualified multi-processor configuration. VM-entering callbacks
    /// must quiesce before SDK cleanup. Image constructors
    /// must not initialize a VM or leave callbacks/TLS destructors that cannot
    /// retire during preflight dlclose. All native functions must return normally.
    ///
    /// # Errors
    /// Returns a loader error if the image path cannot be resolved.
    pub unsafe fn from_image(path: impl AsRef<Path>, entry: &CStr) -> Result<Self, InstanceError> {
        let path = path
            .as_ref()
            .canonicalize()
            .map_err(|e| InstanceError::Loader(e.to_string()))?;
        Ok(Self {
            path,
            entry: entry.to_owned(),
            exports: Vec::new(),
            finalizer: None,
            service: None,
            unload_after_join: false,
        })
    }
}

/// Finite admission and byte-transfer bounds.
#[derive(Clone, Copy, Debug)]
pub struct InstanceConfig {
    pub owners: usize,
    pub queue_capacity: usize,
    pub maximum_request_bytes: usize,
    pub maximum_reply_bytes: usize,
    pub startup_timeout: Duration,
}

/// Observed initialization on the fixed native owner. State is diagnostic only.
#[derive(Clone, Debug)]
pub struct OwnerReceipt {
    pub owner: usize,
    pub state: usize,
    pub processors: u32,
    pub setup_time: Duration,
}

/// Owned reply; no Scheme object or root token crosses the queue.
#[derive(Debug)]
pub struct InstanceReply {
    pub owner: usize,
    pub bytes: Vec<u8>,
}

pub(super) type Completed<R> = (Result<R, InstanceError>, Instant);

/// One accepted request. A timeout preserves the ticket for a later wait.
#[derive(Debug)]
pub struct InstanceTicket<R = InstanceReply> {
    pub(super) started: Arc<AtomicBool>,
    pub(super) response: mpsc::Receiver<Completed<R>>,
}
impl<R> InstanceTicket<R> {
    /// Observe that the Rust owner dequeued this task; this is not proof of Scheme entry.
    #[must_use]
    pub fn has_started(&self) -> bool {
        self.started.load(Ordering::Acquire)
    }
    /// Wait for this request without cancelling Scheme execution.
    /// # Errors
    /// Returns timeout, owner termination, or a native rejection/conversion error.
    pub fn wait(&self, timeout: Duration) -> Result<R, InstanceError> {
        self.wait_completed(timeout).map(|(value, _)| value)
    }
    /// Wait with the owner's completion timestamp, recorded after native root
    /// balancing and before reply publication. This excludes consumer wait order.
    /// # Errors
    /// Returns the same non-cancelling timeout and execution errors as `wait`.
    pub fn wait_completed(&self, timeout: Duration) -> Result<(R, Instant), InstanceError> {
        let (value, completed) =
            self.response
                .recv_timeout(timeout)
                .map_err(|error| match error {
                    mpsc::RecvTimeoutError::Timeout => InstanceError::Timeout,
                    mpsc::RecvTimeoutError::Disconnected => InstanceError::Closed,
                })?;
        value.map(|value| (value, completed))
    }
}

/// Terminal owner observation; unverified quiescence retains the native heap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerShutdown {
    pub owner: usize,
    pub roots_before_cleanup: i64,
    pub gc_collections: u64,
    /// False when failed quiescence/root balance quarantined the initialized VM.
    pub native_cleanup_attempted: bool,
    pub status: i32,
    pub cleanup_time: Duration,
    /// True only after the copied image path was removed or was already absent.
    pub image_file_removed: bool,
    /// File/directory removal error. Native cleanup status is reported separately.
    pub image_file_error: Option<String>,
    /// Requested native fdset scratch bytes retired by the image adapter.
    pub native_io_released_bytes: u64,
    /// Optional Scheme finalizer status: zero means its rooted call completed.
    pub finalizer_status: Option<i32>,
}

/// Bounded shutdown result. Pending owners remain live and retain their image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShutdownReceipt {
    pub owners: Vec<OwnerShutdown>,
    pub joined: usize,
    pub pending: usize,
    pub panicked: usize,
    pub mappings: Vec<super::ImageMappingReceipt>,
}

impl ShutdownReceipt {
    /// True only for observed cleanup, joined owners, balanced roots and removed copies.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.mappings.iter().all(|receipt| {
            matches!(
                receipt.state,
                super::ImageMappingState::Released
                    | super::ImageMappingState::RetainedByPolicy
                    | super::ImageMappingState::RetainedByLeases(_)
            )
        }) && self.pending == 0
            && self.panicked == 0
            && self.owners.len() == self.joined
            && self.owners.iter().all(|owner| {
                owner.native_cleanup_attempted
                    && owner.status == 0
                    && owner.roots_before_cleanup == 0
                    && owner.image_file_removed
                    && owner.image_file_error.is_none()
            })
    }
}
