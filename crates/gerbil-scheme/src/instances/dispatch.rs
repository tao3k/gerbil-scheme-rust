//! Cloneable Rust admission handle. Scheme state never crosses this boundary.
use super::{InstanceError, InstanceTicket, Request, pool::Work};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};

#[derive(Debug)]
pub(super) struct Admission {
    pub(super) senders: Mutex<Senders>,
    #[cfg(feature = "tokio-dispatch")]
    pub(super) ports: std::sync::OnceLock<Box<[Arc<super::owner_admission::Port>]>>,
    #[cfg(feature = "tokio-dispatch")]
    pub(super) accepting: Arc<AtomicBool>,
    #[cfg(feature = "tokio-dispatch")]
    pub(super) queue_capacity: usize,
    #[cfg(feature = "tokio-dispatch")]
    pub(super) capacity_changed:
        Arc<std::sync::OnceLock<crate::instances::capacity_event::CapacityEvents>>,
    #[cfg(feature = "tokio-dispatch")]
    pub(super) short_pool: std::sync::OnceLock<Arc<super::short_pool::Source>>,
    next: AtomicUsize,
    maximum_request: usize,
    maximum_reply: usize,
}

#[derive(Debug, Default)]
pub(super) struct Senders {
    pub(super) owners: Vec<OwnerSender>,
    #[cfg(feature = "tokio-dispatch")]
    pub(super) ever_admitted: bool,
}
#[derive(Debug)]
pub(super) struct OwnerSender {
    #[cfg(feature = "tokio-dispatch")]
    pub(super) port: Arc<super::owner_admission::Port>,
    pub(super) sender: Arc<mpsc::SyncSender<Request>>,
    pub(super) wake: Option<Arc<super::service_wake::Wake>>,
    #[cfg(feature = "tokio-dispatch")]
    pub(super) capacity: Option<Arc<super::capacity::Capacity>>,
    #[cfg(feature = "tokio-dispatch")]
    pub(super) capacity_limit: usize,
}
/// Cloneable, Send + Sync admission for concurrent Rust callers.
/// Closing its owning pool closes all clones before native teardown.
#[derive(Clone, Debug)]
pub struct InstanceDispatcher {
    pub(super) admission: Arc<Admission>,
}
impl InstanceDispatcher {
    pub(super) fn new(maximum_request: usize, maximum_reply: usize, queue_capacity: usize) -> Self {
        #[cfg(not(feature = "tokio-dispatch"))]
        let _ = queue_capacity;
        Self {
            admission: Arc::new(Admission {
                senders: Mutex::new(Senders::default()),
                #[cfg(feature = "tokio-dispatch")]
                ports: std::sync::OnceLock::new(),
                #[cfg(feature = "tokio-dispatch")]
                accepting: Arc::new(AtomicBool::new(true)),
                #[cfg(feature = "tokio-dispatch")]
                queue_capacity,
                #[cfg(feature = "tokio-dispatch")]
                capacity_changed: Arc::default(),
                #[cfg(feature = "tokio-dispatch")]
                short_pool: std::sync::OnceLock::new(),
                next: AtomicUsize::new(0),
                maximum_request,
                maximum_reply,
            }),
        }
    }
    pub(super) fn add_sender(
        &self,
        sender: mpsc::SyncSender<Request>,
        wake: Option<Arc<super::service_wake::Wake>>,
    ) {
        #[cfg(feature = "tokio-dispatch")]
        if let Some(wake) = &wake {
            let _ = wake
                .capacity_changed
                .set(Arc::clone(&self.admission.capacity_changed));
        }
        let sender = Arc::new(sender);
        #[cfg(feature = "tokio-dispatch")]
        let port = Arc::new(super::owner_admission::Port::new(&sender, wake.clone()));
        self.admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .owners
            .push(OwnerSender {
                sender,
                #[cfg(feature = "tokio-dispatch")]
                port,
                wake,
                #[cfg(feature = "tokio-dispatch")]
                capacity: None,
                #[cfg(feature = "tokio-dispatch")]
                capacity_limit: 0,
            });
    }
    pub(super) fn close(&self) {
        let mut senders = self
            .admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        #[cfg(feature = "tokio-dispatch")]
        let ports = super::owner_admission::ports(&senders);
        #[cfg(feature = "tokio-dispatch")]
        let mut states: Vec<_> = ports.iter().map(|port| port.lock()).collect();
        // Invalidate registered owner-local admission at the same fence as table
        // removal. No semaphore callback or foreign waker runs under this lock.
        #[cfg(feature = "tokio-dispatch")]
        self.admission.accepting.store(false, Ordering::Release);
        #[cfg(feature = "tokio-dispatch")]
        let retired: Vec<_> = states.iter_mut().filter_map(|state| state.take()).collect();
        let owners = std::mem::take(&mut senders.owners);
        #[cfg(feature = "tokio-dispatch")]
        drop(states);
        drop(senders);
        #[cfg(feature = "tokio-dispatch")]
        drop(retired);
        #[cfg(feature = "tokio-dispatch")]
        if let Some(source) = self.admission.short_pool.get() {
            source.close();
        }
        #[cfg(feature = "tokio-dispatch")]
        for owner in &owners {
            if let Some(capacity) = &owner.capacity {
                capacity.close();
            }
        }
        for owner in &owners {
            if let Some(wake) = &owner.wake {
                wake.close();
            }
        }
        drop(owners);
        #[cfg(feature = "tokio-dispatch")]
        if let Some(changed) = self.admission.capacity_changed.get() {
            changed.publish();
        }
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
        #[cfg(feature = "tokio-dispatch")]
        let mut senders = senders;
        let sender = senders.owners.get(owner).ok_or(InstanceError::Closed)?;
        #[cfg(feature = "tokio-dispatch")]
        let mut request = request;
        #[cfg(feature = "tokio-dispatch")]
        if let Err(error) = super::capacity::attach(sender, &mut request) {
            drop(senders);
            return Err(error);
        }
        let wake = sender.wake.clone();
        #[cfg(feature = "tokio-dispatch")]
        let (trace, short) = if let super::pool::Work::Service(job) = &request.work {
            (job.entry_trace().map(Arc::clone), job.short_service())
        } else {
            (None, false)
        };
        #[cfg(feature = "tokio-dispatch")]
        if let Some(trace) = &trace {
            trace.mark(super::SchemeServiceEntryPhase::Published);
        }
        let result = sender.sender.try_send(request);
        #[cfg(feature = "tokio-dispatch")]
        if result.is_ok() {
            senders.ever_admitted = true;
        }
        // Request destruction may return a semaphore credit and wake a caller.
        // Never invoke foreign wakers under the admission/close fence.
        drop(senders);
        #[cfg(feature = "tokio-dispatch")]
        super::service_entry::notify(wake, result.is_ok(), trace, short);
        #[cfg(not(feature = "tokio-dispatch"))]
        super::service_wake::notify_enqueued(wake, result.is_ok());
        result.map_err(|error| match error {
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
        #[cfg(feature = "tokio-dispatch")]
        let mut senders = senders;
        if senders.owners.is_empty() {
            return Err(InstanceError::Closed);
        }
        let owner = owner.unwrap_or_else(|| {
            self.admission.next.fetch_add(1, Ordering::Relaxed) % senders.owners.len()
        });
        let sender = senders.owners.get(owner).ok_or(InstanceError::Closed)?;
        let (response, result) = mpsc::channel();
        let started = Arc::new(AtomicBool::new(false));
        let request = Request {
            work: Work::Bytes { bytes, response },
            started: Arc::clone(&started),
            #[cfg(feature = "tokio-dispatch")]
            capacity: None,
            #[cfg(feature = "tokio-dispatch")]
            work_units: 1,
        };
        #[cfg(feature = "tokio-dispatch")]
        let mut request = request;
        #[cfg(feature = "tokio-dispatch")]
        if let Err(error) = super::capacity::attach(sender, &mut request) {
            drop(senders);
            return Err(error);
        }
        let wake = sender.wake.clone();
        let sent = sender.sender.try_send(request);
        #[cfg(feature = "tokio-dispatch")]
        if sent.is_ok() {
            senders.ever_admitted = true;
        }
        drop(senders);
        super::service_wake::notify_enqueued(wake, sent.is_ok());
        sent.map_err(|error| match error {
            mpsc::TrySendError::Full(_) => InstanceError::QueueFull,
            mpsc::TrySendError::Disconnected(_) => InstanceError::Closed,
        })?;
        Ok(InstanceTicket {
            response: result,
            started,
        })
    }
}

#[cfg(all(test, feature = "tokio-dispatch"))]
#[path = "../../tests/unit/producer_admission.rs"]
mod tests;
