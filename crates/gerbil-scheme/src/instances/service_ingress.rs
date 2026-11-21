//! Bounded Rust input admission; Scheme alone executes native work.
use super::{InstanceError, SchemeRouteClass};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use tokio::sync::Notify;

/// Explicit queue and client reservations. Neither limit is a Scheme work budget.
#[derive(Clone, Copy, Debug)]
pub struct SchemeIngressConfig {
    pub queue_capacity: usize,
    pub client_capacity: usize,
    /// Short and bulk minimum opportunities, respectively. Idle shares may be borrowed.
    pub queue_reserved: [usize; 2],
    pub client_reserved: [usize; 2],
}
/// Root-free Rust inputs waiting and claimed through result consumption.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeIngressSnapshot {
    pub queued: [usize; 2],
    pub clients: [usize; 2],
    pub closed: bool,
}
/// Opaque, ingress-specific identity for cancellation before claim.
#[derive(Clone, Debug)]
pub struct SchemeIngressId {
    sequence: u64,
    class: usize,
    origin: Arc<()>,
}
/// Admission may reclaim a borrowed queue slot. The displaced input is returned
/// to its caller for an explicit rejected/cancelled outcome, never silently lost.
#[derive(Debug)]
pub struct SchemeIngressAdmission<T> {
    pub id: SchemeIngressId,
    pub reclaimed: Option<T>,
}
/// Rejected input and explicit admission error; ownership returns to the caller.
#[derive(Debug)]
pub struct SchemeIngressRejected<T> {
    pub reason: InstanceError,
    pub value: T,
}
struct Queued<T> {
    sequence: u64,
    value: T,
}
struct State<T> {
    queue: [VecDeque<Queued<T>>; 2],
    clients: [usize; 2],
    next: u64,
    cursor: usize,
    closed: bool,
}
struct Shared<T> {
    state: Mutex<State<T>>,
    changed: Notify,
    config: SchemeIngressConfig,
    origin: Arc<()>,
}
/// Shared bounded ingress before typed router submission. Store owned Rust inputs
/// here, construct Scheme values only on the selected owner. Clones share quotas.
/// Callers retain a claim through native completion AND client consumption; an
/// idle class can lend active slots, but active native work is never preempted.
pub struct SchemeServiceIngress<T> {
    shared: Arc<Shared<T>>,
}
impl<T> Clone for SchemeServiceIngress<T> {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}
impl<T> std::fmt::Debug for SchemeServiceIngress<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemeServiceIngress")
            .field("snapshot", &self.snapshot())
            .finish()
    }
}
fn index(class: SchemeRouteClass) -> usize {
    match class {
        SchemeRouteClass::Short => 0,
        SchemeRouteClass::Batch => 1,
    }
}
impl<T> SchemeServiceIngress<T> {
    /// Create an opt-in ingress with independent finite queue/client budgets.
    /// # Errors
    /// Zero capacity/reservation or reservation sums exceeding their capacity.
    pub fn new(config: SchemeIngressConfig) -> Result<Self, InstanceError> {
        if config.queue_capacity == 0
            || config.client_capacity == 0
            || config.queue_reserved.contains(&0)
            || config.client_reserved.contains(&0)
            || config.queue_reserved[0]
                .checked_add(config.queue_reserved[1])
                .is_none_or(|n| n > config.queue_capacity)
            || config.client_reserved[0]
                .checked_add(config.client_reserved[1])
                .is_none_or(|n| n > config.client_capacity)
        {
            return Err(InstanceError::InvalidConfig(
                "finite ingress class reservations",
            ));
        }
        Ok(Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    queue: [VecDeque::new(), VecDeque::new()],
                    clients: [0, 0],
                    next: 0,
                    cursor: 0,
                    closed: false,
                }),
                changed: Notify::new(),
                config,
                origin: Arc::new(()),
            }),
        })
    }
    /// Enqueue without spawning or preparing a Scheme payload. A queued borrower
    /// may be displaced to restore the arriving class's reserved queue share.
    /// # Errors
    /// Closed/full ingress; the unaccepted owned input is returned intact.
    pub fn try_enqueue(
        &self,
        class: SchemeRouteClass,
        value: T,
    ) -> Result<SchemeIngressAdmission<T>, SchemeIngressRejected<T>> {
        let c = index(class);
        let other = 1 - c;
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed {
            return Err(SchemeIngressRejected {
                reason: InstanceError::Closed,
                value,
            });
        }
        let Some(next) = state.next.checked_add(1) else {
            return Err(SchemeIngressRejected {
                reason: InstanceError::InvalidConfig("ingress identity space exhausted"),
                value,
            });
        };
        let full = state.queue.iter().map(VecDeque::len).sum::<usize>()
            == self.shared.config.queue_capacity;
        let reclaimed = if full {
            if state.queue[c].len() >= self.shared.config.queue_reserved[c]
                || state.queue[other].len() <= self.shared.config.queue_reserved[other]
            {
                return Err(SchemeIngressRejected {
                    reason: InstanceError::QueueFull,
                    value,
                });
            }
            state.queue[other].pop_back().map(|entry| entry.value)
        } else {
            None
        };
        let sequence = state.next;
        state.next = next;
        state.queue[c].push_back(Queued { sequence, value });
        drop(state);
        self.shared.changed.notify_one();
        Ok(SchemeIngressAdmission {
            id: SchemeIngressId {
                sequence,
                class: c,
                origin: Arc::clone(&self.shared.origin),
            },
            reclaimed,
        })
    }
    /// Cancel only unclaimed input. A claimed input retains its native cancellation
    /// and ownership protocol; cancellation must not release active native quotas.
    pub fn cancel_pending(&self, id: &SchemeIngressId) -> Option<T> {
        if !Arc::ptr_eq(&id.origin, &self.shared.origin) {
            return None;
        }
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let queue = &mut state.queue[id.class];
        let at = queue
            .iter()
            .position(|entry| entry.sequence == id.sequence)?;
        let value = queue.remove(at).map(|entry| entry.value);
        drop(state);
        self.shared.changed.notify_waiters();
        value
    }
    /// Claim one input with an atomic class/client reservation. When both classes
    /// wait, restore a deficient reservation first, then alternate surplus claims.
    /// A borrowed active slot returns only at its existing completion boundary.
    pub async fn recv(&self) -> Option<SchemeIngressClaim<T>> {
        loop {
            let changed = self.shared.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            {
                let mut state = self
                    .shared
                    .state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if state.clients.iter().sum::<usize>() < self.shared.config.client_capacity {
                    let chosen = (0..2)
                        .map(|n| (state.cursor + n) % 2)
                        .find(|&c| {
                            !state.queue[c].is_empty()
                                && state.clients[c] < self.shared.config.client_reserved[c]
                        })
                        .or_else(|| {
                            (0..2)
                                .map(|n| (state.cursor + n) % 2)
                                .find(|&c| !state.queue[c].is_empty())
                        });
                    if let Some((c, entry)) =
                        chosen.and_then(|c| state.queue[c].pop_front().map(|entry| (c, entry)))
                    {
                        state.clients[c] += 1;
                        state.cursor = 1 - c;
                        return Some(SchemeIngressClaim {
                            input: Some(entry.value),
                            class: c,
                            shared: Arc::clone(&self.shared),
                        });
                    }
                }
                if state.closed && state.queue.iter().all(VecDeque::is_empty) {
                    return None;
                }
            }
            changed.await;
        }
    }
    /// Stop publication and wake receivers. Accepted queued inputs remain drainable.
    pub fn close(&self) {
        self.shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .closed = true;
        self.shared.changed.notify_waiters();
    }
    /// Observe bounded Rust state, independent of native roots and owner budgets.
    #[must_use]
    pub fn snapshot(&self) -> SchemeIngressSnapshot {
        let state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        SchemeIngressSnapshot {
            queued: [state.queue[0].len(), state.queue[1].len()],
            clients: state.clients,
            closed: state.closed,
        }
    }
}
/// RAII client charge. Keep this object through result consumption or error
/// handling. Dropping before native completion does not cancel native work and
/// is a caller protocol error; native tickets keep their separate owner charges.
#[must_use = "retain this claim through native completion and client consumption"]
pub struct SchemeIngressClaim<T> {
    input: Option<T>,
    class: usize,
    shared: Arc<Shared<T>>,
}
impl<T> SchemeIngressClaim<T> {
    /// Admission class to carry into the existing typed router without inference.
    #[must_use]
    pub fn route_class(&self) -> SchemeRouteClass {
        if self.class == 0 {
            SchemeRouteClass::Short
        } else {
            SchemeRouteClass::Batch
        }
    }
    /// Move owned Rust input out once, retaining the client charge in this claim.
    pub fn take(&mut self) -> Option<T> {
        self.input.take()
    }
}
impl<T> Drop for SchemeIngressClaim<T> {
    fn drop(&mut self) {
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.clients[self.class] -= 1;
        drop(state);
        self.shared.changed.notify_one();
    }
}

impl<T> std::fmt::Debug for SchemeIngressClaim<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemeIngressClaim")
            .field("class", &self.class)
            .field("input_taken", &self.input.is_none())
            .finish_non_exhaustive()
    }
}
