//! Existing producer pump supplies typed jobs; only Scheme schedules their execution.
use super::{
    InstanceDispatcher, InstanceError,
    batch_stream_runner::{Producer, ProducerStep, Registration},
    capacity::{Capacity, CapacityPermit, acquire_pair},
    dispatch::Admission,
    image::Image,
    pool::Work,
    service::{Job, JobId},
};
use std::{
    future::Future,
    pin::Pin,
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
type Acquiring = Pin<
    Box<dyn Future<Output = Result<(OwnedSemaphorePermit, CapacityPermit), InstanceError>> + Send>,
>;
pub(super) struct ServiceProducer {
    admission: Weak<Admission>,
    owner: usize,
    capacity: Arc<Capacity>,
    _registration: OwnedSemaphorePermit,
    requests: super::service_stream_source::Source,
    active: Arc<Semaphore>,
    maximum: usize,
    acquiring: Option<Acquiring>,
    acquiring_units: usize,
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
    ) -> Self {
        Self {
            admission: Arc::downgrade(&dispatch.admission),
            owner: registration.owner,
            capacity: registration.capacity,
            _registration: registration.permit,
            requests,
            active: Arc::new(Semaphore::new(maximum)),
            maximum,
            acquiring: None,
            acquiring_units: 0,
            deadline: registration.deadline,
            cancelled,
            signal: Arc::new(Signal(registration.waker)),
        }
    }
    fn open(&self) -> bool {
        if !crate::instance_backend::images_ready() {
            return false;
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
    fn deadline(&self) -> Option<Instant> {
        (!self.requests.is_empty()).then_some(self.deadline)
    }
    fn reject(&mut self, error: InstanceError) {
        self.acquiring = None; // return both partial native and active credits first
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
            self.acquiring = None;
            return if self.active.available_permits() == self.maximum {
                ProducerStep::Finished
            } else {
                ProducerStep::Waiting
            };
        };
        if !self.requests.ready() {
            return ProducerStep::Waiting;
        }
        if self.acquiring.is_some() && self.acquiring_units != units {
            self.acquiring = None;
        }
        if self.acquiring.is_none() {
            let active = Arc::clone(&self.active);
            let capacity = Arc::clone(&self.capacity);
            self.acquiring_units = units;
            self.acquiring = Some(Box::pin(async move {
                let active = active
                    .acquire_owned()
                    .await
                    .map_err(|_| InstanceError::Closed)?;
                let capacity = acquire_pair(capacity, units).await?;
                Ok((active, capacity))
            }));
        }
        let mut context = Context::from_waker(&self.signal.0);
        let (active, capacity) = match self.acquiring.as_mut().unwrap().as_mut().poll(&mut context)
        {
            Poll::Pending => return ProducerStep::Waiting,
            Poll::Ready(Err(error)) => {
                self.reject(error);
                return ProducerStep::Progress;
            }
            Poll::Ready(Ok(pair)) => pair,
        };
        self.acquiring = None;
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
        let Some(mut request) = self.requests.take(self.acquiring_units) else {
            // Another owner advanced the shared FIFO while credits were pending.
            // Return speculative credits; never consume a differently priced job.
            drop(capacity);
            drop(active);
            return ProducerStep::Progress;
        };
        let Work::Service(job) = request.work else {
            unreachable!("typed staged service")
        };
        request.work = Work::Service(Box::new(StreamJob {
            job: Some(job),
            _active: active,
        }));
        request.capacity = Some(capacity);
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
    ) -> Result<JobId, InstanceError> {
        self.job.as_mut().unwrap().start(image, input, output)
    }
    fn complete(mut self: Box<Self>, image: &Image, id: JobId, input: usize, output: usize) {
        self.job.take().unwrap().complete(image, id, input, output);
    }
    fn fail(mut self: Box<Self>, error: InstanceError) {
        self.job.take().unwrap().fail(error);
    }
}
