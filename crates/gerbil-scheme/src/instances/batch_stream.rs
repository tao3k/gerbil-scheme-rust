//! Explicit independent batch cursors; each native turn still acquires both budgets.
use super::{
    AsyncInstanceTicket, BatchAdmission, InstanceDispatcher, InstanceError, NativeSession,
    ProfiledBatch, capacity::Capacity, pool::Work,
};
use crate::NumericElement;
use std::{
    sync::{Arc, atomic::AtomicBool, mpsc},
    time::Instant,
};

/// One independent batch outcome. Unadmitted turns have no native completion clock.
#[derive(Debug)]
pub struct BatchStreamRow<R> {
    /// Existing whole-batch native result or an admission failure.
    pub value: Result<R, InstanceError>,
    /// Some only after the shared native execution/balance path returns.
    pub completed: Option<Instant>,
    /// This cursor was offered to its fixed owner's inbox before turn admission.
    /// Shared by its rows; does not imply that any staged turn was accepted.
    pub offered: Instant,
}
/// Unprofiled owner-local numeric producer result.
pub type NumericBatchStreamTicket<T> = AsyncInstanceTicket<Vec<BatchStreamRow<Vec<Vec<T>>>>>;
/// Profiled owner-local numeric producer result.
pub type ProfiledBatchStreamTicket<T> = AsyncInstanceTicket<Vec<BatchStreamRow<ProfiledBatch<T>>>>;
pub(super) type RunBatch<T, R> =
    fn(&NativeSession<'_>, &[Vec<T>], usize, Instant) -> Result<R, InstanceError>;
pub(super) type OutputBytes<R> = fn(&R) -> Option<usize>;
fn bytes<T>(values: &[Vec<T>]) -> Option<usize> {
    values.iter().try_fold(0_usize, |n, v| {
        n.checked_add(std::mem::size_of_val(v.as_slice()))
    })
}
impl InstanceDispatcher {
    /// Stage a bounded sequence of explicitly independent native batches on one owner.
    /// There is one producer per owner. Each original batch separately reserves work
    /// and a request slot, executes unchanged Scheme, releases both and checks the
    /// inbox. No batch is split/fused; regular queued requests keep their FIFO order.
    /// Inputs/outputs are bounded in aggregate by existing transfer limits; staged
    /// turns cannot exceed native queue capacity. Individual Scheme errors do not
    /// cancel subsequent independent batches. Close/deadline rejects unadmitted turns.
    /// `admission.timeout` bounds registration and the start of every unadmitted
    /// turn from this API entry. An entered turn is never preempted. Abandoning the
    /// result does not cancel entered work; later turns run only before that deadline.
    /// Stateful/dependent calls must use the ordinary owner-pinned APIs instead.
    /// # Errors
    /// Rejects invalid bounds, disabled work budgets, registration timeout/close/full.
    pub async fn submit_numeric_batch_stream_backpressured<T>(
        &self,
        batches: Vec<Vec<Vec<T>>>,
        admission: BatchAdmission,
    ) -> Result<NumericBatchStreamTicket<T>, InstanceError>
    where
        T: NumericElement + Send + 'static,
    {
        self.batch_stream(
            batches,
            admission,
            |session, items, quantum, _| {
                super::batch::execute_native(session, items, quantum, |_| {})
            },
            |values| bytes(values),
        )
        .await
    }
    /// Same bounded independent-batch producer with the existing opt-in phase clocks.
    /// Staged Rust input is not accepted native work; per-turn dispatch starts at its
    /// first credit attempt. Scheme runs only on the fixed native owner.
    /// # Errors
    /// Same validation/registration errors as the unprofiled producer API.
    pub async fn submit_numeric_batch_stream_profiled_backpressured<T>(
        &self,
        batches: Vec<Vec<Vec<T>>>,
        admission: BatchAdmission,
    ) -> Result<ProfiledBatchStreamTicket<T>, InstanceError>
    where
        T: NumericElement + Send + 'static,
    {
        self.batch_stream(
            batches,
            admission,
            super::batch_trace::execute_profiled_native,
            |result| bytes(&result.values),
        )
        .await
    }
    async fn batch_stream<T, R>(
        &self,
        batches: Vec<Vec<Vec<T>>>,
        admission: BatchAdmission,
        run: RunBatch<T, R>,
        output_bytes: OutputBytes<R>,
    ) -> Result<AsyncInstanceTicket<Vec<BatchStreamRow<R>>>, InstanceError>
    where
        T: NumericElement + Send + 'static,
        R: Send + 'static,
    {
        if !crate::instance_backend::images_ready() {
            return Err(InstanceError::Closed);
        }
        let deadline = Instant::now()
            .checked_add(admission.timeout)
            .ok_or(InstanceError::InvalidConfig("producer deadline overflow"))?;
        if admission.timeout.is_zero() {
            return Err(InstanceError::Timeout);
        }
        if batches.is_empty() || batches.len() > self.admission.queue_capacity {
            return Err(InstanceError::InvalidConfig("producer turn count bound"));
        }
        let mut total = 0_usize;
        for items in &batches {
            super::batch::validate(items, admission.yield_every, self.maximum_input())?;
            total = total
                .checked_add(
                    super::batch::input_size(items)
                        .ok_or(InstanceError::InvalidConfig("producer input overflow"))?,
                )
                .ok_or(InstanceError::InvalidConfig("producer input overflow"))?;
        }
        if total > self.maximum_input() {
            return Err(InstanceError::InvalidConfig(
                "producer aggregate input bound",
            ));
        }
        let capacity = self.producer_owner(admission.owner)?;
        if capacity.work.is_none() {
            return Err(InstanceError::InvalidConfig(
                "producer requires explicit work budget",
            ));
        }
        for items in &batches {
            capacity.units(items.len())?;
        }
        let registration = tokio::time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            Arc::clone(&capacity.producer).acquire_owned(),
        )
        .await
        .map_err(|_| InstanceError::Timeout)?
        .map_err(|_| InstanceError::Closed)?;
        if Instant::now() >= deadline {
            return Err(InstanceError::Timeout);
        }
        let mut senders = self
            .admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let current = senders
            .owners
            .get(admission.owner)
            .ok_or(InstanceError::Closed)?;
        if !current
            .capacity
            .as_ref()
            .is_some_and(|c| Arc::ptr_eq(c, &capacity))
        {
            drop(senders);
            return Err(InstanceError::Closed);
        }
        let (reply, response) = tokio::sync::oneshot::channel();
        let producer = super::batch_stream_runner::BatchProducer::new(
            self,
            super::batch_stream_runner::Registration {
                capacity: Arc::clone(&capacity),
                permit: registration,
                admission,
                deadline,
                waker: super::owner_loop::waker(current.sender.clone()),
            },
            batches,
            run,
            output_bytes,
            reply,
        );
        let request = super::Request {
            started: Arc::new(AtomicBool::new(false)),
            work: Work::Producer(Box::new(producer)),
            capacity: None,
            work_units: 0,
        };
        let sent = current.sender.try_send(request);
        if sent.is_ok() {
            senders.ever_admitted = true;
        }
        drop(senders);
        sent.map_err(|error| match error {
            mpsc::TrySendError::Full(_) => InstanceError::QueueFull,
            mpsc::TrySendError::Disconnected(_) => InstanceError::Closed,
        })?;
        Ok(AsyncInstanceTicket { response })
    }
    fn producer_owner(&self, owner: usize) -> Result<Arc<Capacity>, InstanceError> {
        let senders = self
            .admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let current = senders.owners.get(owner).ok_or(InstanceError::Closed)?;
        let capacity = Arc::clone(current.capacity.as_ref().ok_or(
            InstanceError::InvalidConfig("shared owner capacity is not enabled"),
        )?);
        Ok(capacity)
    }
}
