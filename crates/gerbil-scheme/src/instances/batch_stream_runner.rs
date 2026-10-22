//! Bounded producer state lives on the native owner; no Tokio runtime enters Scheme.
use super::{
    InstanceDispatcher, InstanceError,
    batch_stream::{BatchStreamRow, OutputBytes, RunBatch},
    capacity::{Capacity, CapacityPermit, acquire_pair},
    dispatch::Admission,
    image::Image,
};
use crate::NumericElement;
use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{Arc, Weak},
    task::{Context, Poll, Waker},
    time::Instant,
};
use tokio::sync::{OwnedSemaphorePermit, oneshot};
pub(super) enum ProducerStep {
    Progress,
    Waiting,
    Finished,
    Service(super::Request),
}
pub(super) trait Producer: Send {
    fn step(&mut self, image: &Image) -> ProducerStep;
    fn deadline(&self) -> Option<Instant>;
    fn reject(&mut self, error: InstanceError);
}
type Acquiring = Pin<Box<dyn Future<Output = Result<CapacityPermit, InstanceError>> + Send>>;
type Reply<R> = oneshot::Sender<super::types::Completed<Vec<BatchStreamRow<R>>>>;
pub(super) struct Registration {
    pub capacity: Arc<Capacity>,
    pub permit: OwnedSemaphorePermit,
    pub owner: usize,
    pub deadline: Instant,
    pub waker: Waker,
    pub wake: Option<Arc<super::service_wake::Wake>>,
}
pub(super) struct BatchOperation<T, R> {
    pub run: RunBatch<T, R>,
    pub output_bytes: OutputBytes<R>,
    pub quantum: usize,
}
pub(super) struct BatchProducer<T, R> {
    admission: Weak<Admission>,
    owner: usize,
    capacity: Arc<Capacity>,
    _registration: OwnedSemaphorePermit,
    batches: VecDeque<Vec<Vec<T>>>,
    rows: Vec<BatchStreamRow<R>>,
    acquiring: Option<Acquiring>,
    dispatched: Instant,
    offered: Instant,
    waker: Waker,
    deadline: Instant,
    quantum: usize,
    maximum_input: usize,
    maximum_output: usize,
    projected: usize,
    run: RunBatch<T, R>,
    output_bytes: OutputBytes<R>,
    reply: Option<Reply<R>>,
}
impl<T, R> BatchProducer<T, R> {
    pub(super) fn new(
        dispatch: &InstanceDispatcher,
        registration: Registration,
        batches: Vec<Vec<Vec<T>>>,
        operation: &BatchOperation<T, R>,
        reply: Reply<R>,
    ) -> Self {
        let Registration {
            capacity,
            permit,
            owner,
            deadline,
            waker,
            wake: _,
        } = registration;
        let offered = Instant::now();
        Self {
            admission: Arc::downgrade(&dispatch.admission),
            owner,
            capacity,
            _registration: permit,
            rows: Vec::with_capacity(batches.len()),
            batches: batches.into(),
            acquiring: None,
            dispatched: offered,
            offered,
            waker,
            deadline,
            quantum: operation.quantum,
            maximum_input: dispatch.maximum_input(),
            maximum_output: dispatch.maximum_output(),
            projected: 0,
            run: operation.run,
            output_bytes: operation.output_bytes,
            reply: Some(reply),
        }
    }
    fn finish(&mut self) {
        if let Some(reply) = self.reply.take() {
            let _ = reply.send((Ok(std::mem::take(&mut self.rows)), Instant::now()));
        }
    }
    fn enter(&self) -> bool {
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
        senders
            .owners
            .get(self.owner)
            .and_then(|owner| owner.capacity.as_ref())
            .is_some_and(|c| Arc::ptr_eq(c, &self.capacity))
    }
    fn bounded(&mut self, value: Result<R, InstanceError>) -> Result<R, InstanceError> {
        let Ok(value) = value else {
            return value;
        };
        let total = (self.output_bytes)(&value).and_then(|n| self.projected.checked_add(n));
        match total {
            Some(total) if total <= self.maximum_output => {
                self.projected = total;
                Ok(value)
            }
            _ => Err(InstanceError::InvalidConfig(
                "producer aggregate output bound",
            )),
        }
    }
}
impl<T, R> Producer for BatchProducer<T, R>
where
    T: NumericElement + Send + 'static,
    R: Send + 'static,
{
    fn deadline(&self) -> Option<Instant> {
        Some(self.deadline)
    }
    fn reject(&mut self, error: InstanceError) {
        self.acquiring = None;
        for _ in self.batches.drain(..) {
            self.rows.push(BatchStreamRow {
                value: Err(error.clone()),
                completed: None,
                offered: self.offered,
            });
        }
        self.finish();
    }
    fn step(&mut self, image: &Image) -> ProducerStep {
        if self.batches.is_empty() {
            self.finish();
            return ProducerStep::Finished;
        }
        if Instant::now() >= self.deadline {
            self.reject(InstanceError::Timeout);
            return ProducerStep::Finished;
        }
        if self.acquiring.is_none() {
            self.dispatched = Instant::now();
            self.acquiring = Some(Box::pin(acquire_pair(
                Arc::clone(&self.capacity),
                self.batches.front().unwrap().len(),
            )));
        }
        let mut context = Context::from_waker(&self.waker);
        let permit = match self.acquiring.as_mut().unwrap().as_mut().poll(&mut context) {
            Poll::Pending => return ProducerStep::Waiting,
            Poll::Ready(Err(error)) => {
                self.reject(error);
                return ProducerStep::Finished;
            }
            Poll::Ready(Ok(permit)) => permit,
        };
        self.acquiring = None;
        if Instant::now() >= self.deadline || !self.enter() {
            drop(permit);
            self.reject(if Instant::now() >= self.deadline {
                InstanceError::Timeout
            } else {
                InstanceError::Closed
            });
            return ProducerStep::Finished;
        }
        // The admission/close fence has accepted exactly this original native turn.
        let items = self.batches.pop_front().unwrap();
        let run = self.run;
        let quantum = self.quantum;
        let dispatched = self.dispatched;
        let mut completed = None;
        super::native_dispatch::run_native(
            image,
            self.maximum_input,
            self.maximum_output,
            |session| run(session, &items, quantum, dispatched),
            |result| {
                completed = Some(result);
                true
            },
        );
        let (value, at) = completed.unwrap();
        let value = self.bounded(value);
        self.rows.push(BatchStreamRow {
            value,
            completed: Some(at),
            offered: self.offered,
        });
        // Release before considering another turn; pending short callers share FIFO credits.
        drop(permit);
        if self.batches.is_empty() {
            self.finish();
            ProducerStep::Finished
        } else {
            ProducerStep::Progress
        }
    }
}
