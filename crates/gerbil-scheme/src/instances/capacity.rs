//! Opt-in owner capacity shared by every dispatcher clone and native API.
use super::{InstanceDispatcher, InstanceError, Request, dispatch::OwnerSender};
use std::{sync::Arc, time::Duration};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
/// One owner capacity observation, including reservations not yet enqueued.
#[derive(Clone, Debug)]
pub struct OwnerCapacitySnapshot {
    /// Fixed native owner index.
    pub owner: usize,
    /// Configured accepted/reserved request bound for this owner.
    pub limit: usize,
    /// Currently unreserved request credits.
    pub available: usize,
}
/// Explicit native Scheme batch routing and finite capacity-wait budget.
#[derive(Clone, Copy, Debug)]
pub struct BatchAdmission {
    /// Fixed owner; Scheme values never migrate.
    pub owner: usize,
    /// Existing Scheme Actor cooperative item quantum.
    pub yield_every: usize,
    /// Capacity observation budget; timeout before admission executes no Scheme.
    pub timeout: Duration,
}
pub(super) struct ReservedOwner {
    pub owner: usize,
    pub permit: OwnedSemaphorePermit,
}
pub(super) fn attach(owner: &OwnerSender, request: &mut Request) -> Result<(), InstanceError> {
    match (&owner.capacity, &request.capacity) {
        (Some(capacity), Some(permit)) if Arc::ptr_eq(capacity, permit.semaphore()) => Ok(()),
        (_, Some(_)) => Err(InstanceError::Closed),
        (None, None) => Ok(()),
        (Some(capacity), None) => {
            request.capacity =
                Some(
                    Arc::clone(capacity)
                        .try_acquire_owned()
                        .map_err(|error| match error {
                            tokio::sync::TryAcquireError::Closed => InstanceError::Closed,
                            tokio::sync::TryAcquireError::NoPermits => InstanceError::QueueFull,
                        })?,
                );
            Ok(())
        }
    }
}
impl InstanceDispatcher {
    /// Enable shared per-owner capacity before the first unbounded admission.
    /// Reconfiguration requires all existing credits to be available. Old waiting
    /// reservations are closed and cannot cross the admission/close fence.
    /// Every API and clone participates once enabled. No default is changed.
    /// # Errors
    /// Rejects closed pools, bounds above the native queue size, active capacity
    /// and first activation after unbounded work has already been admitted.
    pub fn configure_owner_capacity(&self, maximum_in_flight: usize) -> Result<(), InstanceError> {
        if maximum_in_flight == 0 || maximum_in_flight > self.admission.queue_capacity {
            return Err(InstanceError::InvalidConfig(
                "owner capacity must fit the native queue",
            ));
        }
        let mut senders = self
            .admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = configure(&mut senders, maximum_in_flight)?;
        drop(senders);
        retire(previous);
        Ok(())
    }
    /// Observe shared owner credits without entering Scheme.
    #[must_use]
    pub fn owner_capacity(&self) -> Vec<OwnerCapacitySnapshot> {
        let senders = self
            .admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        snapshot(&senders)
    }
    pub(super) async fn reserve_owner(
        &self,
        owner: usize,
        timeout: Duration,
    ) -> Result<ReservedOwner, InstanceError> {
        let started = std::time::Instant::now();
        if timeout.is_zero() {
            return Err(InstanceError::Timeout);
        }
        let capacity = {
            let senders = self
                .admission
                .senders
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let sender = senders.owners.get(owner).ok_or(InstanceError::Closed)?;
            Arc::clone(
                sender
                    .capacity
                    .as_ref()
                    .ok_or(InstanceError::InvalidConfig(
                        "shared owner capacity is not enabled",
                    ))?,
            )
        };
        let permit = tokio::time::timeout(timeout, capacity.acquire_owned())
            .await
            .map_err(|_| InstanceError::Timeout)?
            .map_err(|_| InstanceError::Closed)?;
        if started.elapsed() >= timeout {
            drop(permit);
            return Err(InstanceError::Timeout);
        }
        Ok(ReservedOwner { owner, permit })
    }
}

fn configure(
    senders: &mut super::dispatch::Senders,
    maximum_in_flight: usize,
) -> Result<Vec<Arc<Semaphore>>, InstanceError> {
    if senders.owners.is_empty() {
        return Err(InstanceError::Closed);
    }
    if senders.ever_admitted && senders.owners.iter().any(|owner| owner.capacity.is_none()) {
        return Err(InstanceError::InvalidConfig(
            "enable capacity before unbounded admission",
        ));
    }
    if senders.owners.iter().any(|owner| {
        owner
            .capacity
            .as_ref()
            .is_some_and(|capacity| capacity.available_permits() != owner.capacity_limit)
    }) {
        return Err(InstanceError::InvalidConfig(
            "owner capacity has outstanding reservations",
        ));
    }
    let mut previous = Vec::new();
    for owner in &mut senders.owners {
        previous.extend(
            owner
                .capacity
                .replace(Arc::new(Semaphore::new(maximum_in_flight))),
        );
        owner.capacity_limit = maximum_in_flight;
    }
    Ok(previous)
}
fn retire(previous: Vec<Arc<Semaphore>>) {
    for capacity in previous {
        capacity.close();
    }
}
fn snapshot(senders: &super::dispatch::Senders) -> Vec<OwnerCapacitySnapshot> {
    senders
        .owners
        .iter()
        .enumerate()
        .filter_map(|(owner, sender)| {
            sender
                .capacity
                .as_ref()
                .map(|capacity| OwnerCapacitySnapshot {
                    owner,
                    limit: sender.capacity_limit,
                    available: capacity.available_permits(),
                })
        })
        .collect()
}
