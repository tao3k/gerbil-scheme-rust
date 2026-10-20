//! Tokio dispatch never enters a VM; dedicated native owners execute jobs.
use super::{InstanceDispatcher, InstanceError, NativeSession};
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
