//! Reuse the image's existing bounded native select/sleep trace, diagnostic only.
use super::GATE;
use gerbil_scheme::instances::InstancePool;
use serde_json::{Value, json};
use std::time::Instant;
pub(super) struct Anchor {
    at: Instant,
    clock: u64,
    uncertainty: u64,
}
pub(super) fn clock() -> u64 {
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: writable timespec using the native trace's monotonic clock.
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &raw mut now) },
        0
    );
    u64::try_from(now.tv_sec).unwrap() * 1_000_000_000 + u64::try_from(now.tv_nsec).unwrap()
}
pub(super) fn anchor() -> Anchor {
    let before = Instant::now();
    let clock = clock();
    let after = Instant::now();
    Anchor {
        at: before,
        clock,
        uncertainty: u64::try_from(after.duration_since(before).as_nanos()).unwrap(),
    }
}
impl Anchor {
    pub(super) fn clock(&self, at: Instant) -> u64 {
        if at >= self.at {
            self.clock
                .saturating_add(u64::try_from(at.duration_since(self.at).as_nanos()).unwrap())
        } else {
            self.clock
                .saturating_sub(u64::try_from(self.at.duration_since(at).as_nanos()).unwrap())
        }
    }
    pub(super) fn marks(&self, marks: &[Option<Instant>; 9]) -> Value {
        json!({"uncertainty_ns":self.uncertainty,"marks_ns":marks.map(|at|at.map(|at|self.clock(at)))})
    }
}
fn control(pool: &InstancePool, command: &[i64]) -> Vec<Vec<i64>> {
    let tickets: Vec<_> = (0..4)
        .map(|owner| {
            let command = command.to_vec();
            pool.dispatcher()
                .submit_native_on(owner, move |session| {
                    let input = session.numeric_vector(&command)?;
                    let output = session.call(&input)?;
                    session.copy_numeric::<i64>(&output)
                })
                .unwrap()
        })
        .collect();
    tickets
        .into_iter()
        .map(|ticket| ticket.wait(GATE).unwrap())
        .collect()
}
pub(super) fn begin(pool: &InstancePool) {
    assert!(control(pool, &[10, 1]).iter().all(|row| row == &[0]));
}
pub(super) fn finish(pool: &InstancePool, inputs: &[Vec<i64>]) -> Value {
    let events = control(pool, &[11]);
    let mut sleeps = 0;
    for row in &events {
        assert_eq!(&row[..2], &[0, 0], "native trace overflow/error");
        assert_eq!((row.len() - 2) % 6, 0);
        for event in row[2..].chunks_exact(6) {
            assert!((1..=5).contains(&event[0]) && event[2] >= event[1] && event[3] >= 0);
            sleeps += usize::from(event[0] == 2);
        }
    }
    assert_eq!(sleeps, inputs.iter().filter(|row| row[1] == 1).count());
    json!(events)
}

// Reuse the existing image control; only the fixed owner OS class changes.
pub(super) fn qos(pool: &InstancePool, elevated: bool) -> Value {
    let receipts = control(pool, &[12, i64::from(elevated)]);
    assert!(
        receipts.iter().all(|row| row.len() == 3
            && row[0] == 0
            && if elevated {
                row[1] == 25
            } else {
                row[1] == row[2]
            }),
        "owner QoS application/restoration failed: {receipts:?}"
    );
    json!(receipts)
}

pub(super) fn append(value: &mut Value, pool: &InstancePool, inputs: &[Vec<i64>]) {
    value["native_wait_events"] = finish(pool, inputs);
    value["native_wait_event_width"] = json!(6);
}
