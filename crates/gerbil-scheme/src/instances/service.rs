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
    /// Optional caller/owner entry timing, disabled by default. Diagnostic only.
    fn entry_trace(&self) -> Option<Arc<super::SchemeServiceEntryTrace>> {
        None
    }

    /// Pin a stateful session/continuation to a persistent Scheme service Actor.
    /// Independent requests use the available Scheme workers by default.
    fn lane(&self) -> Option<SchemeServiceLane> {
        None
    }
    /// Opt into Scheme's short queue and bounded completion turns.
    /// Requires one declared work unit and no Actor lane. This is a cooperative
    /// work budget, not a wall-clock limit or preemption of Scheme computation.
    fn short_service(&self) -> bool {
        false
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
    /// Construct the complete service input. Override only with equivalent
    /// payload semantics; generic Scheme requests retain their normal constructor.
    /// # Errors
    /// Native construction, input bounds or application validation failure.
    fn construct_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &super::SchemeServiceInput<'_>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        let value = self.construct(session)?;
        input.wrap(session, &value)
    }
    /// Construct and admit one service input. The returned root is the native
    /// identity receipt, not the payload. Use the context's submission methods
    /// so an attempted entry remains visible even if native code fails.
    /// # Errors
    /// Construction, quotas, service admission or caught Scheme exceptions.
    fn enter_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &super::SchemeServiceInput<'_>,
    ) -> Result<super::SchemeServiceSubmission<'vm>, InstanceError> {
        let value = self.construct_service(session, input)?;
        input.submit(session, &value)
    }
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
    fn construct_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &super::SchemeServiceInput<'_>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        input.numeric_batch(session, &self.items, self.yield_every)
    }
    fn enter_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &super::SchemeServiceInput<'_>,
    ) -> Result<super::SchemeServiceSubmission<'vm>, InstanceError> {
        input.enter_numeric(session, &self.items, self.yield_every)
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
        let message = super::service_contract::Checked::new(message)?;
        self.validate_service_on(owner, &message)?;
        let reservation = self
            .reserve_owner(owner, timeout, message.work_units())
            .await?;
        self.publish_reserved_service(message, reservation)
    }
    pub(super) fn publish_reserved_service<M: SchemeServiceRequest>(
        &self,
        message: M,
        reservation: super::capacity::ReservedOwner,
    ) -> Result<AsyncInstanceTicket<M::Output>, InstanceError> {
        let owner = reservation.owner;
        let trace = message.entry_trace();
        let lane = message.lane();
        let short = message.short_service();
        let units = message.work_units();
        if short && (units != 1 || lane.is_some()) {
            return Err(InstanceError::InvalidConfig(
                "short service requires one stateless work unit",
            ));
        }
        self.validate_service_on(owner, &message)?;
        if units != reservation.units {
            return Err(InstanceError::InvalidConfig(
                "Scheme reservation work mismatch",
            ));
        }
        if let Some(trace) = &trace {
            trace.mark(super::SchemeServiceEntryPhase::CapacityReady);
        }
        let (send, response) = oneshot::channel();
        let request = Request {
            started: Arc::new(AtomicBool::new(false)),
            capacity: Some(reservation.permit),
            work_units: units,
            work: Work::Service(Box::new(TypedJob {
                message,
                send,
                lane,
                short,
                trace,
                submitted: std::cell::Cell::new(false),
            })),
        };
        self.enqueue(owner, request)?;
        Ok(AsyncInstanceTicket { response })
    }
    pub(super) fn validate_service_on<M: SchemeServiceRequest>(
        &self,
        owner: usize,
        message: &M,
    ) -> Result<(), InstanceError> {
        let short = message.short_service();
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
        }
        Ok(())
    }
}
/// Opaque owner-local identity; never a Scheme pointer or cross-owner authority.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct JobId(pub i64);
pub(super) struct Completion {
    pub(super) id: JobId,
    pub(super) ready: Option<super::service_cycle::ReadyResult>,
}
pub(super) enum Retirement {
    Returned,
    Drain(Vec<tokio::sync::OwnedSemaphorePermit>),
}
pub(super) struct Start {
    pub input: usize,
    pub output: usize,
    pub progress: bool,
    pub fused: bool,
}
pub(super) trait Job: Send {
    fn entry_trace(&self) -> Option<&Arc<super::SchemeServiceEntryTrace>>;
    fn short_service(&self) -> bool;
    fn start(
        &mut self,
        image: &super::image::Image,
        start: &super::service::Start,
    ) -> Result<JobId, InstanceError>;
    fn may_have_entered(&self) -> bool;
    fn complete(
        self: Box<Self>,
        image: &super::image::Image,
        completed: Completion,
        input: usize,
        output: usize,
    ) -> Retirement;
    fn fail(self: Box<Self>, error: InstanceError);
}
pub(super) struct TypedJob<M: SchemeServiceRequest> {
    pub(super) message: M,
    pub(super) send: oneshot::Sender<super::types::Completed<M::Output>>,
    pub(super) submitted: std::cell::Cell<bool>,
    pub(super) lane: Option<SchemeServiceLane>,
    pub(super) short: bool,
    pub(super) trace: Option<Arc<super::SchemeServiceEntryTrace>>,
}
impl<M: SchemeServiceRequest> Job for TypedJob<M> {
    fn entry_trace(&self) -> Option<&Arc<super::SchemeServiceEntryTrace>> {
        self.trace.as_ref()
    }
    fn short_service(&self) -> bool {
        self.short
    }
    fn may_have_entered(&self) -> bool {
        self.submitted.get()
    }
    fn start(
        &mut self,
        image: &super::image::Image,
        start: &super::service::Start,
    ) -> Result<JobId, InstanceError> {
        let mut result = None;
        super::native_dispatch::run_native(
            image,
            start.input,
            start.output,
            |session| {
                if let Some(trace) = &self.trace {
                    trace.mark(super::SchemeServiceEntryPhase::Construct);
                }
                let lane = self.lane.map_or(-1, |lane| i64::try_from(lane.0).unwrap());
                let header = [
                    1,
                    lane,
                    i64::from(self.short),
                    i64::try_from(self.message.work_units()).map_err(|_| {
                        InstanceError::InvalidConfig("native progress unit overflow")
                    })?,
                ];
                let length = if start.progress {
                    4
                } else if self.short {
                    3
                } else {
                    2
                };
                let context =
                    super::SchemeServiceInput::new(header, length, start.fused, &self.submitted);
                let token = self.message.enter_service(session, &context)?;
                let ids = session.copy_numeric::<i64>(&token.receipt)?;
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
        completed: Completion,
        input: usize,
        output: usize,
    ) -> Retirement {
        let Self { message, send, .. } = *self;
        let slot_taken = std::cell::Cell::new(completed.ready.is_none());
        super::native_dispatch::run_native(
            image,
            input,
            output,
            |session| {
                let value = match completed.ready {
                    Some(ready) => ready.project(session, &slot_taken)?,
                    None => command(session, &[2, completed.id.0], None)?,
                };
                message.project(session, &value)
            },
            |value| send.send(value).is_ok(),
        );
        if slot_taken.get() {
            Retirement::Returned
        } else {
            Retirement::Drain(Vec::new())
        }
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

pub(super) fn envelope<'vm>(
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

pub(super) fn call<'vm>(
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
