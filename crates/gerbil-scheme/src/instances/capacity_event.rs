//! Capacity returns remain immediate; waiters register before scanning credits.
use tokio::sync::{Notify, watch};
#[derive(Debug)]
pub(super) struct CapacityEvents {
    watch: watch::Sender<()>,
    pub(super) notify: Notify,
    pub(super) short_pool: std::sync::OnceLock<std::sync::Weak<super::short_pool::Source>>,
}
impl CapacityEvents {
    pub(super) fn new() -> Self {
        Self {
            watch: watch::channel(()).0,
            notify: Notify::new(),
            short_pool: std::sync::OnceLock::new(),
        }
    }
    pub(super) fn subscribe(&self) -> watch::Receiver<()> {
        self.watch.subscribe()
    }
    pub(super) fn publish(&self) {
        // Every watch router retains a receiver throughout its lifetime. Without
        // one, no watch consumer needs a historical event and send avoids the
        // payload write lock. Notify consumers arm before scanning live credits.
        let _ = self.watch.send(());
        self.notify.notify_waiters();
        if let Some(source) = self.short_pool.get().and_then(std::sync::Weak::upgrade) {
            source.capacity_available();
        }
    }
}
#[cfg(test)]
#[path = "../../tests/unit/capacity_event.rs"]
mod tests;
