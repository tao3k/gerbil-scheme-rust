//! Perturbing wake-path observation, never production scheduler admission.
use std::{
    future::{Future, poll_fn},
    pin::pin,
    sync::{
        Arc,
        atomic::{
            AtomicU64,
            Ordering::{Acquire, Relaxed, Release},
        },
    },
    task::{Context, Wake, Waker},
    time::Instant,
};
pub(super) struct Probe {
    original: Waker,
    epoch: Instant,
    wakes: AtomicU64,
    start: AtomicU64,
    end: AtomicU64,
}
impl Wake for Probe {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.start.store(
            u64::try_from(self.epoch.elapsed().as_nanos()).unwrap(),
            Release,
        );
        self.wakes.fetch_add(1, Relaxed);
        self.original.wake_by_ref();
        self.end.store(
            u64::try_from(self.epoch.elapsed().as_nanos()).unwrap(),
            Release,
        );
    }
}
/// Fields: polls, pending polls, forwarded wakes, synchronous wake duration,
/// wake-start to ready-poll duration, complete single-wake observation flag.
pub(super) async fn observe<F: Future>(
    future: F,
    probe: &mut Option<Arc<Probe>>,
) -> (F::Output, [u64; 6]) {
    let mut future = pin!(future);
    let (mut polls, mut pending, mut previous) = (0, 0, 0);
    let mut resumed = 0;
    let result = poll_fn(|cx| {
        let probe = probe.get_or_insert_with(|| {
            Arc::new(Probe {
                original: cx.waker().clone(),
                epoch: Instant::now(),
                wakes: AtomicU64::new(0),
                start: AtomicU64::new(0),
                end: AtomicU64::new(0),
            })
        });
        assert!(
            probe.original.will_wake(cx.waker()),
            "stable caller task identity"
        );
        if polls == 0 {
            previous = probe.wakes.load(Relaxed);
        }
        let at_poll = u64::try_from(probe.epoch.elapsed().as_nanos()).unwrap();
        let waker = Waker::from(Arc::clone(probe));
        polls += 1;
        let result = future.as_mut().poll(&mut Context::from_waker(&waker));
        if result.is_pending() {
            pending += 1;
        } else {
            resumed = at_poll;
        }
        result
    })
    .await;
    let probe = probe.as_ref().unwrap();
    let wakes = probe.wakes.load(Relaxed) - previous;
    let start = probe.start.load(Acquire);
    let end = probe.end.load(Acquire);
    // A task may resume before wake_by_ref returns. Never block it to complete
    // the observation; exclude overlapping/multiple-wake intervals explicitly.
    let complete = wakes == 1 && end >= start && start > 0 && resumed >= end;
    (
        result,
        [
            polls,
            pending,
            wakes,
            if complete { end - start } else { 0 },
            if complete { resumed - start } else { 0 },
            u64::from(complete),
        ],
    )
}
