//! Wake obligations never reserve a payload, credit, native root or owner affinity.
use super::{Port, SchemeShortPool, SchemeShortWake, Source};
use crate::instances::InstanceError;
use std::sync::atomic::Ordering;
impl SchemeShortPool {
    /// Read the actual host notification policy without entering Scheme.
    #[must_use]
    pub fn wake_policy(&self) -> SchemeShortWake {
        if self.source.progress.load(Ordering::Acquire) {
            SchemeShortWake::ProgressFair
        } else if self.source.selective.load(Ordering::Acquire) {
            SchemeShortWake::Selective
        } else {
            SchemeShortWake::Broadcast
        }
    }

    /// Change the opt-in facility's wake policy while all native budgets are idle.
    /// Registration is refreshed by one transition broadcast, outside the fence.
    /// This changes neither Scheme scheduling nor ordinary dispatcher defaults.
    /// # Errors
    /// Rejects closed admission, queued/unpublished inputs or outstanding credits.
    pub fn configure_wake(&self, policy: SchemeShortWake) -> Result<(), InstanceError> {
        self.configure_idle_policy(policy)
    }
    fn configure_idle_policy(&self, policy: SchemeShortWake) -> Result<(), InstanceError> {
        let admission = self
            .source
            .admission
            .upgrade()
            .ok_or(InstanceError::Closed)?;
        let senders = admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut queue = self
            .source
            .queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.source.pending.is_closed() || !admission.accepting.load(Ordering::Acquire) {
            return Err(InstanceError::Closed);
        }
        if !queue.requests.is_empty()
            || self.source.pending.available_permits() != self.source.pending_limit
            || self.source.destinations.iter().any(|d| {
                d.capacity.slots.available_permits() != d.capacity.slot_limit
                    || d.capacity
                        .work
                        .as_ref()
                        .is_some_and(|w| w.available_permits() != d.capacity.work_limit)
            })
        {
            return Err(InstanceError::InvalidConfig(
                "short wake policy requires idle budgets",
            ));
        }
        if policy == SchemeShortWake::ProgressFair
            && self.source.destinations.iter().any(|d| {
                d.wake
                    .upgrade()
                    .is_none_or(|w| !w.progress.enabled.load(Ordering::Acquire))
            })
        {
            return Err(InstanceError::InvalidConfig(
                "progress wake requires native progress receipts",
            ));
        }
        queue.bypass.clear();
        queue.waiters.clear();
        queue.notified.clear();
        for target in &self.source.destinations {
            target.waiting.store(false, Ordering::Release);
        }
        self.source
            .selective
            .store(policy != SchemeShortWake::Broadcast, Ordering::Release);
        self.source
            .progress
            .store(policy == SchemeShortWake::ProgressFair, Ordering::Release);
        drop(queue);
        drop(senders);
        // Owners may still be asleep under the previous policy with no selective
        // registration. Transition notification closes that installation window.
        self.source.broadcast();
        Ok(())
    }
}
impl Source {
    pub(super) fn wake(&self) {
        if !self.selective.load(Ordering::Acquire) {
            self.broadcast();
            return;
        }
        let targets = {
            let mut queue = self
                .queue
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let available = queue.requests.len().saturating_sub(queue.notified.len());
            if self.pending.is_closed() {
                Vec::new()
            } else {
                self.select_targets(&mut queue, available)
            }
        };
        for target in targets {
            if let Some(wake) = target.upgrade() {
                wake.notify_for(true);
            } else {
                self.owner_closed();
            }
        }
    }
    pub(super) fn remove_closed_waiters(&self) {
        let dead: Vec<_> = self
            .destinations
            .iter()
            .enumerate()
            .filter_map(|(i, d)| {
                d.wake
                    .upgrade()
                    .is_none_or(|w| {
                        w.closed.load(Ordering::Acquire) || w.failed.load(Ordering::Acquire)
                    })
                    .then_some(i)
            })
            .collect();
        let mut queue = self
            .queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for index in dead {
            queue.waiters.retain(|i| *i != index);
            queue.notified.remove(&index);
            queue.bypass.remove(&index);
            self.destinations[index]
                .waiting
                .store(false, Ordering::Release);
        }
    }
}
impl Port {
    pub(crate) fn selective(&self) -> bool {
        self.source.selective.load(Ordering::Acquire)
    }
    pub(crate) fn arm_wait(&self) {
        if !self.selective() {
            return;
        }
        let index = self.index;
        let Some(wake) = self.source.destinations[index].wake.upgrade() else {
            return;
        };
        let mut queue = self
            .source
            .queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.source.pending.is_closed()
            || !self.selective()
            || wake.closed.load(Ordering::Acquire)
            || wake.failed.load(Ordering::Acquire)
        {
            return;
        }
        if !self.source.destinations[index]
            .waiting
            .swap(true, Ordering::AcqRel)
        {
            queue.waiters.push_back(index);
        }
        drop(queue);
        self.source.capacity_available();
    }
    pub(super) fn resume(&self) -> bool {
        if !self.selective() {
            return false;
        }
        let index = self.index;
        if !self.source.destinations[index]
            .waiting
            .swap(false, Ordering::AcqRel)
        {
            return false;
        }
        let mut queue = self
            .source
            .queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        queue.waiters.retain(|i| *i != index);
        queue.notified.remove(&index);
        queue.bypass.remove(&index);
        true
    }
    pub(crate) fn interrupted(&self) {
        if self.resume() {
            self.source.capacity_available();
        }
    }
}
