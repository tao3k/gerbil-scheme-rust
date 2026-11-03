//! Bounded publication of admitted native inputs; Scheme chooses execution.
use super::{InstanceDispatcher, InstanceError};
use std::sync::atomic::Ordering;
impl InstanceDispatcher {
    /// Opt into feeding up to `maximum` queued or resident-producer service inputs before advancing
    /// Scheme. FIFO, native slot/work permits and Actor affinity are preserved.
    /// The default is one. This bounds input count, not arbitrary Scheme time.
    /// # Errors
    /// Requires an open service owner with idle, configured native capacity and
    /// a positive limit no larger than its request-slot budget.
    pub fn configure_scheme_service_feed_on(
        &self,
        owner: usize,
        maximum: usize,
    ) -> Result<(), InstanceError> {
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
        let capacity = target
            .capacity
            .as_ref()
            .ok_or(InstanceError::InvalidConfig(
                "service feed requires native capacity",
            ))?;
        if maximum == 0
            || maximum > target.capacity_limit
            || capacity.slots.available_permits() != target.capacity_limit
            || capacity.producer.available_permits() != 1
            || capacity
                .work
                .as_ref()
                .is_some_and(|work| work.available_permits() != capacity.work_limit)
        {
            return Err(InstanceError::InvalidConfig(
                "service feed bound or active capacity",
            ));
        }
        wake.feed_limit.store(maximum, Ordering::Release);
        Ok(())
    }
}
