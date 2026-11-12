//! Actual Scheme turn ownership counters, read outside timed work.
use super::{InstanceDispatcher, InstanceError, InstanceTicket};
/// Counts execution ownership transitions, never successful work or root retirement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeActorTurnObservation {
    pub enabled: bool,
    pub outer_grants: u64,
    pub quantum_yields: u64,
    pub budget_yields: u64,
}
impl InstanceDispatcher {
    /// Read ABI19 native counters. A release without a held lease is not counted.
    /// # Errors
    /// Unsupported/closed owner or invalid native metadata.
    pub fn scheme_service_actor_turn_on(
        &self,
        owner: usize,
    ) -> Result<InstanceTicket<SchemeActorTurnObservation>, InstanceError> {
        self.work_plan_version(owner, 4)?;
        self.submit_control_on(owner, |session| {
            let value = super::service::command(session, &[31], None)?;
            let values = session.copy_numeric::<i64>(&value)?;
            let [enabled, grants, quantum, budget] = values.as_slice() else {
                return Err(InstanceError::InvalidConfig("Actor turn observation shape"));
            };
            if !matches!(*enabled, 0 | 1)
                || (*enabled == 0 && (*grants != 0 || *quantum != 0 || *budget != 0))
            {
                return Err(InstanceError::InvalidConfig("Actor turn observation mode"));
            }
            Ok(SchemeActorTurnObservation {
                enabled: *enabled == 1,
                outer_grants: u64::try_from(*grants)
                    .map_err(|_| InstanceError::InvalidConfig("Actor grants"))?,
                quantum_yields: u64::try_from(*quantum)
                    .map_err(|_| InstanceError::InvalidConfig("Actor yields"))?,
                budget_yields: u64::try_from(*budget)
                    .map_err(|_| InstanceError::InvalidConfig("Actor budget yields"))?,
            })
        })
    }
}
