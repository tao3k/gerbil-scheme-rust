//! Directed correctness diagnosis; historical configurations are not performance admission.
use super::{Comparison, Mode, corpus, measurement, stall_forensics};
use gerbil_scheme::instances::InstancePool;
use std::sync::{Arc, mpsc};
pub(super) fn selected() -> bool {
    std::env::args().any(|s| {
        s == "--protocol-check"
            || s == "--watchdog-replay"
            || s == "--watchdog-interleaving"
            || s == "--watchdog-capacity-handoff"
            || s == "--watchdog-source-handoff"
    })
}
pub(super) fn run(
    pool: &InstancePool,
    runtime: &tokio::runtime::Runtime,
    progress: &mpsc::Sender<&'static str>,
    comparison: Comparison,
    observation: Mode,
) {
    assert!(comparison == Comparison::ProgressFair);
    assert!(observation == Mode::Off);
    assert!(super::settings::supply_cycle_ab());
    assert!(super::settings::completion_service_ab());
    if std::env::args().any(|s| {
        s == "--watchdog-replay"
            || s == "--watchdog-interleaving"
            || s == "--watchdog-capacity-handoff"
            || s == "--watchdog-source-handoff"
    }) {
        replay(pool, runtime, progress, comparison, observation);
        return;
    }
    // The last completed historical window and its expected successor both
    // selected whole-chunk preparation. Keep ONE fixed policy, not two arms.
    let warm = Arc::new((0..256).map(|id| corpus::order(id, true)).collect());
    let inputs = Arc::new((0..8192).map(|id| corpus::order(id, true)).collect());
    let _ = measurement::run(pool, runtime, &warm, 8, comparison, false, observation);
    progress.send("protocol warmup validated").unwrap();
    for window in 0..32 {
        stall_forensics::context(window, false, true, 0);
        let _ = measurement::run(pool, runtime, &inputs, 64, comparison, false, observation);
        progress.send("protocol window validated").unwrap();
        println!(
            "PROTOCOL-CHECK window={window} policy=whole-chunk result=validated performance-admission=false"
        );
    }
}

// Preserve the historical call/configuration order through the first
// unvalidated window. Returned measurement values are discarded, never compared.
fn replay(
    pool: &InstancePool,
    runtime: &tokio::runtime::Runtime,
    progress: &mpsc::Sender<&'static str>,
    comparison: Comparison,
    observation: Mode,
) {
    for mixed in [false, true] {
        let inputs = Arc::new(
            (0..8192)
                .map(|id| corpus::order(id, mixed))
                .collect::<Vec<_>>(),
        );
        let warm = Arc::new(inputs[..256].to_vec());
        for arm in [false, true] {
            stall_forensics::context(0, false, mixed, usize::from(arm));
            let _ = measurement::run(pool, runtime, &warm, 8, comparison, arm, observation);
            progress.send("historical warmup validated").unwrap();
        }
        let mut retained = [Vec::new(), Vec::new()];
        for block in 0..5 {
            for reverse in [false, true] {
                for arm in if reverse { [1, 0] } else { [0, 1] } {
                    stall_forensics::context(block, reverse, mixed, arm);
                    if mixed
                        && block == 3
                        && !reverse
                        && arm == 0
                        && std::env::args().any(|s| {
                            s == "--watchdog-interleaving"
                                || s == "--watchdog-capacity-handoff"
                                || s == "--watchdog-source-handoff"
                        })
                    {
                        if std::env::args().any(|s| s == "--watchdog-source-handoff") {
                            super::source_handoff::run(pool, runtime);
                        } else if std::env::args().any(|s| s == "--watchdog-capacity-handoff") {
                            super::capacity_handoff::run(pool, runtime);
                        } else {
                            retirement_interleaving(pool, runtime);
                        }
                        progress
                            .send("controlled wait-dependency interleaving validated")
                            .unwrap();
                        return;
                    }
                    let value = measurement::run(
                        pool,
                        runtime,
                        &inputs,
                        64,
                        comparison,
                        arm == 1,
                        observation,
                    );
                    // Match the historical per-policy result lifetime through
                    // this workload group, without computing comparison statistics.
                    retained[arm].push(value);
                    progress.send("historical window validated").unwrap();
                    println!(
                        "WATCHDOG-REPLAY mixed={mixed} block={block} reverse={reverse} arm={arm} result=validated performance-admission=false"
                    );
                    if mixed && block == 3 && !reverse && arm == 0 {
                        return;
                    }
                }
            }
        }
    }
}

#[cfg(feature = "arrival-clock")]
fn retirement_interleaving(pool: &InstancePool, runtime: &tokio::runtime::Runtime) {
    super::retirement_interleaving::run(pool, runtime);
}
#[cfg(not(feature = "arrival-clock"))]
fn retirement_interleaving(_: &InstancePool, _: &tokio::runtime::Runtime) {
    panic!("watchdog interleaving requires the arrival-clock feature");
}
