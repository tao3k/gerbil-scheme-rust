//! Explicit class-specific routing; Scheme retains all intra-VM scheduling.
use super::{
    charge,
    types::{
        Class, Destination, Ledger, RoutedSchemeTicket, SchemeRouteLoad, SchemeRoutingConfig, State,
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
    dispatch: InstanceDispatcher,
    config: SchemeRoutingConfig,
    state: Arc<State>,
}
impl std::fmt::Debug for SchemeServiceRouter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SchemeServiceRouter")
            .field("config", &self.config)
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
        validate(&dispatch, &config)?;
        let destinations = config
            .owners
            .iter()
            .map(|&owner| Destination {
                owner,
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
        let short_slots = config.short_slots;
        let batch_slots = config.batch_slots;
        Ok(Self {
            dispatch,
            config,
            state: Arc::new(State {
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
        for destination in &self.state.destinations {
            destination.short.close();
            destination.batch.close();
        }
    }
    /// Snapshot includes routing/admission wait through native projection.
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
    fn live_indices(&self) -> std::collections::BTreeSet<usize> {
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
            _charge: charge,
            _permit: permit,
        };
        let ticket = self
            .dispatch
            .submit_scheme_service_backpressured_on(
                owner,
                request,
                timeout.saturating_sub(started.elapsed()),
            )
            .await?;
        Ok(RoutedSchemeTicket { owner, ticket })
    }
}
fn validate(
    dispatch: &InstanceDispatcher,
    config: &SchemeRoutingConfig,
) -> Result<(), InstanceError> {
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
    let capacities: std::collections::BTreeMap<_, _> = dispatch
        .owner_capacity()
        .into_iter()
        .map(|capacity| (capacity.owner, capacity))
        .collect();
    let senders = dispatch
        .admission
        .senders
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut seen = std::collections::BTreeSet::new();
    for owner in &config.owners {
        let native = capacities.get(owner).ok_or_else(invalid)?;
        if senders
            .owners
            .get(*owner)
            .and_then(|o| o.wake.as_ref())
            .is_none()
        {
            return Err(invalid());
        }
        if !seen.insert(*owner)
            || slots > native.limit
            || native.work_limit.is_none_or(|limit| work > limit)
        {
            return Err(invalid());
        }
    }
    Ok(())
}
