//! Claim and bind only while executing on the destination owner.
use super::{Port, Queued, reject};
use crate::instances::{InstanceError, Request, SchemeServiceEntryPhase, capacity::CapacityPermit};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Instant,
};
impl Port {
    pub(crate) fn claim(&self) -> Option<Request> {
        if self.source.length.load(Ordering::Acquire) == 0 {
            return None;
        }
        let target = self
            .source
            .destinations
            .iter()
            .find(|t| t.owner == self.owner)?;
        let active = Arc::clone(&target.active).try_acquire_owned().ok()?;
        let permit = target.capacity.try_pair(1).ok()?.retain_active(active);
        let admission = self.source.admission.upgrade()?;
        let senders = admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let live = admission.accepting.load(Ordering::Acquire)
            && senders.owners.get(self.owner).is_some_and(|o| {
                o.capacity
                    .as_ref()
                    .is_some_and(|c| Arc::ptr_eq(c, &target.capacity))
                    && o.wake.as_ref().is_some_and(|w| {
                        !w.closed.load(Ordering::Acquire) && !w.failed.load(Ordering::Acquire)
                    })
            });
        if !live {
            drop(senders);
            return None;
        }
        let mut queue = self
            .source
            .queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut expired = Vec::new();
        while queue
            .front()
            .is_some_and(|q| q.deadline <= Instant::now() || q.assigned.is_closed())
        {
            expired.push(queue.pop_front().unwrap());
        }
        let queued = queue.pop_front();
        self.source.length.store(queue.len(), Ordering::Release);
        drop(queue);
        drop(senders);
        for queued in expired {
            reject(queued, InstanceError::Timeout);
        }
        queued.map(|queued| claimed(queued, permit, self.owner))
    }
}
fn claimed(mut queued: Queued, permit: CapacityPermit, owner: usize) -> Request {
    queued.request.capacity = Some(permit);
    if let crate::instances::pool::Work::Service(job) = &queued.request.work {
        if let Some(trace) = job.entry_trace() {
            trace.mark(SchemeServiceEntryPhase::ClassReady);
            trace.mark(SchemeServiceEntryPhase::CapacityReady);
            trace.mark(SchemeServiceEntryPhase::Published);
            trace.mark(SchemeServiceEntryPhase::Dequeued);
        }
    }
    // Send/wake outside all locks. A concurrently cancelled receiver cannot
    // retract this committed request: the owner still drains it normally.
    let _ = queued.assigned.send(Ok(owner));
    queued.request
}
