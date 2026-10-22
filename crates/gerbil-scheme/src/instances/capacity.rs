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
    /// Optional declared work-unit bound; numeric batches charge one unit per item.
    pub work_limit: Option<usize>,
    /// Work credits include reservations waiting for a request slot.
    pub work_available: Option<usize>,
    /// One permits no registered owner-local producer; zero means one is live.
    pub producer_available: usize,
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
/// Independent request slots and declared work units for one fixed owner.
/// Work units are application declarations, not elapsed-time predictions.
#[derive(Debug)]
pub(super) struct Capacity {
    pub(super) changed: Arc<std::sync::OnceLock<crate::instances::capacity_event::CapacityEvents>>,
    pub slots: Arc<Semaphore>,
    pub slot_limit: usize,
    pub work: Option<Arc<Semaphore>>,
    pub work_limit: usize,
    pub producer: Arc<Semaphore>,
}
impl Capacity {
    pub fn close(&self) {
        self.slots.close();
        self.producer.close();
        if let Some(work) = &self.work {
            work.close();
        }
        self.signal();
    }
    fn signal(&self) {
        if let Some(changed) = self.changed.get() {
            changed.publish();
        }
    }
    pub(super) fn units(&self, units: usize) -> Result<u32, InstanceError> {
        let units = u32::try_from(units)
            .map_err(|_| InstanceError::InvalidConfig("work units overflow"))?;
        if units == 0 || (self.work.is_some() && units as usize > self.work_limit) {
            return Err(InstanceError::InvalidConfig(
                "work units exceed owner budget",
            ));
        }
        Ok(units)
    }
}
#[derive(Debug)]
pub(super) struct CapacityPermit {
    pub(super) capacity: Arc<Capacity>,
    // Partial immediate acquisition stays in Request until outside the admission lock.
    pub(super) slot: Option<OwnedSemaphorePermit>,
    pub(super) work: Option<OwnedSemaphorePermit>,
    pub(super) active: Option<OwnedSemaphorePermit>,
}
impl CapacityPermit {
    pub(super) fn retain_active(mut self, active: OwnedSemaphorePermit) -> Self {
        assert!(
            self.active.is_none(),
            "native active credit already attached"
        );
        self.active = Some(active);
        self
    }
}
impl Drop for CapacityPermit {
    fn drop(&mut self) {
        drop(self.active.take());
        drop(self.work.take());
        drop(self.slot.take());
        // A pending acquire_many future can hold partial work internally even
        // before work becomes Some. Its cancellation returns those credits too.
        self.capacity.signal();
    }
}
pub(super) struct ReservedOwner {
    pub owner: usize,
    pub permit: CapacityPermit,
    pub units: usize,
}
pub(super) fn attach(owner: &OwnerSender, request: &mut Request) -> Result<(), InstanceError> {
    match (&owner.capacity, &request.capacity) {
        (Some(capacity), Some(permit)) if Arc::ptr_eq(capacity, &permit.capacity) => Ok(()),
        (_, Some(_)) => Err(InstanceError::Closed),
        (None, None) => Ok(()),
        (Some(capacity), None) => {
            let units = capacity.units(request.work_units)?;
            let mut permit = CapacityPermit {
                capacity: Arc::clone(capacity),
                slot: None,
                work: None,
                active: None,
            };
            // Immediate APIs never wait or bypass FIFO work waiters.
            let result = (|| {
                if let Some(work) = &capacity.work {
                    permit.work = Some(
                        Arc::clone(work)
                            .try_acquire_many_owned(units)
                            .map_err(|error| immediate_error(&error))?,
                    );
                }
                permit.slot = Some(
                    Arc::clone(&capacity.slots)
                        .try_acquire_owned()
                        .map_err(|error| immediate_error(&error))?,
                );
                Ok(())
            })();
            request.capacity = Some(permit);
            result
        }
    }
}
pub(super) fn immediate_error(error: &tokio::sync::TryAcquireError) -> InstanceError {
    match error {
        tokio::sync::TryAcquireError::Closed => InstanceError::Closed,
        tokio::sync::TryAcquireError::NoPermits => InstanceError::QueueFull,
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
        self.configure_owner_budgets(maximum_in_flight, None)
    }
    /// Configure request slots independently from declared work credits.
    /// Numeric batch APIs charge their item count; generic/byte jobs charge one.
    /// Use the explicit weighted native API for application-specific work units.
    /// A unit bound cannot preempt Scheme or guarantee a wall-clock service time.
    /// # Errors
    /// Rejects zero/oversized bounds, active reservations and late first activation.
    pub fn configure_owner_budgets(
        &self,
        maximum_in_flight: usize,
        maximum_work: Option<usize>,
    ) -> Result<(), InstanceError> {
        if maximum_work
            .is_some_and(|n| n == 0 || n > u32::MAX as usize || n > Semaphore::MAX_PERMITS)
        {
            return Err(InstanceError::InvalidConfig("invalid owner work budget"));
        }
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
        if maximum_work.is_none()
            && senders.owners.iter().any(|o| {
                o.wake.as_ref().is_some_and(|w| {
                    w.progress
                        .enabled
                        .load(std::sync::atomic::Ordering::Acquire)
                })
            })
        {
            return Err(InstanceError::InvalidConfig(
                "native progress requires a bounded work ledger",
            ));
        }
        if self.admission.short_pool.get().is_some() {
            return Err(InstanceError::InvalidConfig(
                "short pool freezes native budgets",
            ));
        }
        let previous = configure(
            &mut senders,
            maximum_in_flight,
            maximum_work,
            &self.admission.capacity_changed,
        )?;
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
        units: usize,
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
        let acquire = acquire_pair(capacity, units);
        let permit = tokio::time::timeout(timeout, acquire)
            .await
            .map_err(|_| InstanceError::Timeout)??;
        if started.elapsed() >= timeout {
            drop(permit);
            return Err(InstanceError::Timeout);
        }
        Ok(ReservedOwner {
            owner,
            permit,
            units,
        })
    }
}

