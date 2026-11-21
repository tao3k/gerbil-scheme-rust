//! Bounded cross-owner admission; no foreign code or user callback under locks.
use super::{PoolSource, RouteTarget, pool::Work};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

pub(super) fn assist(
    source: &Arc<PoolSource>,
    owner: usize,
    units: usize,
    accepting: &Arc<AtomicBool>,
    cancelled: &Arc<AtomicBool>,
    deadline: Instant,
) -> bool {
    let Some(target) = source.fair.get().and_then(|f| f.target(owner, units)) else {
        return false;
    };
    deliver(source, &target, units, accepting, cancelled, deadline)
}
fn deliver(
    source: &Arc<PoolSource>,
    target: &RouteTarget,
    units: usize,
    accepting: &Arc<AtomicBool>,
    cancelled: &Arc<AtomicBool>,
    deadline: Instant,
) -> bool {
    // This capability was issued under the registration/close fence. A weak
    // mailbox never keeps an old owner inbox alive and cannot target a new one.
    let Some(_mailbox) = target.mailbox.upgrade() else {
        return false;
    };
    if !crate::instance_backend::images_ready()
        || !accepting.load(Ordering::Acquire)
        || target.wake.closed.load(Ordering::Acquire)
        || target.wake.failed.load(Ordering::Acquire)
        || cancelled.load(Ordering::Acquire)
        || Instant::now() >= deadline
    {
        return false;
    }
    let mut requests = source
        .requests
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if target.wake.offers.pending() {
        return false;
    }
    if requests.front().is_none_or(|r| r.work_units != units) {
        return false;
    }
    let request = requests.pop_front().unwrap();
    let Work::Service(job) = &request.work else {
        unreachable!("typed pooled service");
    };
    let trace = job.entry_trace().map(Arc::clone);
    let short = job.short_service();
    if let Some(trace) = &trace {
        trace.mark(super::SchemeServiceEntryPhase::Published);
    }
    let offer = super::Offer::new(
        request,
        source,
        target,
        Arc::clone(accepting),
        Arc::clone(cancelled),
        deadline,
    );
    let sent = target.wake.offers.publish(Arc::clone(&offer), &target.wake);
    if sent {
        source
            .offers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push_back(offer);
        target
            .wake
            .progress
            .peer_deliveries
            .fetch_add(1, Ordering::AcqRel);
    } else {
        requests.push_front(offer.withdraw().unwrap());
    }
    drop(requests);
    // A source request moved into an offer without changing source work/price.
    // Only the target slot's empty-to-full transition needs a notification.
    super::service_entry::notify(Some(Arc::clone(&target.wake)), sent, trace, short);
    sent
}
#[cfg(test)]
#[path = "../../tests/unit/service_progress_route.rs"]
mod tests;
