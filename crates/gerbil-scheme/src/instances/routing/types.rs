//! Shared class-routing contract and load ledger; no scheduling implementation.
use crate::instances::{AsyncInstanceTicket, SchemeServiceExecution};
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
/// Explicit router contract, checked against existing owner slot/work budgets.
/// Callers sharing class protection must use clones of one router. Direct
/// dispatcher users and independent routers do not share these class quotas.
#[derive(Clone, Debug)]
pub struct SchemeRoutingConfig {
    pub owners: Vec<usize>,
    pub short_slots: usize,
    pub batch_slots: usize,
    pub short_max_work: usize,
    pub batch_max_work: usize,
}
/// Router load through projection, not a prediction of runtime duration.
/// Pinned includes per-owner admission waiters; Reselect charges an owner only
/// after all credits are reserved. Unassigned FIFO waiters have no owner charge.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SchemeRouteLoad {
    pub owner: usize,
    pub short_requests: usize,
    pub batch_requests: usize,
    pub batch_work: usize,
}
/// The selected fixed owner and its ordinary typed result ticket.
#[derive(Debug)]
pub struct RoutedSchemeTicket<R: Send + 'static> {
    pub owner: usize,
    pub ticket: AsyncInstanceTicket<R>,
    pub route_class: SchemeRouteClass,
    pub execution_class: SchemeServiceExecution,
}
/// Host admission/routing class; independent of the native execution queue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemeRouteClass {
    Short,
    Batch,
}
pub(super) use SchemeRouteClass as Class;
/// Existing pinned admission remains the default. Reselect commits an owner
/// only after class and native work/slot credits have all been reserved.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SchemeRoutingAdmission {
    #[default]
    Pinned,
    Reselect,
    /// Reselect using native occupancy as well as this router's class ledger.
    /// Includes resident producer reservations through native retirement. These
    /// are approximate capacity observations, not predicted service durations.
    ReselectProgress,
    /// Same occupancy ranking, with register-before-scan capacity notification.
    /// No payload lock or retry timer is needed for this wait protocol.
    ReselectProgressNotified,
}
pub(super) struct Destination {
    pub(super) owner: usize,
    pub(super) capacity: Arc<crate::instances::capacity::Capacity>,
    pub(super) short: Arc<Semaphore>,
    pub(super) batch: Arc<Semaphore>,
}
pub(super) struct Ledger {
    pub(super) loads: Vec<SchemeRouteLoad>,
    pub(super) short_cursor: usize,
    pub(super) batch_cursor: usize,
}
pub(super) struct State {
    pub(super) changed: Arc<std::sync::OnceLock<crate::instances::capacity_event::CapacityEvents>>,
    pub(super) watch: Option<tokio::sync::watch::Receiver<()>>,
    pub(super) short_gate: Arc<Semaphore>,
    pub(super) batch_gate: Arc<Semaphore>,
    pub(super) destinations: Vec<Destination>,
    pub(super) ledger: Mutex<Ledger>,
    pub(super) short_slots: usize,
    pub(super) batch_slots: usize,
    pub(super) producer_progress: bool,
}
