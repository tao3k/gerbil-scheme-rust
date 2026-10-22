//! Nonblocking native reservations shared with the existing capacity fence.
use super::{
    InstanceDispatcher, InstanceError,
    capacity::{Capacity, CapacityPermit, ReservedOwner},
};
use std::sync::Arc;
impl Capacity {
    pub(super) fn try_pair(
        self: &Arc<Self>,
        units: usize,
    ) -> Result<CapacityPermit, InstanceError> {
        let units = self.units(units)?;
        let work = self
            .work
            .as_ref()
            .map(|work| Arc::clone(work).try_acquire_many_owned(units))
            .transpose()
            .map_err(|error| super::capacity::immediate_error(&error))?;
        let slot = Arc::clone(&self.slots)
            .try_acquire_owned()
            .map_err(|error| super::capacity::immediate_error(&error))?;
        Ok(CapacityPermit {
            capacity: Arc::clone(self),
            slot: Some(slot),
            work,
            active: None,
            staged: None,
        })
    }
}
impl InstanceDispatcher {
    pub(super) fn try_reserve_owner(
        &self,
        owner: usize,
        units: usize,
        expected: &Arc<Capacity>,
    ) -> Result<ReservedOwner, InstanceError> {
        let capacity = {
            let senders = self
                .admission
                .senders
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let target = senders.owners.get(owner).ok_or(InstanceError::Closed)?;
            let wake = target.wake.as_ref().ok_or(InstanceError::Closed)?;
            if wake.closed.load(std::sync::atomic::Ordering::Acquire)
                || wake.failed.load(std::sync::atomic::Ordering::Acquire)
            {
                return Err(InstanceError::Closed);
            }
            let current = target.capacity.as_ref().ok_or(InstanceError::Closed)?;
            if !Arc::ptr_eq(current, expected) {
                return Err(InstanceError::Closed);
            }
            Arc::clone(current)
        };
        let permit = capacity.try_pair(units)?;
        Ok(ReservedOwner {
            owner,
            permit,
            units,
        })
    }
}

#[cfg(test)]
#[path = "../../tests/unit/capacity_try.rs"]
mod tests;
