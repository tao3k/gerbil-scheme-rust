//! Bounded nonexclusive claim opportunities; never hold credit for a peer.
use super::{Capacity, service_wake::Wake};
use std::sync::{Arc, Mutex, Weak, atomic::Ordering};
use tokio::sync::Semaphore;
#[derive(Default)]
pub(crate) struct Fair(Mutex<Vec<Peer>>);
struct Peer {
    owner: usize,
    wake: Weak<Wake>,
    capacity: Arc<Capacity>,
    active: Weak<Semaphore>,
    yielded: bool,
}
impl Fair {
    pub(super) fn register(
        &self,
        owner: usize,
        wake: Weak<Wake>,
        capacity: Arc<Capacity>,
        active: Weak<Semaphore>,
    ) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(Peer {
                owner,
                wake,
                capacity,
                active,
                yielded: false,
            });
    }
    pub(super) fn allow(&self, owner: usize, units: usize) -> bool {
        let target = {
            let mut peers = self
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Some(index) = peers.iter().position(|p| p.owner == owner) else {
                return true;
            };
            if peers[index].yielded {
                peers[index].yielded = false;
                return true;
            }
            let Some(own) = peers[index].wake.upgrade() else {
                return true;
            };
            let least = peers
                .iter()
                .filter(|p| p.owner != owner && eligible(p, units))
                .filter_map(|p| p.wake.upgrade().map(|w| (w.progress.load(), w)))
                .min_by_key(|(pending, _)| *pending);
            let Some((remaining, wake)) = least else {
                return true;
            };
            // One unit-priced head of slack avoids lockstep assignment. A peer
            // receives only one opportunity: the next attempt remains runnable
            // even when that peer is delayed, cancelled or descheduled.
            if own.progress.load() <= remaining.saturating_add(units as u64) {
                return true;
            }
            peers[index].yielded = true;
            wake
        };
        target.notify();
        false
    }
}
fn eligible(peer: &Peer, units: usize) -> bool {
    peer.wake.upgrade().is_some_and(|w| {
        !w.closed.load(Ordering::Acquire)
            && !w.failed.load(Ordering::Acquire)
            && w.progress.waiting_for_claim.load(Ordering::Acquire)
    }) && peer
        .active
        .upgrade()
        .is_some_and(|a| a.available_permits() > 0)
        && peer.capacity.slots.available_permits() > 0
        && peer
            .capacity
            .work
            .as_ref()
            .is_some_and(|w| w.available_permits() >= units)
}
#[cfg(test)]
#[path = "../../tests/unit/service_progress_fair.rs"]
mod tests;