/// Semaphore-only acquisition: safe to poll on a native owner without entering Tokio.
pub(super) async fn acquire_pair(
    capacity: Arc<Capacity>,
    units: usize,
) -> Result<CapacityPermit, InstanceError> {
    let units = capacity.units(units)?;
    let mut permit = CapacityPermit {
        capacity: Arc::clone(&capacity),
        slot: None,
        work: None,
        active: None,
    };
    if let Some(work) = &capacity.work {
        permit.work = Some(
            Arc::clone(work)
                .acquire_many_owned(units)
                .await
                .map_err(|_| InstanceError::Closed)?,
        );
    }
    permit.slot = Some(
        Arc::clone(&capacity.slots)
            .acquire_owned()
            .await
            .map_err(|_| InstanceError::Closed)?,
    );
    Ok(permit)
}

fn configure(
    senders: &mut super::dispatch::Senders,
    maximum_in_flight: usize,
    maximum_work: Option<usize>,
    changed: &Arc<std::sync::OnceLock<crate::instances::capacity_event::CapacityEvents>>,
) -> Result<Vec<Arc<Capacity>>, InstanceError> {
    if senders.owners.is_empty() {
        return Err(InstanceError::Closed);
    }
    if senders.ever_admitted && senders.owners.iter().any(|owner| owner.capacity.is_none()) {
        return Err(InstanceError::InvalidConfig(
            "enable capacity before unbounded admission",
        ));
    }
    if senders.owners.iter().any(|owner| {
        owner.capacity.as_ref().is_some_and(|capacity| {
            capacity.producer.available_permits() != 1
                || capacity.slots.available_permits() != owner.capacity_limit
                || capacity
                    .work
                    .as_ref()
                    .is_some_and(|work| work.available_permits() != capacity.work_limit)
        })
    }) {
        return Err(InstanceError::InvalidConfig(
            "owner capacity has outstanding reservations",
        ));
    }
    let mut previous = Vec::new();
    for owner in &mut senders.owners {
        previous.extend(owner.capacity.replace(Arc::new(Capacity {
            changed: Arc::clone(changed),
            slots: Arc::new(Semaphore::new(maximum_in_flight)),
            slot_limit: maximum_in_flight,
            work: maximum_work.map(|n| Arc::new(Semaphore::new(n))),
            work_limit: maximum_work.unwrap_or(0),
            producer: Arc::new(Semaphore::new(1)),
        })));
        owner.capacity_limit = maximum_in_flight;
    }
    Ok(previous)
}
fn retire(previous: Vec<Arc<Capacity>>) {
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
                    available: capacity.slots.available_permits(),
                    work_limit: capacity.work.as_ref().map(|_| capacity.work_limit),
                    work_available: capacity.work.as_ref().map(|work| work.available_permits()),
                    producer_available: capacity.producer.available_permits(),
                })
        })
        .collect()
}
