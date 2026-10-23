//! Nonblocking native reservations shared with the existing capacity fence.
use super::{
    InstanceDispatcher, InstanceError,
    capacity::{Capacity, CapacityPermit, ReservedOwner},
};
use std::sync::Arc;
use tokio::sync::OwnedSemaphorePermit;
impl Capacity {
    pub(super) fn try_pair(
        self: &Arc<Self>,
        units: usize,
    ) -> Result<CapacityPermit, InstanceError> {
        let units = self.units(units)?;
        let work = self.try_work(units)?;
        let slot = self.try_slot()?;
        Ok(CapacityPermit {
            capacity: Arc::clone(self),
            slot: Some(slot),
            work,
            active: None,
            staged: None,
        })
    }
    fn try_work(&self, units: u32) -> Result<Option<OwnedSemaphorePermit>, InstanceError> {
        self.work
            .as_ref()
            .map(|work| Arc::clone(work).try_acquire_many_owned(units))
            .transpose()
            .map_err(|e| super::capacity::immediate_error(&e))
    }
    fn try_slot(&self) -> Result<OwnedSemaphorePermit, InstanceError> {
        Arc::clone(&self.slots)
            .try_acquire_owned()
            .map_err(|e| super::capacity::immediate_error(&e))
    }
}
/// Partial immediate acquisition, distinct from an accepted native request.
/// Callers release their entry fence before this guard can return credits.
pub(super) struct EntryCredits {
    capacity: Arc<Capacity>,
    active: Option<OwnedSemaphorePermit>,
    work: Option<OwnedSemaphorePermit>,
    slot: Option<OwnedSemaphorePermit>,
}
impl EntryCredits {
    pub(super) fn new(capacity: Arc<Capacity>, active: OwnedSemaphorePermit) -> Self {
        Self {
            capacity,
            active: Some(active),
            work: None,
            slot: None,
        }
    }
    pub(super) fn acquire(&mut self, units: usize) -> Result<(), InstanceError> {
        let units = self.capacity.units(units)?;
        self.work = self.capacity.try_work(units)?;
        self.slot = Some(self.capacity.try_slot()?);
        Ok(())
    }
    pub(super) fn finish(mut self, staged: super::StagedWork) -> CapacityPermit {
        CapacityPermit {
            capacity: Arc::clone(&self.capacity),
            active: self.active.take(),
            work: self.work.take(),
            slot: self.slot.take(),
            staged: Some(staged),
        }
    }
}
impl Drop for EntryCredits {
    fn drop(&mut self) {
        let native_changed = self.work.is_some() || self.slot.is_some();
        drop(self.active.take());
        drop(self.work.take());
        drop(self.slot.take());
        // Active-only rollback wakes its semaphore waiters, but did not change
        // the shared native request/work ledger used by short admission.
        if native_changed {
            self.capacity.signal();
        }
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
