//! Explicit typed async Scheme work; only owned projections leave the owner.
use super::{
    AsyncInstanceTicket, InstanceDispatcher, InstanceError, NativeRoot, NativeSession, Request,
    pool::Work,
};
use crate::NumericElement;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::oneshot;

/// An application-owned native input/projection pair. Scheme owns execution.
/// Constructors run on the fixed owner; temporary roots cannot be sent away.
/// No continuation may escape a foreign frame or finalize the owning runtime.
pub trait SchemeServiceRequest: Send + 'static {
    type Output: Send + 'static;
    /// Pin a stateful session/continuation to a persistent Scheme service Actor.
    /// Independent requests use the available Scheme workers by default.
    fn lane(&self) -> Option<SchemeServiceLane> {
        None
    }
    /// Charge outstanding work through the shared owner budget until projection.
    fn work_units(&self) -> usize;
    /// Construct a native Scheme payload on its owner.
    /// # Errors
    /// Native construction, bounds or application validation failures.
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError>;
    /// Project a completed Scheme value before releasing its temporary root.
    /// # Errors
    /// Native type/bounds or application result-validation failures.
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError>;
}
/// Native numeric Actor-batch envelope for a Scheme service handler.
#[derive(Debug)]
pub struct NumericServiceBatch<T> {
    pub items: Vec<Vec<T>>,
    pub yield_every: usize,
}
impl<T: NumericElement + Send + 'static> SchemeServiceRequest for NumericServiceBatch<T> {
    type Output = Vec<Vec<T>>;
    fn work_units(&self) -> usize {
        self.items.len()
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        session.numeric_batch_envelope(&self.items, self.yield_every)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        super::batch::project(session, output, self.items.len())
    }
}
/// Stable Actor affinity inside one fixed owner; never a transferable VM handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeServiceLane(pub usize);
impl InstanceDispatcher {
    /// Admit explicit asynchronous Scheme work under the same slot/work budgets.
    /// Accepted work, its Scheme graph and permits survive caller timeout/drop.
    /// Scheme completion plus native projection/retirement returns the permits.
    /// # Errors
    /// Requires an opted-in service, configured capacity, valid work charge,
    /// finite admission deadline and a live owner. No default backend is changed.
    pub async fn submit_scheme_service_backpressured_on<M: SchemeServiceRequest>(
        &self,
        owner: usize,
        message: M,
        timeout: Duration,
    ) -> Result<AsyncInstanceTicket<M::Output>, InstanceError> {
        let lane = message.lane();
        {
            let senders = self
                .admission
                .senders
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let target = senders.owners.get(owner).ok_or(InstanceError::Closed)?;
            let wake = target
                .wake
                .as_ref()
                .ok_or(InstanceError::InvalidConfig("Scheme service not enabled"))?;
            if wake.closed.load(Ordering::Acquire) {
                return Err(InstanceError::Closed);
            }
            if lane.is_some_and(|lane| lane.0 >= wake.lanes.load(Ordering::Acquire)) {
                return Err(InstanceError::InvalidConfig(
                    "Scheme service lane out of range",
                ));
            }
        }
        let units = message.work_units();
        let reservation = self.reserve_owner(owner, timeout, units).await?;
        let (send, response) = oneshot::channel();
        let request = Request {
            started: Arc::new(AtomicBool::new(false)),
            capacity: Some(reservation.permit),
            work_units: units,
            work: Work::Service(Box::new(TypedJob {
                message,
                send,
                lane,
                submitted: false,
            })),
        };
        self.enqueue(owner, request)?;
        Ok(AsyncInstanceTicket { response })
    }
}
/// Opaque owner-local identity; never a Scheme pointer or cross-owner authority.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct JobId(pub i64);
pub(super) trait Job: Send {
    fn start(
        &mut self,
        image: &super::image::Image,
        input: usize,
        output: usize,
    ) -> Result<JobId, InstanceError>;
    fn may_have_entered(&self) -> bool;
    fn complete(
        self: Box<Self>,
        image: &super::image::Image,
        id: JobId,
        input: usize,
        output: usize,
    );
    fn fail(self: Box<Self>, error: InstanceError);
}
struct TypedJob<M: SchemeServiceRequest> {
    message: M,
    send: oneshot::Sender<super::types::Completed<M::Output>>,
    submitted: bool,
    lane: Option<SchemeServiceLane>,
}
impl<M: SchemeServiceRequest> Job for TypedJob<M> {
    fn may_have_entered(&self) -> bool {
        self.submitted
    }
    fn start(
        &mut self,
        image: &super::image::Image,
        input: usize,
        output: usize,
    ) -> Result<JobId, InstanceError> {
        let mut result = None;
        super::native_dispatch::run_native(
            image,
            input,
            output,
            |session| {
                let value = self.message.construct(session)?;
                let lane = self.lane.map_or(-1, |lane| i64::try_from(lane.0).unwrap());
                let input = envelope(session, &[1, lane], Some(&value))?;
                self.submitted = true;
                let token = call(session, &input)?;
                let ids = session.copy_numeric::<i64>(&token)?;
                if ids.len() != 1 || ids[0] <= 0 {
                    return Err(InstanceError::InvalidConfig("Scheme service identity"));
                }
                Ok(JobId(ids[0]))
            },
            |value| {
                result = Some(value.0);
                true
            },
        );
        result.unwrap()
    }
    fn complete(
        self: Box<Self>,
        image: &super::image::Image,
        id: JobId,
        input: usize,
        output: usize,
    ) {
        let Self { message, send, .. } = *self;
        super::native_dispatch::run_native(
            image,
            input,
            output,
            |session| {
                let value = command(session, &[2, id.0], None)?;
                message.project(session, &value)
            },
            |value| send.send(value).is_ok(),
        );
    }
    fn fail(self: Box<Self>, error: InstanceError) {
        let _ = self.send.send((Err(error), Instant::now()));
    }
}
pub(super) fn command<'vm>(
    session: &NativeSession<'vm>,
    header: &[i64],
    payload: Option<&NativeRoot<'_>>,
) -> Result<NativeRoot<'vm>, InstanceError> {
    let envelope = envelope(session, header, payload)?;
    call(session, &envelope)
}

fn envelope<'vm>(
    session: &NativeSession<'vm>,
    header: &[i64],
    payload: Option<&NativeRoot<'_>>,
) -> Result<NativeRoot<'vm>, InstanceError> {
    let operation = session.numeric_vector(header)?;
    let value = if let Some(payload) = payload {
        session.vector(&[&operation, payload])?
    } else {
        session.vector(&[&operation])?
    };
    Ok(value)
}

fn call<'vm>(
    session: &NativeSession<'vm>,
    input: &NativeRoot<'_>,
) -> Result<NativeRoot<'vm>, InstanceError> {
    session.local(input)?;
    let call = session
        .image
        .service
        .ok_or(InstanceError::InvalidConfig("Scheme service not enabled"))?;
    // SAFETY: private rooted unary entry resolved before setup; same owner/image.
    session.call_result(unsafe { call(input.root) })
}
