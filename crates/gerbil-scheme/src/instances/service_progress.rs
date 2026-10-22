//! Native service hints are separate from retained capacity and rooted graphs.
use super::{InstanceDispatcher, InstanceError, InstanceTicket};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
/// Last native-turn receipt, not a CPU/time estimate or resource-release authority.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SchemeServiceProgress {
    pub enabled: bool,
    /// Owner has armed/rechecked its inbox and declared the native wait path.
    /// This is not kernel blocked/runnable state or an OS time guarantee.
    pub waiting_for_claim: bool,
    /// Capacity-backed peer reservations/inbox work not yet submitted to Scheme.
    /// A provisional reservation can roll back; this is not a delivery receipt.
    pub inbox_remaining: u64,
    /// Cumulative successful capacity-backed peer inbox admissions.
    pub peer_deliveries: u64,
    pub bulk_remaining: u32,
    pub short_remaining: u32,
}
#[derive(Debug, Default)]
pub(super) struct Progress {
    pub enabled: AtomicBool,
    pub waiting_for_claim: AtomicBool,
    remaining: AtomicU64,
    pub queued: Arc<AtomicU64>,
    pub peer_deliveries: AtomicU64,
}
impl Progress {
    pub(super) fn snapshot(&self) -> SchemeServiceProgress {
        let bits = self.remaining.load(Ordering::Acquire);
        SchemeServiceProgress {
            enabled: self.enabled.load(Ordering::Acquire),
            waiting_for_claim: self.waiting_for_claim.load(Ordering::Acquire),
            inbox_remaining: self.queued.load(Ordering::Acquire),
            peer_deliveries: self.peer_deliveries.load(Ordering::Acquire),
            bulk_remaining: u32::try_from(bits >> 32).unwrap(),
            short_remaining: u32::try_from(bits & u64::from(u32::MAX)).unwrap(),
        }
    }
    pub(super) fn load(&self) -> u64 {
        let before = self.queued.load(Ordering::Acquire);
        let s = self.snapshot();
        // Submission charges native progress before clearing the inbox hint.
        // Retain the larger inbox observation across that advisory transition.
        u64::from(s.bulk_remaining) + u64::from(s.short_remaining) + before.max(s.inbox_remaining)
    }
    pub(super) fn charge(&self, units: usize, short: bool) {
        if self.enabled.load(Ordering::Acquire) {
            let units = u64::try_from(units).unwrap();
            self.remaining
                .fetch_add(if short { units } else { units << 32 }, Ordering::AcqRel);
        }
    }
    pub(super) fn receipt(&self, values: &mut Vec<i64>) -> Result<(), InstanceError> {
        if !self.enabled.load(Ordering::Acquire) {
            return Ok(());
        }
        let [-6, bulk, short, ..] = values.as_slice() else {
            return Err(InstanceError::InvalidConfig(
                "native service progress receipt",
            ));
        };
        let bulk = u32::try_from(*bulk)
            .map_err(|_| InstanceError::InvalidConfig("native bulk progress bound"))?;
        let short = u32::try_from(*short)
            .map_err(|_| InstanceError::InvalidConfig("native short progress bound"))?;
        self.remaining.store(
            (u64::from(bulk) << 32) | u64::from(short),
            Ordering::Release,
        );
        values.drain(..3);
        Ok(())
    }
}
impl InstanceDispatcher {
    pub(super) fn require_progress_owners(&self, owners: &[usize]) -> Result<(), InstanceError> {
        if owners
            .iter()
            .any(|&o| !self.scheme_service_progress_on(o).is_ok_and(|p| p.enabled))
        {
            return Err(InstanceError::InvalidConfig(
                "progress fairness requires native progress receipts",
            ));
        }
        Ok(())
    }
    /// Enable native unit receipts on an idle ABI-v6 owner. No defaults change.
    /// Activation is fixed until VM shutdown, so registered or pending producers
    /// cannot lose this capability. Native Actor batches report stable item identities; other handlers may
    /// explicitly report units in Scheme or remain conservatively charged until
    /// completion. Hints never release active, work, slot or native-root guards.
    /// # Errors
    /// Unsupported/closed owner, active capacity or native configuration failure.
    pub fn enable_scheme_service_progress_on(
        &self,
        owner: usize,
    ) -> Result<InstanceTicket<()>, InstanceError> {
        let wake = {
            let senders = self
                .admission
                .senders
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let target = senders.owners.get(owner).ok_or(InstanceError::Closed)?;
            let wake = target.wake.as_ref().ok_or(InstanceError::Closed)?;
            let capacity = target
                .capacity
                .as_ref()
                .ok_or(InstanceError::InvalidConfig(
                    "progress requires native budgets",
                ))?;
            if wake.service_version.load(Ordering::Acquire) < 6
                || capacity.work_limit > u32::MAX as usize
                || capacity.slots.available_permits() != capacity.slot_limit
                || capacity.producer.available_permits() != 1
                || capacity
                    .work
                    .as_ref()
                    .is_none_or(|w| w.available_permits() != capacity.work_limit)
            {
                return Err(InstanceError::InvalidConfig(
                    "progress requires idle ABI-v6 capacity",
                ));
            }
            Arc::clone(wake)
        };
        self.submit_native_on(owner, move |session| {
            let value = super::service::command(session, &[12, 1], None)?;
            if session.copy_numeric::<i64>(&value)? != [0, 1] {
                return Err(InstanceError::InvalidConfig(
                    "native progress configuration receipt",
                ));
            }
            wake.progress.remaining.store(0, Ordering::Release);
            wake.progress.enabled.store(true, Ordering::Release);
            Ok(())
        })
    }
    /// Read cached native-turn metadata without entering Scheme or sampling GC.
    /// # Errors
    /// Unknown owner or an owner without a Scheme service.
    pub fn scheme_service_progress_on(
        &self,
        owner: usize,
    ) -> Result<SchemeServiceProgress, InstanceError> {
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
        Ok(wake.progress.snapshot())
    }
}
#[cfg(test)]
#[path = "../../tests/unit/service_progress.rs"]
mod tests;
