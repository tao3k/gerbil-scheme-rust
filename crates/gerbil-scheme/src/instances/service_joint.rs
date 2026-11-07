//! Scheme owns short-dispatch obligations, scheduler checkpoints and wake receipts.
use super::{InstanceDispatcher, InstanceError, InstanceTicket};
use std::sync::atomic::Ordering;
/// Actual Scheme state, not a Rust estimate of runnable or collector time.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeJointProgress {
    pub enabled: bool,
    pub pending_dispatch: u64,
    pub checkpoints: u64,
    pub worker_starts: u64,
}
impl InstanceDispatcher {
    /// Opt into a Scheme scheduler opportunity before bulk harvest or native wait
    /// while a short request awaits worker acknowledgement. Requires an idle
    /// ABI 13 `PipelineCycle`; selecting any completion policy disables it.
    /// A checkpoint does not clear an obligation or wait for user work to finish.
    /// # Errors
    /// Closed/unsupported owner, non-idle service, or rejected Scheme configuration.
    pub fn configure_scheme_service_joint_progress_on(
        &self,
        owner: usize,
        enabled: bool,
    ) -> Result<InstanceTicket<bool>, InstanceError> {
        self.joint_version(owner)?;
        self.submit_control_on(owner, move |session| {
            let value = super::service::command(session, &[20, i64::from(enabled)], None)?;
            if session.copy_numeric::<i64>(&value)? != [i64::from(enabled)] {
                return Err(InstanceError::InvalidConfig("joint progress receipt"));
            }
            Ok(enabled)
        })
    }
    /// Read acknowledged worker starts, outstanding dispatches and checkpoints.
    /// Checkpoints are scheduling opportunities, not completed work or time bounds.
    /// # Errors
    /// Closed/unsupported owner or malformed native receipt.
    pub fn scheme_service_joint_progress_on(
        &self,
        owner: usize,
    ) -> Result<InstanceTicket<SchemeJointProgress>, InstanceError> {
        self.joint_version(owner)?;
        self.submit_control_on(owner, |session| {
            let value = super::service::command(session, &[21], None)?;
            let value = session.copy_numeric::<i64>(&value)?;
            let [mode, pending, checkpoints, starts] = value.as_slice() else {
                return Err(InstanceError::InvalidConfig("joint progress shape"));
            };
            if !matches!(mode, 0 | 1) {
                return Err(InstanceError::InvalidConfig("joint progress mode"));
            }
            let count = |n| {
                u64::try_from(n).map_err(|_| InstanceError::InvalidConfig("joint progress count"))
            };
            Ok(SchemeJointProgress {
                enabled: *mode == 1,
                pending_dispatch: count(*pending)?,
                checkpoints: count(*checkpoints)?,
                worker_starts: count(*starts)?,
            })
        })
    }
    fn joint_version(&self, owner: usize) -> Result<(), InstanceError> {
        let senders = self
            .admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let wake = senders
            .owners
            .get(owner)
            .and_then(|o| o.wake.as_ref())
            .ok_or(InstanceError::Closed)?;
        if wake.service_version.load(Ordering::Acquire) < 13 {
            return Err(InstanceError::InvalidConfig(
                "joint progress requires ABI 13",
            ));
        }
        Ok(())
    }
}

/// Scheme-owned short-request progress across runnable, registration and grant.
/// Reentry and suspension may take more leases without adding first grants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeExecutionProgress {
    pub enabled: bool,
    pub pending_execution: u64,
    pub issued: u64,
    pub runnable: u64,
    pub registered: u64,
    pub first_grants: u64,
    pub finished_without_grant: u64,
}
