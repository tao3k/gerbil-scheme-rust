//! Typed reservation state for FIFO and opt-in owner-local native supply.
use super::{
    InstanceError,
    capacity::{Capacity, CapacityPermit, acquire_pair},
};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
/// Explicit supply admission policy; no policy schedules Scheme execution.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SchemeServiceSupplyAdmission {
    /// Existing asynchronous FIFO reservation path.
    #[default]
    Queued,
    /// Experimental registered-owner supply with cached close admission.
    /// Keep acquisition state on the owner and active credits in the native
    /// retirement guard. Ready credits need no temporary future; genuine waits
    /// retain FIFO position. Scheme execution and native bounds are unchanged.
    OwnerLocal,
    /// Experimental bounded refill after the complete result snapshot is retired.
    /// Retains `OwnerLocal` bounds; short publication takes priority over bulk.
    RetirementRefill,
}
type Pair = (OwnedSemaphorePermit, CapacityPermit);
type Waiting = Pin<Box<dyn Future<Output = Result<Pair, InstanceError>> + Send>>;
struct Pending {
    units: usize,
    future: Waiting,
}
pub(super) struct Acquisition {
    mode: SchemeServiceSupplyAdmission,
    pending: Option<Pending>,
}
impl Acquisition {
    pub(super) fn new(mode: SchemeServiceSupplyAdmission) -> Self {
        Self {
            mode,
            pending: None,
        }
    }
    pub(super) fn owner_local(&self) -> bool {
        self.mode != SchemeServiceSupplyAdmission::Queued
    }
    pub(super) fn retirement_refill(&self) -> bool {
        self.mode == SchemeServiceSupplyAdmission::RetirementRefill
    }
    pub(super) fn clear(&mut self) {
        self.pending = None;
    }
    pub(super) fn poll(
        &mut self,
        active: &Arc<Semaphore>,
        capacity: &Arc<Capacity>,
        units: usize,
        context: &mut Context<'_>,
    ) -> Poll<Result<Pair, InstanceError>> {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.units != units)
        {
            self.clear();
        }
        if self.pending.is_none() {
            if self.mode != SchemeServiceSupplyAdmission::Queued {
                match ready(active, capacity, units) {
                    Ok(Some(pair)) => return Poll::Ready(Ok(pair)),
                    Ok(None) => {}
                    Err(error) => return Poll::Ready(Err(error)),
                }
            }
            let active = Arc::clone(active);
            let capacity = Arc::clone(capacity);
            self.pending = Some(Pending {
                units,
                future: Box::pin(async move {
                    let active = active
                        .acquire_owned()
                        .await
                        .map_err(|_| InstanceError::Closed)?;
                    let capacity = acquire_pair(capacity, units).await?;
                    Ok((active, capacity))
                }),
            });
        }
        let result = self.pending.as_mut().unwrap().future.as_mut().poll(context);
        if result.is_ready() {
            self.clear();
        }
        result
    }
}
fn ready(
    active: &Arc<Semaphore>,
    capacity: &Arc<Capacity>,
    units: usize,
) -> Result<Option<Pair>, InstanceError> {
    let active = match Arc::clone(active).try_acquire_owned() {
        Ok(permit) => permit,
        Err(tokio::sync::TryAcquireError::NoPermits) => return Ok(None),
        Err(tokio::sync::TryAcquireError::Closed) => return Err(InstanceError::Closed),
    };
    match capacity.try_pair(units) {
        Ok(capacity) => Ok(Some((active, capacity))),
        Err(InstanceError::QueueFull) => Ok(None),
        Err(error) => Err(error),
    }
}
#[cfg(test)]
#[path = "../../tests/unit/service_stream_acquire.rs"]
mod tests;
