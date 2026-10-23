//! Snapshot application declarations once, before asynchronous admission.
use super::{
    InstanceError, NativeRoot, NativeSession, SchemeServiceEntryTrace, SchemeServiceLane,
    SchemeServiceRequest,
};
use std::{num::NonZeroUsize, sync::Arc};
/// Native Scheme queue selection, independent of cross-instance routing class.
/// Short is a cooperative declaration, not a wall-clock execution guarantee.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemeServiceExecution {
    General,
    Short,
}
pub(super) struct Checked<M> {
    message: M,
    units: NonZeroUsize,
    lane: Option<SchemeServiceLane>,
    pub(super) execution: SchemeServiceExecution,
    trace: Option<Arc<SchemeServiceEntryTrace>>,
}
impl<M: SchemeServiceRequest> Checked<M> {
    pub(super) fn new(message: M) -> Result<Self, InstanceError> {
        let units = NonZeroUsize::new(message.work_units())
            .ok_or(InstanceError::InvalidConfig("zero Scheme work charge"))?;
        let lane = message.lane();
        let execution = if message.short_service() {
            SchemeServiceExecution::Short
        } else {
            SchemeServiceExecution::General
        };
        if execution == SchemeServiceExecution::Short && (units.get() != 1 || lane.is_some()) {
            return Err(InstanceError::InvalidConfig(
                "short service requires one stateless work unit",
            ));
        }
        let trace = message.entry_trace();
        Ok(Self {
            message,
            units,
            lane,
            execution,
            trace,
        })
    }
}
impl<M: SchemeServiceRequest> SchemeServiceRequest for Checked<M> {
    type Output = M::Output;
    fn work_units(&self) -> usize {
        self.units.get()
    }
    fn short_service(&self) -> bool {
        self.execution == SchemeServiceExecution::Short
    }
    fn lane(&self) -> Option<SchemeServiceLane> {
        self.lane
    }
    fn entry_trace(&self) -> Option<Arc<SchemeServiceEntryTrace>> {
        self.trace.clone()
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.message.construct(session)
    }
    fn construct_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &super::SchemeServiceInput<'_>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.message.construct_service(session, input)
    }
    fn enter_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &super::SchemeServiceInput<'_>,
    ) -> Result<super::SchemeServiceSubmission<'vm>, InstanceError> {
        self.message.enter_service(session, input)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        self.message.project(session, output)
    }
}
