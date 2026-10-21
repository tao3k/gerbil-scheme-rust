//! Tokio dispatch never enters a VM; dedicated native owners execute jobs.
use super::{InstanceDispatcher, InstanceError, NativeSession};
use crate::NumericElement;
use std::time::{Duration, Instant};
/// One asynchronous owner result. A timeout does not cancel accepted Scheme work.
#[derive(Debug)]
pub struct AsyncInstanceTicket<R> {
    response: tokio::sync::oneshot::Receiver<super::types::Completed<R>>,
}
impl<R> AsyncInstanceTicket<R> {
    /// Wait with a finite delivery budget, returning the native completion timestamp.
    /// # Errors
    /// Reports timeout, closed channel and the original native/task error.
    pub async fn wait_completed(self, timeout: Duration) -> Result<(R, Instant), InstanceError> {
        let (value, completed) = tokio::time::timeout(timeout, self.response)
            .await
            .map_err(|_| InstanceError::Timeout)?
            .map_err(|_| InstanceError::Closed)?;
        value.map(|value| (value, completed))
    }
}
impl InstanceDispatcher {
    /// Submit the same bounded Scheme Actor batch without blocking a waiter thread.
    /// Native construction, Scheme scheduling, projection and root retirement use
    /// the same owner-local implementation as `submit_numeric_batch_on`.
    /// # Errors
    /// Reports the same input, admission and native errors as the synchronous API.
    /// An expired wait does not cancel accepted Scheme work.
    pub fn submit_numeric_batch_async_on<T>(
        &self,
        owner: usize,
        items: Vec<Vec<T>>,
        yield_every: usize,
    ) -> Result<AsyncInstanceTicket<Vec<Vec<T>>>, InstanceError>
    where
        T: NumericElement + Send + 'static,
    {
        super::batch::validate(&items, yield_every, self.maximum_input())?;
        self.submit_native_async_on(owner, move |session| {
            super::batch::execute(session, &items, yield_every, |_| {})
        })
    }

    /// Enqueue native Scheme work and await delivery without a blocking waiter thread.
    /// # Errors
    /// Reports admission failures before returning a ticket.
    pub fn submit_native_async_on<R, F>(
        &self,
        owner: usize,
        job: F,
    ) -> Result<AsyncInstanceTicket<R>, InstanceError>
    where
        R: Send + 'static,
        F: for<'vm> FnOnce(&NativeSession<'vm>) -> Result<R, InstanceError> + Send + 'static,
    {
        let (send, response) = tokio::sync::oneshot::channel();
        self.native_request(owner, job, move |value| send.send(value).is_ok())?;
        Ok(AsyncInstanceTicket { response })
    }
}

impl InstanceDispatcher {
    /// Await shared owner capacity before enqueuing a native operation.
    /// FIFO reservations share capacity with all synchronous/asynchronous APIs.
    /// Observation timeout after acceptance never returns the native credit early.
    /// # Errors
    /// Reports disabled capacity, admission timeout, close or native queue rejection.
    pub async fn submit_native_backpressured_on<R, F>(
        &self,
        owner: usize,
        job: F,
        timeout: Duration,
    ) -> Result<AsyncInstanceTicket<R>, InstanceError>
    where
        R: Send + 'static,
        F: for<'vm> FnOnce(&NativeSession<'vm>) -> Result<R, InstanceError> + Send + 'static,
    {
        let reservation = self.reserve_owner(owner, timeout).await?;
        let (send, response) = tokio::sync::oneshot::channel();
        self.native_request_reserved(reservation, job, move |value| send.send(value).is_ok())?;
        Ok(AsyncInstanceTicket { response })
    }
    /// Submit an explicitly bounded Scheme Actor batch through shared capacity.
    /// Batch error/drain and cooperative quantum are the existing native semantics;
    /// arbitrary Scheme calls are never implicitly fused or translated.
    /// # Errors
    /// Preserves native batch validation and shared-capacity admission failures.
    pub async fn submit_numeric_batch_backpressured<T>(
        &self,
        items: Vec<Vec<T>>,
        admission: super::BatchAdmission,
    ) -> Result<AsyncInstanceTicket<Vec<Vec<T>>>, InstanceError>
    where
        T: NumericElement + Send + 'static,
    {
        super::batch::validate(&items, admission.yield_every, self.maximum_input())?;
        self.submit_native_backpressured_on(
            admission.owner,
            move |session| super::batch::execute(session, &items, admission.yield_every, |_| {}),
            admission.timeout,
        )
        .await
    }
}
