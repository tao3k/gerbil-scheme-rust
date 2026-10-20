//! Cloneable Rust admission handle. Scheme state never crosses this boundary.
use super::{InstanceError, InstanceTicket, Request, pool::Work};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};

#[derive(Debug)]
struct Admission {
    senders: Mutex<Vec<mpsc::SyncSender<Request>>>,
    next: AtomicUsize,
    maximum_request: usize,
    maximum_reply: usize,
}

/// Cloneable, Send + Sync admission for concurrent Rust callers.
/// Closing its owning pool closes all clones before native teardown.
#[derive(Clone, Debug)]
pub struct InstanceDispatcher {
    admission: Arc<Admission>,
}
impl InstanceDispatcher {
    pub(super) fn new(maximum_request: usize, maximum_reply: usize) -> Self {
        Self {
            admission: Arc::new(Admission {
                senders: Mutex::new(Vec::new()),
                next: AtomicUsize::new(0),
                maximum_request,
                maximum_reply,
            }),
        }
    }
    pub(super) fn add_sender(&self, sender: mpsc::SyncSender<Request>) {
        self.admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(sender);
    }
    pub(super) fn close(&self) {
        self.admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }
    /// Round-robin admission of an owned, stateless request.
    /// # Errors
    /// Returns closed, queue-full, backend-failure or request-bound rejection.
    pub fn submit(&self, bytes: Vec<u8>) -> Result<InstanceTicket, InstanceError> {
        self.admit(None, bytes)
    }
    /// Pin stateful work to the same native owner across foreign returns.
    /// # Errors
    /// Rejects closed/out-of-range owners, oversized requests or a full queue.
    pub fn submit_on(&self, owner: usize, bytes: Vec<u8>) -> Result<InstanceTicket, InstanceError> {
        self.admit(Some(owner), bytes)
    }
    pub(super) fn maximum_input(&self) -> usize {
        self.admission.maximum_request
    }
    pub(super) fn maximum_output(&self) -> usize {
        self.admission.maximum_reply
    }
    pub(super) fn enqueue(&self, owner: usize, request: Request) -> Result<(), InstanceError> {
        if !crate::instance_backend::images_ready() {
            return Err(InstanceError::Closed);
        }
        let senders = self
            .admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        senders
            .get(owner)
            .ok_or(InstanceError::Closed)?
            .try_send(request)
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => InstanceError::QueueFull,
                mpsc::TrySendError::Disconnected(_) => InstanceError::Closed,
            })
    }
    fn admit(&self, owner: Option<usize>, bytes: Vec<u8>) -> Result<InstanceTicket, InstanceError> {
        if !crate::instance_backend::images_ready() {
            return Err(InstanceError::Closed);
        }
        if bytes.len() > self.admission.maximum_request {
            return Err(InstanceError::InvalidConfig(
                "request exceeds configured bound",
            ));
        }
        // Close and admission share this short Rust-only lock. It never covers
        // Scheme execution or a blocking channel send.
        let senders = self
            .admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if senders.is_empty() {
            return Err(InstanceError::Closed);
        }
        let owner = owner
            .unwrap_or_else(|| self.admission.next.fetch_add(1, Ordering::Relaxed) % senders.len());
        let sender = senders.get(owner).ok_or(InstanceError::Closed)?;
        let (response, result) = mpsc::channel();
        let started = Arc::new(AtomicBool::new(false));
        sender
            .try_send(Request {
                work: Work::Bytes { bytes, response },
                started: Arc::clone(&started),
            })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => InstanceError::QueueFull,
                mpsc::TrySendError::Disconnected(_) => InstanceError::Closed,
            })?;
        Ok(InstanceTicket {
            response: result,
            started,
        })
    }
}
