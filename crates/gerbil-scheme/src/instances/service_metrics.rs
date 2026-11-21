//! Optional owner-local phase measurement; native counters never enter Scheme.
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
/// Native owner phases, in snapshot array order.
#[derive(Clone, Copy, Debug)]
pub enum SchemeServicePhase {
    Advance,
    Wait,
    Submit,
    Retire,
    /// Owner-resident Rust supply, including capacity and admission checks.
    Supply,
    /// Native slot/work retirement and host admission notifications.
    Release,
}
/// Cumulative completed phase intervals. CPU and GC overlap; do not add them.
/// `AfterRetirement` publication nests `Release` inside `Retire`; those phase totals
/// are inclusive and must not be added as an exclusive time partition.
/// Wall minus thread CPU includes blocking and descheduling, not isolated OS cost.
#[derive(Clone, Copy, Debug, Default)]
pub struct SchemePhaseObservation {
    pub calls: u64,
    pub wall_ns: u64,
    pub cpu_ns: Option<u64>,
    pub gc_real_ns: Option<u64>,
    /// SDK process CPU over GC intervals, not isolated owner/collector CPU.
    pub gc_cpu_ns: Option<u64>,
}
/// Read on the fixed owner after prior phases finish; no native values escape.
#[derive(Clone, Copy, Debug, Default)]
pub struct SchemeServiceObservation {
    /// Advance, wait, construct/submit, take/project, resident supply, capacity release, respectively.
    pub phases: [SchemePhaseObservation; 6],
}
#[derive(Debug, Default)]
struct Phase {
    calls: AtomicU64,
    wall: AtomicU64,
    cpu: AtomicU64,
    gc_real: AtomicU64,
    gc_cpu: AtomicU64,
    missing: AtomicU64,
}
#[derive(Debug, Default)]
pub(super) struct Monitor {
    phases: [Phase; 6],
    pub(super) history: super::service_history::History,
}
fn nanos(seconds: f64) -> Option<u64> {
    Duration::try_from_secs_f64(seconds)
        .ok()?
        .as_nanos()
        .try_into()
        .ok()
}
impl Monitor {
    pub(super) fn measure<T>(
        &self,
        phase: SchemeServicePhase,
        read: impl Fn() -> Option<[f64; 6]>,
        action: impl FnOnce() -> T,
    ) -> T {
        let started = Instant::now();
        let before = read();
        let value = action();
        let after = read();
        let finished = Instant::now();
        let row = &self.phases[phase as usize];
        row.wall.fetch_add(
            u64::try_from(finished.duration_since(started).as_nanos()).unwrap_or(u64::MAX),
            Ordering::Relaxed,
        );
        row.calls.fetch_add(1, Ordering::Relaxed);
        let delta = before.zip(after).and_then(|(a, b)| {
            Some([
                nanos(b[3] - a[3])?,
                nanos(b[2] - a[2])?,
                nanos((b[0] - a[0]) + (b[1] - a[1]))?,
            ])
        });
        self.history.record(phase, started, finished, delta);
        if let Some([cpu, gc_real, gc_cpu]) = delta {
            row.cpu.fetch_add(cpu, Ordering::Relaxed);
            row.gc_real.fetch_add(gc_real, Ordering::Relaxed);
            row.gc_cpu.fetch_add(gc_cpu, Ordering::Relaxed);
        } else {
            row.missing.fetch_add(1, Ordering::Relaxed);
        }
        value
    }
    pub(super) fn snapshot(&self) -> SchemeServiceObservation {
        SchemeServiceObservation {
            phases: std::array::from_fn(|index| {
                let row = &self.phases[index];
                let valid = row.missing.load(Ordering::Relaxed) == 0;
                SchemePhaseObservation {
                    calls: row.calls.load(Ordering::Relaxed),
                    wall_ns: row.wall.load(Ordering::Relaxed),
                    cpu_ns: valid.then(|| row.cpu.load(Ordering::Relaxed)),
                    gc_real_ns: valid.then(|| row.gc_real.load(Ordering::Relaxed)),
                    gc_cpu_ns: valid.then(|| row.gc_cpu.load(Ordering::Relaxed)),
                }
            }),
        }
    }
}
