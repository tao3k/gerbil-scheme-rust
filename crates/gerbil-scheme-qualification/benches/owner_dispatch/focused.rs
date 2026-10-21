//! Fixed four-VM short-request comparisons; the original matrix stays unchanged.
use super::{GATE, contracts, corpus, executor::Executor, finish, observed, sample};
use gerbil_scheme::instances::{InstanceConfig, InstancePool, IsolatedProgram};
use serde_json::json;
use std::sync::mpsc::Sender;
const CONTROLS: [(&str, usize); 4] = [("direct", 0), ("tokio", 2), ("rayon", 2), ("rayon", 4)];
pub(super) fn run(progress: &Sender<&'static str>, smoke: bool, profile: bool) {
    // SAFETY: the existing Cargo build owns the unchanged native Scheme ABI.
    let program = unsafe {
        IsolatedProgram::from_image(
            gerbil_scheme_qualification::owner_instance_image(),
            c"gerbil_owner_orders_call",
        )
    }
    .unwrap();
    let mut pool = InstancePool::start_observed(
        &program,
        InstanceConfig {
            owners: 4,
            queue_capacity: 128,
            maximum_request_bytes: 16 * 1024 * 1024,
            maximum_reply_bytes: 16 * 1024 * 1024,
            startup_timeout: GATE,
        },
        |_| observed(progress, "focused native owner completed setup"),
    )
    .unwrap();
    observed(progress, "focused native owners initialized");
    let states: Vec<_> = pool.owners().iter().map(|owner| owner.state).collect();
    println!(
        "{}",
        json!({"event": "flow_configuration", "states": states, "owners": 4,
        "window": 128, "batch": 1, "candidate_workers": 2, "profile": profile,
        "candidate": "owner-flow", "candidate_spawned_request_tasks": 0,
        "credit_scope": "invocation, not global across concurrent callers"})
    );
    for (mode, workers) in CONTROLS.into_iter().chain([("owner-flow", 2)]) {
        let executor = Executor::start(mode, workers, profile, false);
        contracts(&pool, &executor, progress);
        executor.finish();
    }
    for mixed in [false, true] {
        compare(&pool, progress, &states, (smoke, profile, mixed));
    }
    finish(&mut pool, progress);
}
fn compare(
    pool: &InstancePool,
    progress: &Sender<&'static str>,
    states: &[usize],
    (smoke, profile, mixed): (bool, bool, bool),
) {
    let jobs = if smoke { 128 } else { 1024 };
    let blocks = if smoke {
        1
    } else if profile {
        2
    } else {
        5
    };
    let inputs: Vec<_> = (0..jobs).map(|id| corpus::order(id, mixed)).collect();
    for (mode, workers) in CONTROLS {
        let mut controls = Vec::new();
        let mut candidates = Vec::new();
        for block in 0..blocks {
            for reverse in [false, true] {
                let arms = if reverse {
                    [("owner-flow", 2), (mode, workers)]
                } else {
                    [(mode, workers), ("owner-flow", 2)]
                };
                for (selected, count) in arms {
                    let executor = Executor::start(selected, count, profile, false);
                    sample::run(pool, &executor, &inputs[..128], 1, progress);
                    let value = sample::run(pool, &executor, &inputs, 1, progress);
                    println!(
                        "{}",
                        json!({"event": "flow_sample", "control": mode, "control_workers": workers,
                        "mode": selected, "workers": count, "mixed": mixed, "block": block, "reverse": reverse,
                        "diagnostic": profile, "elapsed_ns": value.elapsed, "native_latency_ns": value.latency,
                        "window_delivery_ns": value.delivery, "window_after_native_ns": value.after_native,
                        "owner_progress_min": value.owner_progress_min})
                    );
                    executor.finish();
                    if selected == "owner-flow" {
                        candidates.push(value);
                    } else {
                        controls.push(value);
                    }
                    assert_eq!(
                        states,
                        pool.owners()
                            .iter()
                            .map(|owner| owner.state)
                            .collect::<Vec<_>>()
                    );
                    observed(
                        progress,
                        "focused A/B retains identical native VM identities",
                    );
                }
            }
        }
        println!(
            "{}",
            json!({"event": "flow_pair", "control": mode, "control_workers": workers, "mixed": mixed})
        );
        sample::emit(
            &controls,
            4,
            (mode, workers, profile, false),
            mixed,
            1,
            smoke,
            jobs,
        );
        sample::emit(
            &candidates,
            4,
            ("owner-flow", 2, profile, false),
            mixed,
            1,
            smoke,
            jobs,
        );
    }
}
