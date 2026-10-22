//! Existing producer pump supplies typed jobs; only Scheme schedules their execution.
use super::{
    InstanceDispatcher, InstanceError,
    batch_stream_runner::{Producer, ProducerStep, Registration},
    capacity::Capacity,
    dispatch::Admission,
    image::Image,
    pool::Work,
    service::{Job, JobId},
    service_stream_acquire::{Acquisition, SchemeServiceSupplyAdmission},
};
use std::{
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
    time::Instant,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
#[derive(Debug)]
pub(super) struct Signal(Waker);
impl Signal {
    pub(super) fn wake(&self) {
        self.0.wake_by_ref();
    }
}
pub(super) struct ServiceProducer {
    admission: Weak<Admission>,
    accepting: Arc<AtomicBool>,
    wake: Option<Arc<super::service_wake::Wake>>,
    owner: usize,
    capacity: Arc<Capacity>,
    _registration: OwnedSemaphorePermit,
    requests: super::service_stream_source::Source,
    active: Arc<Semaphore>,
    maximum: usize,
    acquiring: Acquisition,
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
    pub(super) signal: Arc<Signal>,
}
impl ServiceProducer {
    pub(super) fn new(
        dispatch: &InstanceDispatcher,
        registration: Registration,
        maximum: usize,
        requests: super::service_stream_source::Source,
        cancelled: Arc<AtomicBool>,
        mode: SchemeServiceSupplyAdmission,
    ) -> Self {
        let active = Arc::new(Semaphore::new(maximum));
        if let super::service_stream_source::Source::Pool(source) = &requests {
            if let Some(fair) = source.fair.get() {
                fair.register(
                    registration.owner,
                    registration
                        .wake
                        .as_ref()
                        .map_or_else(Weak::new, Arc::downgrade),
                    Arc::clone(&registration.capacity),
                    Arc::downgrade(&active),
                    registration.mailbox.clone(),
                );
            }
        }
        Self {
            admission: Arc::downgrade(&dispatch.admission),
            accepting: Arc::clone(&dispatch.admission.accepting),
            wake: registration.wake,
            owner: registration.owner,
            capacity: registration.capacity,
            _registration: registration.permit,
            requests,
            active,
            maximum,
            acquiring: Acquisition::new(mode),
            deadline: registration.deadline,
            cancelled,
            signal: Arc::new(Signal(registration.waker)),
        }
    }
    fn open(&self) -> bool {
        if !crate::instance_backend::images_ready() {
            return false;
        }
        if self.acquiring.owner_local() {
            // The held registration permit forbids capacity replacement. Close
            // invalidates accepting under the same table fence; wake identity is
            // immutable for this registration. Retain the final lifecycle check
            // without upgrading the shared Admission or locking its whole table.
            return self.accepting.load(Ordering::Acquire)
                && self.wake.as_ref().is_some_and(|wake| {
                    !wake.closed.load(Ordering::Acquire) && !wake.failed.load(Ordering::Acquire)
                });
        }
        let Some(admission) = self.admission.upgrade() else {
            return false;
        };
        let senders = admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        senders.owners.get(self.owner).is_some_and(|owner| {
            owner
                .capacity
                .as_ref()
                .is_some_and(|c| Arc::ptr_eq(c, &self.capacity))
                && owner.wake.as_ref().is_some_and(|wake| {
                    !wake.closed.load(Ordering::Acquire) && !wake.failed.load(Ordering::Acquire)
                })
        })
    }
}
impl Producer for ServiceProducer {
    fn retirement_refill_budget(&self) -> usize {
        if self.acquiring.retirement_refill() {
            self.maximum
        } else {
            0
        }
    }
    fn deadline(&self) -> Option<Instant> {
        (!self.requests.is_empty()).then_some(self.deadline)
    }
    fn reject(&mut self, error: InstanceError) {
        self.acquiring.clear(); // return both partial native and active credits first
        self.requests.reject(&error);
    }
    fn step(&mut self, _: &Image) -> ProducerStep {
        if !self.open() {
            self.reject(InstanceError::Closed);
        } else if self.cancelled.load(Ordering::Acquire) {
            self.reject(InstanceError::Cancelled);
        } else if Instant::now() >= self.deadline {
            self.reject(InstanceError::Timeout);
        }
        let Some(units) = self.requests.units() else {
            self.acquiring.clear();
            return if self.active.available_permits() == self.maximum {
                ProducerStep::Finished
            } else {
                ProducerStep::Waiting
            };
        };
        if !self.requests.ready() {
            return ProducerStep::Waiting;
        }
        // One actual remote admission per producer turn; never sacrifice a local
        // native turn to an unacknowledged wake. Partial local waits retain FIFO.
        if self.requests.assist(
            self.owner,
            units,
            &self.accepting,
            &self.cancelled,
            self.deadline,
        ) && self.requests.units() != Some(units)
        {
            if self.acquiring.waiting() {
                self.acquiring.clear();
            }
            return ProducerStep::Progress;
        }
        let mut context = Context::from_waker(&self.signal.0);
        let (active, capacity) =
            match self
                .acquiring
                .poll(&self.active, &self.capacity, units, &mut context)
            {
                Poll::Pending => return ProducerStep::Waiting,
                Poll::Ready(Err(error)) => {
                    self.reject(error);
                    return ProducerStep::Progress;
                }
                Poll::Ready(Ok(pair)) => pair,
            };
        // The same admission/close fence used by producers commits exactly one
        // entry. A racing close after this check drains it as accepted Scheme work.
        let error = if !self.open() {
            Some(InstanceError::Closed)
        } else if self.cancelled.load(Ordering::Acquire) {
            Some(InstanceError::Cancelled)
        } else if Instant::now() >= self.deadline {
            Some(InstanceError::Timeout)
        } else {
            None
        };
        if let Some(error) = error {
            drop(capacity);
            drop(active);
            self.reject(error);
            return ProducerStep::Progress;
        }
        let Some(mut request) = self.requests.take(units) else {
            // Another owner advanced the FIFO. Return speculative credits;
            // never consume a differently priced job.
            drop(capacity);
            drop(active);
            return ProducerStep::Progress;
        };
        let Work::Service(job) = request.work else {
            unreachable!("typed staged service")
        };
        if self.acquiring.owner_local() {
            // Native credits already span projection and every failure path. Retain
            // the stream credit there instead of allocating a second boxed job.
            request.work = Work::Service(job);
            request.capacity = Some(capacity.retain_active(active));
        } else {
            request.work = Work::Service(Box::new(StreamJob {
                job: Some(job),
                _active: active,
            }));
            request.capacity = Some(capacity);
        }
        ProducerStep::Service(request)
    }
}
struct StreamJob {
    job: Option<Box<dyn Job>>,
    _active: OwnedSemaphorePermit,
}
impl Job for StreamJob {
    fn entry_trace(&self) -> Option<&Arc<super::SchemeServiceEntryTrace>> {
        self.job.as_ref().unwrap().entry_trace()
    }
    fn short_service(&self) -> bool {
        self.job.as_ref().unwrap().short_service()
    }
    fn may_have_entered(&self) -> bool {
        self.job.as_ref().unwrap().may_have_entered()
    }
    fn start(
        &mut self,
        image: &Image,
        input: usize,
        output: usize,
        progress: bool,
    ) -> Result<JobId, InstanceError> {
        self.job
            .as_mut()
            .unwrap()
            .start(image, input, output, progress)
    }
    fn complete(mut self: Box<Self>, image: &Image, id: JobId, input: usize, output: usize) {
        self.job.take().unwrap().complete(image, id, input, output);
    }
    fn fail(mut self: Box<Self>, error: InstanceError) {
        self.job.take().unwrap().fail(error);
    }
}
