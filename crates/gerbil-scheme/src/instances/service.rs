//! Explicit typed async Scheme work; only owned projections leave the owner.
use super::{
    AsyncInstanceTicket, InstanceDispatcher, InstanceError, NativeRoot, NativeSession, Request,
    pool::Work,
};
use crate::NumericElement;
use std::{
    sync::{Arc, atomic::AtomicBool},
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
    /// Optional completion/send intervals, independent of entry diagnostics.
    fn completion_trace(&self) -> Option<Arc<super::SchemeCompletionTrace>> {
        None
    }

    /// Pin a stateful session/continuation to a persistent Scheme service Actor.
    /// Independent requests use the available Scheme workers by default.
    fn lane(&self) -> Option<SchemeServiceLane> {
        None
    }
    /// Opt into Scheme's short queue and bounded completion turns.
    /// Requires one retained admission unit and no Actor lane. This selects a
    /// service class; executed-work safe spans are configured independently with
    /// `configure_scheme_service_work_on`. Neither preempts Scheme computation.
    fn short_service(&self) -> bool {
        false
    }
    /// Retained admission/progress charge through projection, independent of
    /// executed-work safe spans. This is neither CPU time nor execution fuel.
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
    /// Construct and admit one service input. The opaque submission contains the
    /// native identity receipt, never the payload. Use the context's submission methods
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
    /// Project a result through its owner-local slot. The default preserves all
    /// Scheme types; bounded numeric consumers may avoid a temporary value root.
    /// # Errors
    /// Native custody, type, quota or application validation failure.
    fn project_service(
        self,
        session: &NativeSession<'_>,
        output: &super::SchemeServiceOutput<'_>,
    ) -> Result<Self::Output, InstanceError>
    where
        Self: Sized,
    {
        let value = output.rooted(session)?;
        self.project(session, &value)
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
        if self.owner_port(owner).is_none() {
            self.validate_service_on(owner, &message)?;
        }
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
                send: send.into(),
                lane,
                short,
                trace,
                submitted: std::cell::Cell::new(false),
            })),
        };
        if let Some(port) = self.owner_port(owner) {
            port?.publish(request, short, lane)?;
        } else {
            self.enqueue(owner, request)?;
        }
        Ok(AsyncInstanceTicket { response })
    }
    pub(super) fn validate_service_on<M: SchemeServiceRequest>(
        &self,
        owner: usize,
        message: &M,
    ) -> Result<(), InstanceError> {
        let short = message.short_service();
        let lane = message.lane();
        if let Some(port) = self.owner_port(owner) {
            return port?.validate(short, lane);
        }
        let senders = self
            .admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let target = senders.owners.get(owner).ok_or(InstanceError::Closed)?;
        super::owner_admission::validate(target.wake.as_ref(), short, lane)
    }
}
/// Opaque owner-local identity; never a Scheme pointer or cross-owner authority.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct JobId(pub i64);
pub(super) struct Completion {
    pub(super) id: JobId,
    pub(super) ready: Option<super::service_cycle::ReadyResult>,
    pub(super) retire_before_publish: bool,
    pub(super) input: usize,
    pub(super) output: usize,
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
    fn completion_trace(&self) -> Option<Arc<super::SchemeCompletionTrace>>;
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
        retire: &mut dyn FnMut(),
    ) -> Retirement;
    fn fail(self: Box<Self>, error: InstanceError);
}
pub(super) struct TypedJob<M: SchemeServiceRequest> {
    pub(super) message: M,
    pub(super) send: super::service_ready::Reply<M::Output>,
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
    fn completion_trace(&self) -> Option<Arc<super::SchemeCompletionTrace>> {
        self.message.completion_trace()
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
                token.id(session)
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
        retire: &mut dyn FnMut(),
    ) -> Retirement {
        let Self { message, send, .. } = *self;
        let trace = message.completion_trace();
        let slot_taken = std::cell::Cell::new(completed.ready.is_none());
        super::native_dispatch::run_native(
            image,
            completed.input,
            completed.output,
            |session| {
                let output =
                    super::SchemeServiceOutput::new(completed.id, completed.ready, &slot_taken);
                message.project_service(session, &output)
            },
            |value| {
                if completed.retire_before_publish && slot_taken.get() {
                    retire();
                }
                if let Some(trace) = &trace {
                    trace.before_send(value.1);
                }
                let delivered = send.send(value, Some(image.receipt.owner));
                if let Some(trace) = &trace {
                    trace.sent(delivered);
                }
                delivered
            },
        );
        if slot_taken.get() {
            Retirement::Returned
        } else {
            Retirement::Drain(Vec::new())
        }
    }
    fn fail(self: Box<Self>, error: InstanceError) {
        let _ = self.send.send((Err(error), Instant::now()), None);
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
