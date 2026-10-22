//! Bounded cross-owner admission; no foreign code or user callback under locks.
use super::{CapacityPermit, PoolSource, RouteTarget, StagedWork, pool::Work};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

pub(super) fn assist(
    source: &PoolSource,
    owner: usize,
    units: usize,
    accepting: &AtomicBool,
    cancelled: &AtomicBool,
    deadline: Instant,
) -> bool {
    let Some(target) = source.fair.get().and_then(|f| f.target(owner, units)) else {
        return false;
    };
    let Some(permit) = reserve(&target, units) else {
        return false;
    };
    let mut permit = Some(permit);
    let sent = deliver(source, &target, accepting, &mut permit, cancelled, deadline);
    // Rollback returns capacity outside both the source and admission fences.
    drop(permit);
    sent
}
fn reserve(target: &RouteTarget, units: usize) -> Option<CapacityPermit> {
    let active = Arc::clone(&target.active).try_acquire_owned().ok()?;
    let mut permit = target.capacity.try_pair(units).ok()?.retain_active(active);
    permit.staged = Some(StagedWork::new(Arc::clone(&target.staged), units));
    Some(permit)
}
fn deliver(
    source: &PoolSource,
    target: &RouteTarget,
    accepting: &AtomicBool,
    permit: &mut Option<CapacityPermit>,
    cancelled: &AtomicBool,
    deadline: Instant,
) -> bool {
    // This capability was issued under the registration/close fence. A weak
    // mailbox never keeps an old owner inbox alive and cannot target a new one.
    let Some(mailbox) = target.mailbox.upgrade() else {
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
    let units = permit.as_ref().unwrap().staged.as_ref().unwrap().units();
    if requests.front().is_none_or(|r| r.work_units != units) {
        return false;
    }
    let mut request = requests.pop_front().unwrap();
    request.capacity = permit.take();
    let Work::Service(job) = &request.work else {
        unreachable!("typed pooled service");
    };
    let trace = job.entry_trace().map(Arc::clone);
    let short = job.short_service();
    if let Some(trace) = &trace {
        trace.mark(super::SchemeServiceEntryPhase::Published);
    }
    let result = mailbox.try_send(request);
    let sent = match result {
        Ok(()) => {
            target
                .wake
                .progress
                .peer_deliveries
                .fetch_add(1, Ordering::AcqRel);
            true
        }
        Err(
            std::sync::mpsc::TrySendError::Full(mut request)
            | std::sync::mpsc::TrySendError::Disconnected(mut request),
        ) => {
            *permit = request.capacity.take();
            requests.push_front(request);
            false
        }
    };
    let head_changed = sent && requests.front().map(|r| r.work_units) != Some(units);
    drop(requests);
    if head_changed {
        source.wake();
    }
    // The request and all three native budgets are committed before notification.
    super::service_entry::notify(Some(Arc::clone(&target.wake)), sent, trace, short);
    sent
}
#[cfg(test)]
#[path = "../../tests/unit/service_progress_route.rs"]
mod tests;
