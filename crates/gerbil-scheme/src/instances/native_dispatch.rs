//! Typed jobs enter Scheme only on their fixed native owner.
use super::{
    InstanceDispatcher, InstanceError, InstanceTicket, NativeSession, Request, pool::Work,
};
use std::{
    cell::Cell,
    sync::{Arc, atomic::AtomicBool, mpsc},
    time::Instant,
};
impl InstanceDispatcher {
    /// Schedule native construction, Scheme execution and owned projection.
    /// # Errors
    /// Reports admission errors; task and native errors arrive through the ticket.
    pub fn submit_native_on<R, F>(
        &self,
        owner: usize,
        job: F,
    ) -> Result<InstanceTicket<R>, InstanceError>
    where
        R: Send + 'static,
        F: for<'vm> FnOnce(&NativeSession<'vm>) -> Result<R, InstanceError> + Send + 'static,
    {
        let (response, result) = mpsc::channel();
        let started = self.native_request(owner, job, move |value| response.send(value).is_ok())?;
        Ok(InstanceTicket {
            started,
            response: result,
        })
    }
    pub(super) fn native_request<R, F>(
        &self,
        owner: usize,
        job: F,
        deliver: impl FnOnce(super::types::Completed<R>) -> bool + Send + 'static,
    ) -> Result<Arc<AtomicBool>, InstanceError>
    where
        R: Send + 'static,
        F: for<'vm> FnOnce(&NativeSession<'vm>) -> Result<R, InstanceError> + Send + 'static,
    {
        let request = self.native_work(job, deliver);
        let started = Arc::clone(&request.started);
        self.enqueue(owner, request)?;
        Ok(started)
    }
    #[cfg(feature = "tokio-dispatch")]
    pub(super) fn native_request_reserved<R, F>(
        &self,
        reservation: super::capacity::ReservedOwner,
        job: F,
        deliver: impl FnOnce(super::types::Completed<R>) -> bool + Send + 'static,
    ) -> Result<(), InstanceError>
    where
        R: Send + 'static,
        F: for<'vm> FnOnce(&NativeSession<'vm>) -> Result<R, InstanceError> + Send + 'static,
    {
        let mut request = self.native_work(job, deliver);
        request.capacity = Some(reservation.permit);
        self.enqueue(reservation.owner, request)
    }
    fn native_work<R, F>(
        &self,
        job: F,
        deliver: impl FnOnce(super::types::Completed<R>) -> bool + Send + 'static,
    ) -> Request
    where
        R: Send + 'static,
        F: for<'vm> FnOnce(&NativeSession<'vm>) -> Result<R, InstanceError> + Send + 'static,
    {
        let maximum_input = self.maximum_input();
        let maximum_output = self.maximum_output();
        let started = Arc::new(AtomicBool::new(false));
        Request {
            started,
            #[cfg(feature = "tokio-dispatch")]
            capacity: None,
            work: Work::Native(Box::new(move |image| {
                let first = image.next_retained.get();
                let session = NativeSession {
                    image,
                    maximum_input,
                    maximum_output,
                    transferred: Cell::new(0),
                };
                let value = if crate::instance_backend::images_ready() {
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| job(&session)))
                        .unwrap_or(Err(InstanceError::TaskPanicked))
                } else {
                    Err(InstanceError::Closed)
                };
                if value.is_err() {
                    image.rollback_retained(first);
                }
                let value = image.balanced().and(value);
                if !deliver((value, Instant::now())) {
                    // An abandoned reply must not retain newly created owner roots.
                    image.rollback_retained(first);
                }
            })),
        }
    }
}
