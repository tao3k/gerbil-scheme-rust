//! Forward/reverse matched controls share native owners, routes and corpus.
use super::{CASES, Executor, corpus, observed, sample};
use gerbil_scheme::instances::InstancePool;
use serde_json::json;
use std::sync::mpsc::Sender;
pub(super) fn run(
    pool: &InstancePool,
    smoke: bool,
    profile: bool,
    grouped_ab: bool,
    progress: &Sender<&'static str>,
) {
    let owners = pool.owners().len();
    let states: Vec<_> = pool.owners().iter().map(|o| o.state).collect();
    let jobs = if smoke { 128 } else { 1024 };
    let blocks = if smoke {
        1
    } else if profile {
        2
    } else {
        5
    };
    for mixed in [false, true] {
        let inputs: Vec<_> = (0..jobs).map(|id| corpus::order(id, mixed)).collect();
        for batch in [1, 32] {
            let mut samples: Vec<Vec<sample::Sample>> =
                (0..CASES.len() * 2).map(|_| Vec::new()).collect();
            for block in 0..blocks {
                let order: Vec<_> = (0..CASES.len())
                    .map(|i| (i + block) % CASES.len())
                    .collect();
                for reverse in [false, true] {
                    let sweep: Vec<_> = if reverse {
                        order.iter().rev().copied().collect()
                    } else {
                        order.clone()
                    };
                    for case in sweep {
                        let arms: &[bool] = if !grouped_ab {
                            &[false]
                        } else if reverse {
                            &[true, false]
                        } else {
                            &[false, true]
                        };
                        for &grouped in arms {
                            let (mode, workers) = CASES[case];
                            let executor = Executor::start(mode, workers, profile, grouped);
                            sample::run(pool, &executor, &inputs[..128], batch, progress);
                            let value = sample::run(pool, &executor, &inputs, batch, progress);
                            println!(
                                "{}",
                                json!({"event": "dispatch_sample", "diagnostic": profile, "grouped": grouped,
                                "owners": owners, "mode": mode, "workers": workers, "mixed": mixed, "batch": batch,
                                "block": block, "elapsed_ns": value.elapsed, "native_latency_ns": value.latency,
                                "owner_progress_min": value.owner_progress_min, "window_delivery_ns": value.delivery, "window_after_native_ns": value.after_native})
                            );
                            executor.finish();
                            samples[case * 2 + usize::from(grouped)].push(value);
                            assert_eq!(
                                states,
                                pool.owners().iter().map(|o| o.state).collect::<Vec<_>>()
                            );
                            observed(
                                progress,
                                "measured dispatcher workers retired with VM identities unchanged",
                            );
                        }
                    }
                }
            }
            for (case, &(mode, workers)) in CASES.iter().enumerate() {
                for grouped in [false, true] {
                    let values = &samples[case * 2 + usize::from(grouped)];
                    if !values.is_empty() {
                        sample::emit(
                            values,
                            owners,
                            (mode, workers, profile, grouped),
                            mixed,
                            batch,
                            smoke,
                            jobs,
                        );
                    }
                }
            }
        }
    }
}
