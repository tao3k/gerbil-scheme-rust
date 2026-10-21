//! Shared class-routing contract and load ledger; no scheduling implementation.
use crate::instances::AsyncInstanceTicket;
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
/// Queueing/reserved/accepted router load, not a prediction of runtime duration.
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
}
#[derive(Clone, Copy)]
pub(super) enum Class {
    Short,
    Batch,
}
pub(super) struct Destination {
    pub(super) owner: usize,
    pub(super) short: Arc<Semaphore>,
    pub(super) batch: Arc<Semaphore>,
}
pub(super) struct Ledger {
    pub(super) loads: Vec<SchemeRouteLoad>,
    pub(super) short_cursor: usize,
    pub(super) batch_cursor: usize,
}
pub(super) struct State {
    pub(super) destinations: Vec<Destination>,
    pub(super) ledger: Mutex<Ledger>,
    pub(super) short_slots: usize,
    pub(super) batch_slots: usize,
}
