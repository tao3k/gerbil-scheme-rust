//! Reservation guard spans host queueing through native projection/failure.
use super::types::{Class, SchemeRouteLoad, State};
use crate::instances::{InstanceError, SchemeServiceRequest};
use crate::instances::{NativeRoot, NativeSession};
use std::sync::Arc;
pub(super) struct Charge {
    state: Arc<State>,
    pub index: usize,
    class: Class,
    units: usize,
}
impl Charge {
    pub(super) fn select(
        state: Arc<State>,
        class: Class,
        units: usize,
        eligible: &std::collections::BTreeSet<usize>,
    ) -> Result<Self, InstanceError> {
        let mut ledger = state
            .ledger
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cursor = match class {
            Class::Short => ledger.short_cursor,
            Class::Batch => ledger.batch_cursor,
        };
        let count = ledger.loads.len();
        let index = (0..count)
            .map(|offset| (cursor + offset) % count)
            .filter(|index| eligible.contains(index))
            .min_by_key(|&index| {
                let load = ledger.loads[index];
                match class {
                    Class::Short => (
                        usize::from(load.short_requests >= state.short_slots),
                        load.short_requests,
                        load.batch_requests.saturating_add(load.short_requests),
                        load.batch_work,
                    ),
                    Class::Batch => (
                        usize::from(load.batch_requests >= state.batch_slots),
                        load.batch_work,
                        load.batch_requests,
                        load.short_requests,
                    ),
                }
            })
            .ok_or(InstanceError::Closed)?;
        let load = &mut ledger.loads[index];
        let overflow = || InstanceError::InvalidConfig("routing load overflow");
        match class {
            Class::Short => {
                load.short_requests = load.short_requests.checked_add(1).ok_or_else(overflow)?;
            }
            Class::Batch => {
                let requests = load.batch_requests.checked_add(1).ok_or_else(overflow)?;
                let work = load.batch_work.checked_add(units).ok_or_else(overflow)?;
                load.batch_requests = requests;
                load.batch_work = work;
            }
        }
        match class {
            Class::Short => ledger.short_cursor = (index + 1) % count,
            Class::Batch => ledger.batch_cursor = (index + 1) % count,
        }
        drop(ledger);
        Ok(Self {
            state,
            index,
            class,
            units,
        })
    }
}
impl Drop for Charge {
    fn drop(&mut self) {
        let mut ledger = self
            .state
            .ledger
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let load: &mut SchemeRouteLoad = &mut ledger.loads[self.index];
        match self.class {
            Class::Short => load.short_requests -= 1,
            Class::Batch => {
                load.batch_requests -= 1;
                load.batch_work -= self.units;
            }
        }
    }
}
pub(super) struct RoutedRequest<M> {
    pub message: M,
    pub units: usize,
    pub _charge: Charge,
    pub _permit: tokio::sync::OwnedSemaphorePermit,
}
impl<M: SchemeServiceRequest> SchemeServiceRequest for RoutedRequest<M> {
    type Output = M::Output;
    fn work_units(&self) -> usize {
        self.units
    }
    fn short_service(&self) -> bool {
        self.message.short_service()
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.message.construct(session)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        self.message.project(session, output)
    }
}
