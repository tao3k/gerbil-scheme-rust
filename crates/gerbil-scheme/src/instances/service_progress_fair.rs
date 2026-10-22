//! Route actual bounded work; a wake without a committed request grants no turn.
use super::{Capacity, service_wake::Wake};
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::Semaphore;
#[derive(Default)]
pub(crate) struct Fair(Mutex<Vec<Peer>>);
struct Peer {
    owner: usize,
    wake: Weak<Wake>,
    capacity: Arc<Capacity>,
    active: Weak<Semaphore>,
    staged: Arc<AtomicU64>,
}
pub(super) struct RouteTarget {
    pub owner: usize,
    pub wake: Arc<Wake>,
    pub capacity: Arc<Capacity>,
    pub active: Arc<Semaphore>,
    pub staged: Arc<AtomicU64>,
}
/// An inbox hint, never release authority for active/work/slot/root ownership.
#[derive(Debug)]
pub(super) struct StagedWork {
    counter: Arc<AtomicU64>,
    units: u64,
}
impl StagedWork {
    pub(super) fn units(&self) -> usize {
        usize::try_from(self.units).unwrap()
    }
    pub(super) fn new(counter: Arc<AtomicU64>, units: usize) -> Self {
        let units = u64::try_from(units).unwrap();
        counter.fetch_add(units, Ordering::AcqRel);
        Self { counter, units }
    }
}
impl Drop for StagedWork {
    fn drop(&mut self) {
        self.counter.fetch_sub(self.units, Ordering::AcqRel);
    }
}
impl Fair {
    pub(super) fn register(
        &self,
        owner: usize,
        wake: Weak<Wake>,
        capacity: Arc<Capacity>,
        active: Weak<Semaphore>,
    ) {
        let staged = wake
            .upgrade()
            .map_or_else(Arc::default, |w| Arc::clone(&w.progress.queued));
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(Peer {
                owner,
                wake,
                capacity,
                active,
                staged,
            });
    }
    pub(super) fn target(&self, owner: usize, units: usize) -> Option<RouteTarget> {
        let peers = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let own = peers.iter().find(|p| p.owner == owner)?;
        let own_work = own.wake.upgrade()?.progress.load();
        let (peer, wake, active, work) = peers
            .iter()
            .filter(|p| p.owner != owner)
            .filter_map(|p| {
                let wake = p.wake.upgrade()?;
                let active = p.active.upgrade()?;
                let work = wake.progress.load();
                (eligible(p, &wake, &active, units)).then_some((p, wake, active, work))
            })
            .min_by_key(|(_, _, _, work)| *work)?;
        if own_work <= work.saturating_add(units as u64) {
            return None;
        }
        Some(RouteTarget {
            owner: peer.owner,
            wake,
            active,
            capacity: Arc::clone(&peer.capacity),
            staged: Arc::clone(&peer.staged),
        })
    }
}
fn eligible(peer: &Peer, wake: &Wake, active: &Semaphore, units: usize) -> bool {
    !wake.closed.load(Ordering::Acquire)
        && !wake.failed.load(Ordering::Acquire)
        && active.available_permits() > 0
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
