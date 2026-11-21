//! Fairness bounds concern eligible wake opportunities, never OS time slices.
use super::{Queue, Source};
use crate::instances::service_wake::Wake;
use std::sync::{Weak, atomic::Ordering};
impl Source {
    pub(super) fn select_targets(&self, queue: &mut Queue, available: usize) -> Vec<Weak<Wake>> {
        let mut eligible: Vec<_> = queue
            .waiters
            .iter()
            .copied()
            .filter(|&index| {
                let t = &self.destinations[index];
                !t.capacity.slots.is_closed()
                    && t.active.available_permits() > 0
                    && t.capacity.slots.available_permits() > 0
                    && t.capacity
                        .work
                        .as_ref()
                        .is_none_or(|w| !w.is_closed() && w.available_permits() > 0)
            })
            .collect();
        let mut targets = Vec::new();
        for _ in 0..available.min(eligible.len()) {
            let position = if self.progress.load(Ordering::Acquire) {
                // Selection evaluates priority, not membership. Work is bounded
                // by configured destinations times newly owed wake duties.
                eligible
                    .iter()
                    .enumerate()
                    .min_by_key(|(position, index)| {
                        let overdue = queue.bypass.get(index).copied().unwrap_or(0) >= 2;
                        let remaining = if overdue {
                            0
                        } else {
                            self.destinations[**index]
                                .wake
                                .upgrade()
                                .map_or(u64::MAX, |w| w.progress.load())
                        };
                        (!overdue, remaining, *position)
                    })
                    .unwrap()
                    .0
            } else {
                0
            };
            let index = eligible.remove(position);
            if self.progress.load(Ordering::Acquire) {
                for peer in &eligible {
                    let debt = queue.bypass.entry(*peer).or_default();
                    *debt = debt.saturating_add(1).min(2);
                }
            }
            queue.bypass.remove(&index);
            queue.notified.insert(index);
            targets.push(self.destinations[index].wake.clone());
        }
        queue.waiters.retain(|i| !queue.notified.contains(i));
        targets
    }
}
