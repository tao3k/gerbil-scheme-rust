//! Optional diagnostic controls; phase deltas do not identify OS time alone.
use super::GATE;
use gerbil_scheme::instances::{InstancePool, SchemeServiceObservation};
use serde_json::{Value, json};
pub(super) fn snapshot(pool: &InstancePool) -> Vec<SchemeServiceObservation> {
    let tickets: Vec<_> = (0..4)
        .map(|owner| {
            pool.dispatcher()
                .scheme_service_observation_on(owner)
                .unwrap()
        })
        .collect();
    tickets
        .into_iter()
        .map(|ticket| ticket.wait(GATE).unwrap())
        .collect()
}
pub(super) fn delta(
    before: &[SchemeServiceObservation],
    after: &[SchemeServiceObservation],
) -> Value {
    json!(
        before
            .iter()
            .zip(after)
            .map(|(a, b)| {
                a.phases
                    .iter()
                    .zip(b.phases)
                    .map(|(start, end)| {
                        json!({
                            "calls":end.calls-start.calls, "wall_ns":end.wall_ns-start.wall_ns,
                            "cpu_ns":end.cpu_ns.zip(start.cpu_ns).map(|(x,y)|x-y),
                            "gc_real_ns":end.gc_real_ns.zip(start.gc_real_ns).map(|(x,y)|x-y),
                            "gc_cpu_ns":end.gc_cpu_ns.zip(start.gc_cpu_ns).map(|(x,y)|x-y),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    )
}

pub(super) fn append(
    value: &mut Value,
    pool: &InstancePool,
    before: Option<Vec<SchemeServiceObservation>>,
) {
    if let Some(before) = before {
        value["owner_phases"] = delta(&before, &snapshot(pool));
    }
}

/// Qualification collection scope; Probe never enables whole-owner monitors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    Off,
    Entry,
    Probe,
    Phase,
}
impl Mode {
    pub(super) fn paired(self, comparison: super::measurement::Comparison, arm: bool) -> Self {
        if comparison == super::measurement::Comparison::ObserverCost {
            if arm {
                if self == Self::Entry {
                    Self::Entry
                } else {
                    Self::Probe
                }
            } else {
                Self::Off
            }
        } else {
            self
        }
    }
    pub(super) fn enabled(self) -> bool {
        self != Self::Off
    }
    pub(super) fn counters(self) -> bool {
        matches!(self, Self::Probe | Self::Phase)
    }
    pub(super) fn full(self) -> bool {
        self == Self::Phase
    }
}

/// Owns the optional sampler so unwind stops it before native owners retire.
pub(super) struct Capture {
    mode: Mode,
    anchored: bool,
    before: Option<Vec<SchemeServiceObservation>>,
    sampling: Option<super::os_sampling::Sampler>,
    clock: Option<super::native_wait::Anchor>,
}
impl Capture {
    pub(super) fn start(pool: &InstancePool, mode: Mode, balanced_fence: bool) -> Self {
        let before = mode.full().then(|| snapshot(pool));
        let anchored = mode.enabled() || balanced_fence;
        if mode.full() {
            super::native_wait::begin(pool);
        } else if anchored {
            super::native_wait::anchor_begin(pool);
        }
        Self {
            mode,
            anchored,
            before,
            sampling: mode.full().then(super::os_sampling::Sampler::start),
            clock: anchored.then(super::native_wait::anchor),
        }
    }
    pub(super) fn finish(
        self,
        value: &mut Value,
        pool: &InstancePool,
        inputs: &[Vec<i64>],
        snapshots: usize,
        released: std::time::Instant,
    ) {
        super::os_sampling::append(
            value,
            self.sampling.map(super::os_sampling::Sampler::finish),
        );
        if self.mode.full() {
            super::native_wait::append(value, pool, inputs);
        } else if self.anchored {
            super::native_wait::append_probe_anchors(value, pool);
        }
        append(value, pool, self.before);
        value["observation_mode"] = json!(format!("{:?}", self.mode));
        if self.mode.counters() && value["short_layout"] == true {
            assert_eq!(
                snapshots,
                value["probe_actual_ns"].as_array().unwrap().len() * 3,
                "native probe counters unavailable"
            );
        }
        if !self.mode.counters() {
            assert_eq!(snapshots, 0, "unexpected native diagnostic reads");
        }
        value["native_probe_snapshots"] = json!(snapshots);
        value["native_clock_bridge"] = json!(self.anchored);
        value["sample_native_clock"] = json!(self.clock.map(|clock| clock.instant(released)));
    }
}

/// Keep observer wall cost separate from native construction/execution cost.
pub(super) fn counter(
    session: &gerbil_scheme::instances::NativeSession<'_>,
) -> (Option<[f64; 6]>, u128) {
    let start = std::time::Instant::now();
    let value = session.diagnostics();
    (value, start.elapsed().as_nanos())
}

pub(super) fn select(args: &[String]) -> Mode {
    assert!(
        args.iter()
            .filter(|s| matches!(
                s.as_str(),
                "--phase-observation" | "--probe-observation" | "--entry-observation"
            ))
            .count()
            <= 1,
        "observation modes are exclusive"
    );
    if args.iter().any(|s| s == "--phase-observation") {
        Mode::Phase
    } else if args.iter().any(|s| s == "--probe-observation") {
        Mode::Probe
    } else if args.iter().any(|s| s == "--entry-observation") {
        Mode::Entry
    } else {
        Mode::Off
    }
}
