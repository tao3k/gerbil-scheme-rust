//! Reservation guard spans host queueing through native projection/failure.
use super::types::{Class, Ledger, SchemeRouteLoad, State};
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
    pub(super) fn ranked(
        state: &State,
        class: Class,
        eligible: &std::collections::BTreeSet<usize>,
    ) -> Vec<usize> {
        let ledger = state
            .ledger
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cursor = match class {
            Class::Short => ledger.short_cursor,
            Class::Batch => ledger.batch_cursor,
        };
        let count = ledger.loads.len();
        let mut indices: Vec<_> = (0..count)
            .map(|offset| (cursor + offset) % count)
            .filter(|index| eligible.contains(index))
            .collect();
        indices.sort_by_key(|&index| key(class, index, ledger.loads[index], state));
        indices
    }
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
            .min_by_key(|&index| key(class, index, ledger.loads[index], &state))
            .ok_or(InstanceError::Closed)?;
        record(&mut ledger, class, index, units)?;
        drop(ledger);
        Ok(Self {
            state,
            index,
            class,
            units,
        })
    }
    pub(super) fn reserve(
        state: Arc<State>,
        class: Class,
        units: usize,
        index: usize,
    ) -> Result<Self, InstanceError> {
        let mut ledger = state
            .ledger
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        record(&mut ledger, class, index, units)?;
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
        // Class and load retirement precede the native CapacityPermit release.
        // Its event publishes usable work/slot credits after projection/root balance.
        // A rejected pre-publication reservation still holds the class FIFO turn.
        drop(ledger);
    }
}
pub(super) struct RoutedRequest<M> {
    pub message: M,
    pub units: usize,
    pub _permit: tokio::sync::OwnedSemaphorePermit,
    pub _charge: Charge,
}
impl<M: SchemeServiceRequest> SchemeServiceRequest for RoutedRequest<M> {
    type Output = M::Output;
    fn entry_trace(&self) -> Option<Arc<crate::instances::SchemeServiceEntryTrace>> {
        self.message.entry_trace()
    }
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
    fn construct_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &super::super::SchemeServiceInput,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.message.construct_service(session, input)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        self.message.project(session, output)
    }
}

fn key(
    class: Class,
    index: usize,
    load: SchemeRouteLoad,
    state: &State,
) -> (usize, usize, usize, usize, usize) {
    let capacity = &state.destinations[index].capacity;
    let occupied = if state.producer_progress {
        capacity
            .slot_limit
            .saturating_sub(capacity.slots.available_permits())
    } else {
        0
    };
    match class {
        Class::Short => (
            usize::from(load.short_requests >= state.short_slots),
            load.short_requests,
            occupied,
            load.batch_requests.saturating_add(load.short_requests),
            load.batch_work,
        ),
        Class::Batch => (
            usize::from(load.batch_requests >= state.batch_slots),
            load.batch_work,
            occupied,
            load.batch_requests,
            load.short_requests,
        ),
    }
}

fn record(
    ledger: &mut Ledger,
    class: Class,
    index: usize,
    units: usize,
) -> Result<(), InstanceError> {
    let count = ledger.loads.len();
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
    Ok(())
}
