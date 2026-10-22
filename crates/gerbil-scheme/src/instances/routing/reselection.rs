//! FIFO class admission, rechecking all owners without partial-credit waits.
use super::{
    charge::{Charge, RoutedRequest},
    policy::SchemeServiceRouter,
    types::{Class, RoutedSchemeTicket},
};
use crate::instances::{
    InstanceError, SchemeServiceEntryPhase, SchemeServiceRequest, service_contract::Checked,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
impl SchemeServiceRouter {
    pub(super) async fn submit_reselected<M: SchemeServiceRequest>(
        &self,
        class: Class,
        message: Checked<M>,
        timeout: Duration,
    ) -> Result<RoutedSchemeTicket<M::Output>, InstanceError> {
        if timeout.is_zero() {
            return Err(InstanceError::Timeout);
        }
        let started = Instant::now();
        let execution_class = message.execution;
        let trace = message.entry_trace();
        let units = message.work_units();
        let reserve = async {
            let gate = match class {
                Class::Short => &self.state.short_gate,
                Class::Batch => &self.state.batch_gate,
            };
            // One FIFO waiter per class scans, so newer arrivals cannot repeatedly
            // steal credit from an older waiter in this router. Classes stay separate.
            let turn = Arc::clone(gate)
                .acquire_owned()
                .await
                .map_err(|_| InstanceError::Closed)?;
            let events = self.state.changed.get().unwrap();
            let mut changes = self.state.watch.clone();
            loop {
                let notified = events.notify.notified();
                tokio::pin!(notified);
                if let Some(changes) = changes.as_mut() {
                    changes.mark_unchanged();
                } else {
                    // Registration precedes the scan, including its failed path.
                    // notify_waiters after this point cannot be lost before await.
                    notified.as_mut().enable();
                }
                if gate.is_closed() {
                    return Err(InstanceError::Closed);
                }
                let eligible = self.live_indices();
                if eligible.is_empty() {
                    return Err(InstanceError::Closed);
                }
                for index in Charge::ranked(&self.state, class, &eligible) {
                    let target = &self.state.destinations[index];
                    let quota = match class {
                        Class::Short => &target.short,
                        Class::Batch => &target.batch,
                    };
                    let permit = match Arc::clone(quota).try_acquire_owned() {
                        Ok(permit) => permit,
                        Err(tokio::sync::TryAcquireError::NoPermits) => continue,
                        Err(tokio::sync::TryAcquireError::Closed) => {
                            return Err(InstanceError::Closed);
                        }
                    };
                    match self.dispatch.validate_service_on(target.owner, &message) {
                        Ok(()) => {}
                        Err(InstanceError::Closed) => continue,
                        Err(error) => return Err(error),
                    }
                    match self
                        .dispatch
                        .try_reserve_owner(target.owner, units, &target.capacity)
                    {
                        Ok(reservation) => {
                            let charge =
                                Charge::reserve(Arc::clone(&self.state), class, units, index)?;
                            return Ok((turn, permit, charge, reservation));
                        }
                        Err(InstanceError::QueueFull | InstanceError::Closed) => {}
                        Err(error) => return Err(error),
                    }
                    // No class, work or request-slot permit crosses this wait.
                    drop(permit);
                }
                if let Some(changes) = changes.as_mut() {
                    changes.changed().await.map_err(|_| InstanceError::Closed)?;
                } else {
                    notified.await;
                }
            }
        };
        let (_turn, permit, charge, reservation) = tokio::time::timeout(timeout, reserve)
            .await
            .map_err(|_| InstanceError::Timeout)??;
        if started.elapsed() >= timeout {
            return Err(InstanceError::Timeout);
        }
        if let Some(trace) = &trace {
            trace.mark(SchemeServiceEntryPhase::ClassReady);
        }
        let owner = reservation.owner;
        let request = RoutedRequest {
            message,
            units,
            _permit: permit,
            _charge: charge,
        };
        let ticket = self
            .dispatch
            .publish_reserved_service(request, reservation)?;
        Ok(RoutedSchemeTicket {
            owner,
            ticket,
            route_class: class,
            execution_class,
        })
    }
}
