//! Verified epochs extend a deadline without waking an observer per request.
use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicU8, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
#[derive(Clone, Copy)]
pub(super) enum Event {
    Initialized,
    NativeCompleted,
    ServiceCompleted,
    ControlsCompleted,
    Retired,
}
impl Event {
    fn code(self) -> u8 {
        match self {
            Self::Initialized => 1,
            Self::NativeCompleted => 2,
            Self::ServiceCompleted => 3,
            Self::ControlsCompleted => 4,
            Self::Retired => 5,
        }
    }
}
struct State {
    anchor: Instant,
    limit: Duration,
    last_ns: AtomicU64,
    event: AtomicU8,
    done: Mutex<bool>,
    stopped: Condvar,
}
impl State {
    fn new(limit: Duration) -> Self {
        Self {
            anchor: Instant::now(),
            limit,
            last_ns: AtomicU64::new(0),
            event: AtomicU8::new(0),
            done: Mutex::new(false),
            stopped: Condvar::new(),
        }
    }
    // Returns false for an out-of-order record. An already expired deadline
    // cannot be revived by a queued or late notification.
    fn verify_at(&self, now: Duration) -> Result<bool, ()> {
        let now = u64::try_from(now.as_nanos()).expect("bounded benchmark epoch");
        let mut last = self.last_ns.load(Ordering::Acquire);
        loop {
            if now <= last {
                return Ok(false);
            }
            if Duration::from_nanos(now - last) >= self.limit {
                match self
                    .last_ns
                    .compare_exchange(last, last, Ordering::AcqRel, Ordering::Acquire)
                {
                    Ok(_) => return Err(()),
                    Err(new) => {
                        last = new;
                        continue;
                    }
                }
            }
            match self.last_ns.compare_exchange_weak(
                last,
                now,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(true),
                Err(new) => last = new,
            }
        }
    }
    fn remaining_at(&self, now: Duration) -> Duration {
        let last = Duration::from_nanos(self.last_ns.load(Ordering::Acquire));
        self.limit.saturating_sub(now.saturating_sub(last))
    }
}
#[derive(Clone)]
pub(super) struct Progress(Arc<State>);
impl Progress {
    pub(super) fn record(&self, event: Event) {
        match self.0.verify_at(self.0.anchor.elapsed()) {
            Ok(true) => self.0.event.store(event.code(), Ordering::Relaxed),
            Ok(false) => {}
            Err(()) => stall(&self.0),
        }
    }
}
pub(super) struct Watchdog {
    state: Arc<State>,
    task: thread::JoinHandle<()>,
}
impl Watchdog {
    pub(super) fn start(limit: Duration) -> (Self, Progress) {
        assert!(!limit.is_zero());
        let state = Arc::new(State::new(limit));
        let observer = Arc::clone(&state);
        let task = thread::spawn(move || observe(&observer));
        (
            Self {
                state: Arc::clone(&state),
                task,
            },
            Progress(state),
        )
    }
    pub(super) fn finish(self) {
        *self
            .state
            .done
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        self.state.stopped.notify_one();
        self.task.join().unwrap();
    }
}
fn observe(state: &State) {
    let mut done = state
        .done
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    loop {
        if *done {
            return;
        }
        let remaining = state.remaining_at(state.anchor.elapsed());
        if remaining.is_zero() {
            // Recheck after the clock read in case a verified completion raced
            // the old deadline. Progress never needs an OS notification here:
            // it can only move the next deadline later.
            if state.remaining_at(state.anchor.elapsed()).is_zero() {
                stall(state);
            }
            continue;
        }
        (done, _) = state
            .stopped
            .wait_timeout(done, remaining)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
    }
}
fn stall(state: &State) -> ! {
    let event = match state.event.load(Ordering::Relaxed) {
        1 => "initialized",
        2 => "native completion",
        3 => "service completion",
        4 => "exception controls",
        5 => "retired",
        _ => "start",
    };
    eprintln!("SINGLE-VM-STALL last={event} cleanup=UNVERIFIED");
    std::process::exit(70)
}
#[cfg(test)]
mod tests {
    #[test]
    fn verification_extends_exact_deadline_without_notification() {
        use super::State;
        use std::time::Duration;
        let state = State::new(Duration::from_secs(5));
        assert_eq!(state.verify_at(Duration::from_secs(2)), Ok(true));
        assert_eq!(
            state.remaining_at(Duration::from_secs(6)),
            Duration::from_secs(1)
        );
        assert_eq!(state.verify_at(Duration::from_secs(7)), Err(()));
        assert_eq!(state.remaining_at(Duration::from_secs(7)), Duration::ZERO);
    }
    #[test]
    fn stale_records_cannot_regress_or_revive_progress() {
        use super::State;
        use std::time::Duration;
        let state = State::new(Duration::from_secs(5));
        assert_eq!(state.verify_at(Duration::from_secs(2)), Ok(true));
        assert_eq!(state.verify_at(Duration::from_secs(1)), Ok(false));
        assert_eq!(state.verify_at(Duration::from_secs(8)), Err(()));
        assert_eq!(
            state.remaining_at(Duration::from_secs(6)),
            Duration::from_secs(1)
        );
    }
    #[test]
    fn completed_observer_is_joined_without_request_notifications() {
        use super::{Event, Watchdog};
        use std::time::Duration;
        let (watchdog, progress) = Watchdog::start(Duration::from_secs(5));
        for event in [
            Event::Initialized,
            Event::ControlsCompleted,
            Event::NativeCompleted,
            Event::ServiceCompleted,
            Event::Retired,
        ] {
            progress.record(event);
        }
        watchdog.finish();
    }
}
