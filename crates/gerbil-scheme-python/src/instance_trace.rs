//! Opt-in bounded owner spans; no native object or pointer leaves its owner.
use super::{Command, InstanceError, NativeOutput, Registry, values};
use gerbil_scheme::instances::NativeSession;
use serde::Serialize;
use std::{
    ffi::CStr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Instant,
};
const LIMIT: usize = 16384;
/// Optional native phase clocks and cumulative SDK counters on the actual owner.
#[derive(Debug)]
pub(super) struct Stages {
    started: Instant,
    wall: [u64; 2],
    counters: [Option<[f64; 6]>; 2],
}
impl Stages {
    fn new(started: Instant) -> Self {
        Self {
            started,
            wall: [0; 2],
            counters: [None; 2],
        }
    }
    pub(super) fn constructed(&mut self, session: &NativeSession<'_>) {
        self.wall[0] = u64::try_from(self.started.elapsed().as_nanos()).unwrap_or(u64::MAX);
        self.counters[0] = session.diagnostics();
    }
    pub(super) fn executed(&mut self, session: &NativeSession<'_>) {
        self.wall[1] = u64::try_from(self.started.elapsed().as_nanos()).unwrap_or(u64::MAX);
        self.counters[1] = session.diagnostics();
    }
    fn deltas(&self, before: [f64; 6]) -> Option<[[f64; 6]; 2]> {
        let constructed = self.counters[0]?;
        let executed = self.counters[1]?;
        Some([
            std::array::from_fn(|i| constructed[i] - before[i]),
            std::array::from_fn(|i| executed[i] - constructed[i]),
        ])
    }
}
#[derive(Debug)]
pub(super) struct Trace {
    enabled: AtomicBool,
    epoch: Instant,
    rows: Mutex<Vec<Span>>,
    dropped: AtomicUsize,
}
impl Default for Trace {
    fn default() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            epoch: Instant::now(),
            rows: Mutex::new(Vec::new()),
            dropped: AtomicUsize::new(0),
        }
    }
}
#[derive(Debug, Serialize)]
struct Span {
    owner: Option<usize>,
    kind: &'static str,
    queued_ns: u64,
    start_ns: u64,
    end_ns: u64,
    stages_ns: [u64; 2],
    diagnostics_delta: Option<[f64; 6]>,
    stage_diagnostics_delta: Option<[[f64; 6]; 2]>,
    failed: bool,
}
impl Trace {
    pub(super) fn start(&self) -> Option<Instant> {
        self.enabled.load(Ordering::Acquire).then(Instant::now)
    }
    pub(super) fn enable(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
    }
    pub(super) fn take(&self) -> Result<String, String> {
        let rows = std::mem::take(
            &mut *self
                .rows
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        serde_json::to_string(
            &serde_json::json!({"spans": rows, "dropped": self.dropped.swap(0, Ordering::Relaxed)}),
        )
        .map_err(|error| error.to_string())
    }
    pub(super) fn host(&self, kind: &'static str, started: Instant) {
        if !self.enabled.load(Ordering::Acquire) {
            return;
        }
        let ended = Instant::now();
        let mut rows = self
            .rows
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if rows.len() < LIMIT {
            rows.push(Span {
                owner: None,
                kind,
                queued_ns: self.ns(started),
                start_ns: self.ns(started),
                end_ns: self.ns(ended),
                stages_ns: [0; 2],
                diagnostics_delta: None,
                stage_diagnostics_delta: None,
                failed: false,
            });
        } else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn ns(&self, instant: Instant) -> u64 {
        u64::try_from(instant.duration_since(self.epoch).as_nanos()).unwrap_or(u64::MAX)
    }
    pub(super) fn run<R>(
        &self,
        owner: usize,
        kind: &'static str,
        queued: Instant,
        session: &NativeSession<'_>,
        job: impl FnOnce(&mut Option<Stages>) -> Result<R, InstanceError>,
    ) -> Result<R, InstanceError> {
        if !self.enabled.load(Ordering::Acquire) {
            return job(&mut None);
        }
        let started = Instant::now();
        let before = session.diagnostics();
        let mut stages = Some(Stages::new(started));
        let value = job(&mut stages);
        let after = session.diagnostics();
        let ended = Instant::now();
        let mut rows = self
            .rows
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if rows.len() < LIMIT {
            rows.push(Span {
                owner: Some(owner),
                kind,
                queued_ns: self.ns(queued),
                start_ns: self.ns(started),
                end_ns: self.ns(ended),
                stages_ns: stages.as_ref().map_or([0; 2], |stages| stages.wall),
                stage_diagnostics_delta: before.and_then(|before| stages.as_ref()?.deltas(before)),
                diagnostics_delta: before
                    .zip(after)
                    .map(|(a, b)| std::array::from_fn(|i| b[i] - a[i])),
                failed: value.is_err(),
            });
        } else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
        value
    }
}
pub(super) fn prepare(
    command: Command,
    registry: Arc<Registry>,
    apply: &'static CStr,
    owner: usize,
) -> impl for<'vm> FnOnce(&NativeSession<'vm>) -> Result<NativeOutput, InstanceError> + Send {
    let kind = match &command {
        Command::Call(_) => "call",
        Command::Apply(_, _) => "apply",
        Command::Inspect(_, _) => "inspect",
        Command::Release(_) => "release",
        _ => "other",
    };
    prepare_named(command, registry, apply, owner, kind)
}
pub(super) fn prepare_named(
    command: Command,
    registry: Arc<Registry>,
    apply: &'static CStr,
    owner: usize,
    kind: &'static str,
) -> impl for<'vm> FnOnce(&NativeSession<'vm>) -> Result<NativeOutput, InstanceError> + Send {
    let queued = Instant::now();
    move |session| {
        registry.trace.run(owner, kind, queued, session, |stages| {
            values::execute_staged(session, command, &registry, apply, stages)
        })
    }
}
