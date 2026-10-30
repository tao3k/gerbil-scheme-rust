//! Owner-local service publication; registry controls acquire gates in owner order.
use super::{InstanceDispatcher, InstanceError, Request, SchemeServiceLane, capacity::Capacity};
use std::sync::{Arc, Mutex, MutexGuard, Weak, atomic::Ordering, mpsc};

#[derive(Debug)]
pub(super) struct Publication {
    sender: Weak<mpsc::SyncSender<Request>>,
    wake: Option<Arc<super::service_wake::Wake>>,
    pub(super) capacity: Option<Arc<Capacity>>,
    pub(super) published: bool,
}
/// Cached ports retain no sender. Close empties metadata before native teardown.
#[derive(Debug)]
pub(super) struct Port(Mutex<Option<Publication>>);
impl Port {
    pub(super) fn new(
        sender: &Arc<mpsc::SyncSender<Request>>,
        wake: Option<Arc<super::service_wake::Wake>>,
    ) -> Self {
        Self(Mutex::new(Some(Publication {
            sender: Arc::downgrade(sender),
            wake,
            capacity: None,
            published: false,
        })))
    }
    pub(super) fn lock(&self) -> MutexGuard<'_, Option<Publication>> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub(super) fn capacity(&self) -> Result<Arc<Capacity>, InstanceError> {
        let state = self.lock();
        let state = state.as_ref().ok_or(InstanceError::Closed)?;
        Ok(Arc::clone(state.capacity.as_ref().ok_or(
            InstanceError::InvalidConfig("shared owner capacity is not enabled"),
        )?))
    }
    pub(super) fn validate(
        &self,
        short: bool,
        lane: Option<SchemeServiceLane>,
    ) -> Result<(), InstanceError> {
        let state = self.lock();
        let state = state.as_ref().ok_or(InstanceError::Closed)?;
        validate(state.wake.as_ref(), short, lane)
    }
    pub(super) fn publish(
        &self,
        request: Request,
        short: bool,
        lane: Option<SchemeServiceLane>,
    ) -> Result<(), InstanceError> {
        if !crate::instance_backend::images_ready() {
            return Err(InstanceError::Closed);
        }
        let trace = if let super::pool::Work::Service(job) = &request.work {
            job.entry_trace().map(Arc::clone)
        } else {
            return Err(InstanceError::InvalidConfig(
                "owner port requires reserved service work",
            ));
        };
        let mut gate = self.lock();
        let state = gate.as_mut().ok_or(InstanceError::Closed)?;
        validate(state.wake.as_ref(), short, lane)?;
        match (&state.capacity, &request.capacity) {
            (Some(current), Some(permit)) if Arc::ptr_eq(current, &permit.capacity) => (),
            _ => return Err(InstanceError::Closed),
        }
        let sender = state.sender.upgrade().ok_or(InstanceError::Closed)?;
        let wake = state.wake.clone();
        if let Some(trace) = &trace {
            trace.mark(super::SchemeServiceEntryPhase::Published);
        }
        let result = sender.try_send(request);
        state.published |= result.is_ok();
        // No request destruction, semaphore return or foreign waker under a gate.
        // Close cannot remove the registry's strong sender while this gate is
        // held. This decrement cannot be the last Arc or disconnect the queue.
        // No transient strong sender escapes the gate to delay shutdown.
        drop(sender);
        drop(gate);
        super::service_entry::notify(wake, result.is_ok(), trace, short);
        result.map_err(|error| match error {
            mpsc::TrySendError::Full(_) => InstanceError::QueueFull,
            mpsc::TrySendError::Disconnected(_) => InstanceError::Closed,
        })
    }
}
pub(super) fn validate(
    wake: Option<&Arc<super::service_wake::Wake>>,
    short: bool,
    lane: Option<SchemeServiceLane>,
) -> Result<(), InstanceError> {
    let wake = wake.ok_or(InstanceError::InvalidConfig("Scheme service not enabled"))?;
    if short && wake.service_version.load(Ordering::Acquire) < 2 {
        return Err(InstanceError::InvalidConfig(
            "Scheme service lacks short scheduling capability",
        ));
    }
    if wake.closed.load(Ordering::Acquire) {
        return Err(InstanceError::Closed);
    }
    if lane.is_some_and(|lane| lane.0 >= wake.lanes.load(Ordering::Acquire)) {
        return Err(InstanceError::InvalidConfig(
            "Scheme service lane out of range",
        ));
    }
    Ok(())
}
pub(super) fn ports(senders: &super::dispatch::Senders) -> Vec<Arc<Port>> {
    senders
        .owners
        .iter()
        .map(|owner| Arc::clone(&owner.port))
        .collect()
}
impl InstanceDispatcher {
    /// Publish immutable owner indexes only after successful pool startup.
    pub(super) fn seal_owner_ports(&self) {
        let senders = self
            .admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.admission
            .ports
            .set(ports(&senders).into_boxed_slice())
            .expect("owner publication indexes are sealed once");
    }
    pub(super) fn owner_port(&self, owner: usize) -> Option<Result<&Arc<Port>, InstanceError>> {
        self.admission
            .ports
            .get()
            .map(|ports| ports.get(owner).ok_or(InstanceError::Closed))
    }
}
