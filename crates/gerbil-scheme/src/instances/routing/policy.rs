//! Explicit class-specific routing; Scheme retains all intra-VM scheduling.
use super::{
    charge,
    types::{
        Class, Destination, Ledger, RoutedSchemeTicket, SchemeRouteLoad, SchemeRoutingAdmission,
        SchemeRoutingConfig, State,
    },
};
use crate::instances::{InstanceDispatcher, InstanceError, SchemeServiceRequest};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;
/// Stateless cross-instance router with distinct short and batch algorithms.
/// Stateful sessions must retain their explicit owner/lane through the existing
/// dispatcher API; this router never migrates Scheme state or continuations.
#[derive(Clone)]
pub struct SchemeServiceRouter {
    pub(super) dispatch: InstanceDispatcher,
    config: SchemeRoutingConfig,
    pub(super) state: Arc<State>,
    pub(super) admission: SchemeRoutingAdmission,
}
impl std::fmt::Debug for SchemeServiceRouter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SchemeServiceRouter")
            .field("config", &self.config)
            .field("admission", &self.admission)
            .finish_non_exhaustive()
    }
}
impl SchemeServiceRouter {
    /// Construct explicit per-class quotas under configured native budgets.
    /// # Errors
    /// Empty/duplicate/unknown owners, absent budgets or incompatible bounds.
    pub fn new(
        dispatch: InstanceDispatcher,
        config: SchemeRoutingConfig,
    ) -> Result<Self, InstanceError> {
        Self::with_admission(dispatch, config, SchemeRoutingAdmission::Pinned)
    }
    /// Explicit stateless admission strategy; no defaults or native scheduling change.
    /// # Errors
    /// Same owner and quota validation as `new`.
    pub fn with_admission(
        dispatch: InstanceDispatcher,
        config: SchemeRoutingConfig,
        admission: SchemeRoutingAdmission,
    ) -> Result<Self, InstanceError> {
        let capacities = validate(&dispatch, &config)?;
        if admission != SchemeRoutingAdmission::Pinned {
            dispatch
                .admission
                .capacity_changed
                .get_or_init(crate::instances::capacity_event::CapacityEvents::new);
        }
        let destinations = config
            .owners
            .iter()
            .zip(capacities)
            .map(|(&owner, capacity)| Destination {
                owner,
                capacity,
                short: Arc::new(Semaphore::new(config.short_slots)),
                batch: Arc::new(Semaphore::new(config.batch_slots)),
            })
            .collect();
        let loads = config
            .owners
            .iter()
            .map(|&owner| SchemeRouteLoad {
                owner,
                ..SchemeRouteLoad::default()
            })
            .collect();
        let changed = Arc::clone(&dispatch.admission.capacity_changed);
        let short_slots = config.short_slots;
        let batch_slots = config.batch_slots;
        Ok(Self {
            dispatch,
            config,
            admission,
            state: Arc::new(State {
                changed: Arc::clone(&changed),
                watch: if matches!(
                    admission,
                    SchemeRoutingAdmission::Reselect | SchemeRoutingAdmission::ReselectProgress
                ) {
                    Some(dispatch_events(&changed))
                } else {
                    None
                },
                producer_progress: matches!(
                    admission,
                    SchemeRoutingAdmission::ReselectProgress
                        | SchemeRoutingAdmission::ReselectProgressNotified
                ),
                short_gate: Arc::new(Semaphore::new(1)),
                batch_gate: Arc::new(Semaphore::new(1)),
                destinations,
                short_slots,
                batch_slots,
                ledger: Mutex::new(Ledger {
                    loads,
                    short_cursor: 0,
                    batch_cursor: 0,
                }),
            }),
        })
    }
    /// Close class admission and wake class waiters. Already-granted class
    /// permits may complete native admission and execution. Drain their tickets,
    /// then shut down the native pool through its existing close protocol.
    pub fn close(&self) {
        self.state.short_gate.close();
        self.state.batch_gate.close();
        for destination in &self.state.destinations {
            destination.short.close();
            destination.batch.close();
        }
        if let Some(changed) = self.dispatch.admission.capacity_changed.get() {
            changed.publish();
        }
    }
    /// Snapshot spans owner reservation through projection. Pinned additionally
    /// charges owner-specific admission waits; unassigned reselection waiters do not.
    #[must_use]
    pub fn loads(&self) -> Vec<SchemeRouteLoad> {
        self.state
            .ledger
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .loads
            .clone()
    }
    /// Short routing minimizes pending short requests, then total request count,
    /// then declared batch work. Rotating ties avoid a permanently favored VM.
    /// # Errors
    /// Invalid declaration/lane, closed owner or finite admission deadline.
    pub async fn submit_short<M: SchemeServiceRequest>(
        &self,
        message: M,
        timeout: Duration,
    ) -> Result<RoutedSchemeTicket<M::Output>, InstanceError> {
        self.submit(Class::Short, message, timeout).await
    }
    /// Batch routing minimizes declared outstanding batch work, then request
    /// count. It cannot consume this router's reserved short callback quota.
    /// # Errors
    /// Invalid declaration/lane, closed owner or finite admission deadline.
    pub async fn submit_batch<M: SchemeServiceRequest>(
        &self,
        message: M,
        timeout: Duration,
    ) -> Result<RoutedSchemeTicket<M::Output>, InstanceError> {
        self.submit(Class::Batch, message, timeout).await
    }
    pub(super) fn live_indices(&self) -> std::collections::BTreeSet<usize> {
        let senders = self
            .dispatch
            .admission
            .senders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.state
            .destinations
            .iter()
            .enumerate()
            .filter_map(|(index, destination)| {
                senders
                    .owners
                    .get(destination.owner)
                    .filter(|owner| {
                        owner
                            .capacity
                            .as_ref()
                            .is_some_and(|capacity| Arc::ptr_eq(capacity, &destination.capacity))
                    })
                    .and_then(|owner| owner.wake.as_ref())
                    .filter(|wake| {
                        !wake.closed.load(std::sync::atomic::Ordering::Acquire)
                            && !wake.failed.load(std::sync::atomic::Ordering::Acquire)
                    })
                    .map(|_| index)
            })
            .collect()
    }
    async fn submit<M: SchemeServiceRequest>(
        &self,
        class: Class,
        message: M,
        timeout: Duration,
    ) -> Result<RoutedSchemeTicket<M::Output>, InstanceError> {
        let started = Instant::now();
        let message = crate::instances::service_contract::Checked::new(message)?;
        let execution_class = message.execution;
        let trace = message.entry_trace();
        if let Some(trace) = &trace {
            trace.mark(crate::instances::SchemeServiceEntryPhase::Route);
        }
        if timeout.is_zero() {
            return Err(InstanceError::Timeout);
        }
        if message.lane().is_some() {
            return Err(InstanceError::InvalidConfig(
                "stateful Scheme lane requires explicit owner",
            ));
        }
        let units = message.work_units();
        let bound = match class {
            Class::Short => self.config.short_max_work,
            Class::Batch => self.config.batch_max_work,
        };
        if units == 0 || units > bound {
            return Err(InstanceError::InvalidConfig("routing class work bound"));
        }
        if self.admission != SchemeRoutingAdmission::Pinned {
            return self
                .submit_reselected(class, message, timeout.saturating_sub(started.elapsed()))
                .await;
        }
        let eligible = self.live_indices();
        let charge = charge::Charge::select(Arc::clone(&self.state), class, units, &eligible)?;
        let destination = &self.state.destinations[charge.index];
        let owner = destination.owner;
        let semaphore = match class {
            Class::Short => &destination.short,
            Class::Batch => &destination.batch,
        };
        let permit = tokio::time::timeout(timeout, Arc::clone(semaphore).acquire_owned())
            .await
            .map_err(|_| InstanceError::Timeout)?
            .map_err(|_| InstanceError::Closed)?;
        if let Some(trace) = &trace {
            trace.mark(crate::instances::SchemeServiceEntryPhase::ClassReady);
        }
        let request = charge::RoutedRequest {
            message,
            units,
            _permit: permit,
            _charge: charge,
        };
        let ticket = self
            .dispatch
            .submit_scheme_service_backpressured_on(
                owner,
                request,
                timeout.saturating_sub(started.elapsed()),
            )
            .await?;
        Ok(RoutedSchemeTicket {
            owner,
            ticket,
            route_class: class,
            execution_class,
        })
    }
}
fn validate(
    dispatch: &InstanceDispatcher,
    config: &SchemeRoutingConfig,
) -> Result<Vec<Arc<crate::instances::capacity::Capacity>>, InstanceError> {
    let invalid = || InstanceError::InvalidConfig("Scheme routing quota configuration");
    if config.owners.is_empty()
        || config.short_slots == 0
        || config.batch_slots == 0
        || config.short_max_work == 0
        || config.batch_max_work == 0
    {
        return Err(invalid());
    }
    let slots = config
        .short_slots
        .checked_add(config.batch_slots)
        .ok_or_else(invalid)?;
    if slots > 65536 {
        return Err(invalid());
    }
    let work = config
        .short_slots
        .checked_mul(config.short_max_work)
        .and_then(|short| {
            config
                .batch_slots
                .checked_mul(config.batch_max_work)
                .and_then(|batch| short.checked_add(batch))
        })
        .ok_or_else(invalid)?;
    let senders = dispatch
        .admission
        .senders
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut capacities = Vec::with_capacity(config.owners.len());
    let mut seen = std::collections::BTreeSet::new();
    for owner in &config.owners {
        let target = senders.owners.get(*owner).ok_or_else(invalid)?;
        let native = target.capacity.as_ref().ok_or_else(invalid)?;
        if target.wake.is_none()
            || !seen.insert(*owner)
            || slots > target.capacity_limit
            || native.work.is_none()
            || work > native.work_limit
        {
            return Err(invalid());
        }
        capacities.push(Arc::clone(native));
    }
    Ok(capacities)
}

fn dispatch_events(
    changed: &std::sync::OnceLock<crate::instances::capacity_event::CapacityEvents>,
) -> tokio::sync::watch::Receiver<()> {
    changed
        .get()
        .expect("reselection events initialized")
        .subscribe()
}
