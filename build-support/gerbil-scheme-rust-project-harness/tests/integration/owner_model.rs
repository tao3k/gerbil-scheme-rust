//! Finite protocol checks, separate from native execution acceptance.
use std::{
    cell::RefCell,
    collections::VecDeque,
    path::PathBuf,
    process::{Command, Output},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

struct Check {
    scenario: bool,
    work: Box<dyn FnOnce() + Send>,
}
thread_local! {
    static PLANNING: RefCell<Option<Vec<Check>>> = const { RefCell::new(None) };
}
struct FinishPlanning;
impl Drop for FinishPlanning {
    fn drop(&mut self) {
        PLANNING.with(|plan| plan.replace(None));
    }
}
fn collect_success(arguments: &[&str]) -> bool {
    PLANNING.with(|plan| {
        let mut plan = plan.borrow_mut();
        let Some(plan) = plan.as_mut() else {
            return false;
        };
        let scenario = arguments[0] == "test";
        let owned: Vec<_> = arguments.iter().map(|arg| (*arg).to_owned()).collect();
        plan.push(Check {
            scenario,
            work: Box::new(move || {
                let args: Vec<_> = owned.iter().map(String::as_str).collect();
                success(&args);
            }),
        });
        true
    })
}
fn collect_mutation(arguments: [&str; 5]) -> bool {
    PLANNING.with(|plan| {
        let mut plan = plan.borrow_mut();
        let Some(plan) = plan.as_mut() else {
            return false;
        };
        let owned = arguments.map(str::to_owned);
        plan.push(Check {
            scenario: false,
            work: Box::new(move || {
                mutation_counterexample(&owned[0], &owned[1], &owned[2], &owned[3], &owned[4]);
            }),
        });
        true
    })
}
fn protocol_plan(source: &str) -> Vec<Check> {
    PLANNING.with(|plan| {
        let mut plan = plan.borrow_mut();
        assert!(plan.is_none(), "nested model check collection");
        *plan = Some(Vec::new());
    });
    let _finish = FinishPlanning;
    lifecycle_protocol(source);
    capacity_protocol(source);
    architecture_protocol(source);
    shared_protocol(source);
    owner_completion_scenarios_and_control();
    progress_protocol(source);
    PLANNING.with(|plan| plan.borrow_mut().take().unwrap())
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn run(arguments: &[&str]) -> Output {
    eprintln!("QUINT-CHECK {arguments:?}");
    let started = std::time::Instant::now();
    let output = Command::new("quint")
        .args(super::owner_model_source::arguments(arguments))
        .current_dir(root())
        .output()
        .expect("installed Quint CLI");
    eprintln!(
        "QUINT-DONE {arguments:?} {:?} {:?}",
        output.status.code(),
        started.elapsed()
    );
    output
}
fn success(arguments: &[&str]) {
    if collect_success(arguments) {
        return;
    }
    let output = run(arguments);
    assert!(
        output.status.success(),
        "{arguments:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
#[ignore = "explicit Quint gate; ordinary Rust tests do not require the CLI"]
fn owner_protocol_scenarios_and_controls() {
    let source = "specs/owner_lifecycle.qnt";
    success(&["typecheck", source]);
    let model = std::sync::Arc::new(super::owner_model_source::ModelSource::prepare(&root()));
    let mut checks = protocol_plan(source);
    // Put long scenario commands ahead of shorter simulations. Each immutable
    // check binds to a worker only at actual claim; no static shard can strand
    // pending work while a peer is idle.
    checks.sort_by_key(|check| !check.scenario);
    let total = checks.len();
    let shard = std::env::var("GERBIL_OWNER_MODEL_SHARD")
        .ok()
        .map(|value| value.parse::<usize>().expect("integer protocol shard"));
    if let Some(shard) = shard {
        assert!(shard < 2, "protocol shard must be zero or one");
        checks = checks
            .into_iter()
            .enumerate()
            .filter_map(|(index, check)| (index % 2 == shard).then_some(check))
            .collect();
    }
    let count = checks.len();
    eprintln!("QUINT-COVERAGE total={total} shard={shard:?} selected={count}");
    let mut checks = VecDeque::from(checks);
    // Quint lazily installs its Rust evaluator without a cross-process lock.
    // Execute one real check before concurrent invocations can observe a
    // partially downloaded archive. It remains part of the original checklist.
    let first = checks.pop_front().expect("nonempty protocol checklist");
    super::owner_model_source::scoped(&model, || (first.work)());
    let queue = Mutex::new(checks);
    let completed = AtomicUsize::new(1);
    eprintln!("QUINT-PLAN jobs={count} workers=3 initialized=1");
    std::thread::scope(|scope| {
        for _ in 0..3 {
            scope.spawn(|| {
                super::owner_model_source::scoped(&model, || {
                    loop {
                        let check = queue.lock().unwrap().pop_front();
                        let Some(check) = check else { break };
                        (check.work)();
                        completed.fetch_add(1, Ordering::Relaxed);
                    }
                })
            });
        }
    });
    assert_eq!(completed.load(Ordering::Relaxed), count);
    eprintln!("QUINT-PLAN-DONE jobs={count}");
}
fn lifecycle_protocol(source: &str) {
    for (domain, seed) in [
        ("owner_lifecycle_small", "1"),
        ("owner_lifecycle_wide", "42"),
    ] {
        success(&["test", source, "--main", domain, "--seed", "1"]);
        success(&[
            "run",
            source,
            "--main",
            domain,
            "--invariant",
            "safety",
            "--seed",
            seed,
            "--max-samples",
            "1000",
            "--max-steps",
            "64",
            "--verbosity",
            "1",
        ]);
    }
    for mutation in [
        "unsafePublicationStep",
        "unsafeExecutorStep",
        "unsafeJoinStep",
        "unsafeForeignStep",
        "unsafeCodeReleaseStep",
        "unsafeCreditReturnStep",
        "unsafeProducerExitStep",
        "unsafeParkStep",
        "unsafeServiceRootStep",
    ] {
        mutation_counterexample(source, "owner_lifecycle_small", "init", mutation, "safety");
    }
    success(&[
        "run",
        source,
        "--main",
        "owner_lifecycle_small",
        "--step",
        "reselectionStep",
        "--invariant",
        "safety",
        "--seed",
        "1",
        "--max-samples",
        "1000",
        "--max-steps",
        "64",
        "--verbosity",
        "1",
    ]);
    mutation_counterexample(
        source,
        "owner_lifecycle_small",
        "init",
        "unsafeOwnerMoveStep",
        "safety",
    );
    success(&[
        "run",
        source,
        "--main",
        "owner_lifecycle_small",
        "--step",
        "streamStep",
        "--invariant",
        "safety",
        "--seed",
        "1",
        "--max-samples",
        "1000",
        "--max-steps",
        "64",
        "--verbosity",
        "1",
    ]);
    mutation_counterexample(
        source,
        "owner_lifecycle_small",
        "producerMutationInit",
        "unsafeProducerActiveStep",
        "safety",
    );
    success(&[
        "run",
        source,
        "--main",
        "owner_lifecycle_small",
        "--step",
        "poolStep",
        "--invariant",
        "safety",
        "--seed",
        "1",
        "--max-samples",
        "1000",
        "--max-steps",
        "64",
        "--verbosity",
        "1",
    ]);
    mutation_counterexample(
        source,
        "owner_lifecycle_small",
        "poolRegistrationMutationInit",
        "unsafePoolRegistrationStep",
        "safety",
    );
}
fn architecture_protocol(source: &str) {
    for domain in ["gerbil_architecture_small", "gerbil_architecture_wide"] {
        success(&[
            "test",
            source,
            "--main",
            domain,
            "--match",
            "architecture.*Test",
            "--seed",
            "1",
        ]);
    }
}
fn capacity_protocol(source: &str) {
    for step in [
        "capacityProtocolStep",
        "shortPoolStep",
        "selectiveShortStep",
    ] {
        success(&[
            "run",
            source,
            "--main",
            "owner_lifecycle_small",
            "--step",
            step,
            "--invariant",
            "safety",
            "--seed",
            "1",
            "--max-samples",
            "1000",
            "--max-steps",
            "64",
            "--verbosity",
            "1",
        ]);
    }
}
#[test]
#[ignore = "explicit targeted native progress model gate"]
fn owner_progress_scenarios_and_control() {
    let source = "specs/owner_lifecycle.qnt";
    success(&["typecheck", source]);
    for domain in ["owner_lifecycle_small", "owner_lifecycle_wide"] {
        success(&[
            "test",
            source,
            "--main",
            domain,
            "--match",
            "progress.*Test",
            "--seed",
            "1",
        ]);
    }
    progress_protocol(source);
}
fn progress_protocol(source: &str) {
    success(&[
        "run",
        source,
        "--main",
        "owner_lifecycle_small",
        "--step",
        "progressStep",
        "--invariant",
        "progressSafety",
        "--seed",
        "42",
        "--max-samples",
        "1000",
        "--max-steps",
        "64",
        "--verbosity",
        "1",
    ]);
    mutation_counterexample(
        source,
        "owner_lifecycle_small",
        "init",
        "unsafeProgressReleaseStep",
        "progressSafety",
    );
}
fn symbolic(bound: &str) {
    // Keep solver output, including incomplete attempts, outside the checkout.
    let directory = tempfile::Builder::new()
        .prefix("gerbil-owner-symbolic-")
        .tempdir()
        .expect("external checker output directory")
        .keep();
    println!("Symbolic output: {}", directory.display());
    let status = Command::new("quint")
        .arg("verify")
        .arg(root().join("specs/owner_lifecycle.qnt"))
        .args([
            "--main",
            "owner_lifecycle_small",
            "--invariant",
            "safety",
            "--max-steps",
            bound,
            "--apalache-config",
        ])
        .arg(root().join("specs/apalache.json"))
        .current_dir(directory)
        .status()
        .expect("installed Quint CLI");
    assert!(
        status.success(),
        "symbolic bound {bound} did not complete successfully"
    );
}
#[test]
#[ignore = "explicit symbolic target; sampled checks do not substitute"]
fn owner_protocol_symbolic_12() {
    symbolic("12");
}
#[test]
#[ignore = "separate smaller bound; does not admit the 12-transition target"]
fn owner_protocol_symbolic_8() {
    symbolic("8");
}

#[test]
#[ignore = "explicit targeted completion model gate"]
fn owner_completion_scenarios_and_control() {
    let source = "specs/owner_lifecycle.qnt";
    success(&["typecheck", source]);
    completion_sharing(source);
    for domain in ["service_completion_small", "service_completion_wide"] {
        completion_domain(source, domain);
    }
    client_progress_scenarios(source);
    native_cycle_scenarios(source);
    for domain in ["service_completion_small", "service_completion_wide"] {
        prepared_cycle_control(source, domain);
    }
}
fn completion_sharing(source: &str) {
    for domain in ["service_sharing_small", "service_sharing_wide"] {
        success(&[
            "test",
            source,
            "--main",
            domain,
            "--match",
            "completion.*Test",
            "--seed",
            "1",
        ]);
        success(&[
            "run",
            source,
            "--main",
            domain,
            "--init",
            "completionStart",
            "--step",
            "sharedStep",
            "--invariant",
            "sharedSafety",
            "--seed",
            "42",
            "--max-samples",
            "1000",
            "--max-steps",
            "64",
            "--verbosity",
            "1",
        ]);
        mutation_counterexample(
            source,
            domain,
            "completionReadyStart",
            "completionMutationStep",
            "sharedSafety",
        );
    }
}
fn completion_domain(source: &str, domain: &str) {
    success(&[
        "test",
        source,
        "--main",
        domain,
        "--match",
        "(streamingJoint|preparation|completionService|arrivalClock|clientTurn|delivery|claimProgress|publication|client|cross|native|readyCycle|executionCheckpoint|directed|registration|reclamation|completion|coalesced|runtime|joint|workBudget|workPlan|leasePlan|activation|preparedCycle|pipeline).*Test",
        "--seed",
        "1",
    ]);
    success(&[
        "run",
        source,
        "--main",
        domain,
        "--init",
        "completionCoalescedStart",
        "--step",
        "sharedStep",
        "--invariant",
        "sharedSafety",
        "--seed",
        "42",
        "--max-samples",
        "1000",
        "--max-steps",
        "64",
        "--verbosity",
        "1",
    ]);
    mutation_counterexample(
        source,
        domain,
        "completionLateReadyStart",
        "completionStaleAckMutationStep",
        "sharedSafety",
    );
}
fn prepared_cycle_control(source: &str, domain: &str) {
    mutation_counterexample(
        source,
        domain,
        "turnSpentStart",
        "turnRefundMutationStep",
        "sharedSafety",
    );
    mutation_counterexample(
        source,
        domain,
        "turnStart",
        "turnHeldYieldMutationStep",
        "sharedSafety",
    );
    mutation_counterexample(
        source,
        domain,
        "cyclePendingStart",
        "cycleDoubleSettleMutationStep",
        "sharedSafety",
    );
    mutation_counterexample(
        source,
        domain,
        "activationAliasStart",
        "activationAliasMutationStep",
        "sharedSafety",
    );
    mutation_counterexample(
        source,
        domain,
        "activationEpochStart",
        "activationEpochMutationStep",
        "sharedSafety",
    );
    mutation_counterexample(
        source,
        domain,
        "leasePlanLateStart",
        "leasePlanMutationStep",
        "sharedSafety",
    );
    mutation_counterexample(
        source,
        domain,
        "planLateArrivalStart",
        "planMutationStep",
        "sharedSafety",
    );
    mutation_counterexample(
        source,
        domain,
        "workContendedStart",
        "workMutationStep",
        "sharedSafety",
    );
    mutation_counterexample(
        source,
        domain,
        "jointBulkReadyStart",
        "jointMutationStep",
        "sharedSafety",
    );
    mutation_counterexample(
        source,
        domain,
        "pipelineCreditMutationStart",
        "pipelineCreditMutationStep",
        "sharedSafety",
    );
    mutation_counterexample(
        source,
        domain,
        "preparedCycleHeldStart",
        "preparedCycleOvertakeMutationStep",
        "sharedSafety",
    );
}
fn native_cycle_scenarios(source: &str) {
    let inits = [
        "nativeCycleStart",
        "preparedCycleStart",
        "pipelineCycleStart",
        "jointStart",
        "workBudgetStart",
        "workPlanStart",
        "leasePlanStart",
        "activationStart",
        "cycleAccountingStart",
        "turnStart",
    ];
    for domain in ["service_completion_small", "service_completion_wide"] {
        for init in inits {
            success(&[
                "run",
                source,
                "--main",
                domain,
                "--init",
                init,
                "--step",
                if init == "turnStart" {
                    "turnStep"
                } else if init == "cycleAccountingStart" {
                    "cycleAccountingStep"
                } else if init == "activationStart" {
                    "activationSharedStep"
                } else {
                    "sharedStep"
                },
                "--invariant",
                "sharedSafety",
                "--seed",
                "42",
                "--max-samples",
                "1000",
                "--max-steps",
                "64",
                "--verbosity",
                "1",
            ]);
        }
        mutation_counterexample(
            source,
            domain,
            "nativeCycleTransferredStart",
            "nativeCycleEarlyCreditMutationStep",
            "sharedSafety",
        );
    }
}
fn shared_protocol(source: &str) {
    for (domain, seed) in [
        ("service_sharing_small", "1"),
        ("service_sharing_wide", "42"),
    ] {
        success(&["test", source, "--main", domain, "--seed", "1"]);
        for init in ["currentInit", "sharedInit"] {
            success(&[
                "run",
                source,
                "--main",
                domain,
                "--init",
                init,
                "--step",
                "sharedStep",
                "--invariant",
                "sharedSafety",
                "--seed",
                seed,
                "--max-samples",
                "1000",
                "--max-steps",
                "64",
                "--verbosity",
                "1",
            ]);
        }
    }
    success(&[
        "test",
        source,
        "--main",
        "service_routing_isolated",
        "--seed",
        "1",
    ]);
    for mutation in [
        "boundaryMutationStep",
        "cpuMutationStep",
        "feedMutationStep",
    ] {
        mutation_counterexample(
            source,
            "service_sharing_small",
            "mutationStart",
            mutation,
            "sharedSafety",
        );
    }
}
fn mutation_counterexample(
    source: &str,
    domain: &str,
    init: &str,
    mutation: &str,
    invariant: &str,
) {
    if collect_mutation([source, domain, init, mutation, invariant]) {
        return;
    }
    let output = run(&[
        "run",
        source,
        "--main",
        domain,
        "--init",
        init,
        "--step",
        mutation,
        "--invariant",
        invariant,
        "--seed",
        "1",
        "--max-samples",
        "1000",
        "--max-steps",
        "32",
        "--verbosity",
        "1",
    ]);
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(1), "{mutation}: {diagnostic}");
    assert!(
        diagnostic.contains("error: Invariant violated"),
        "{mutation}: {diagnostic}"
    );
    println!("{mutation}: expected safety counterexample");
}

fn client_progress_scenarios(source: &str) {
    for domain in ["service_completion_small", "service_completion_wide"] {
        success(&[
            "run",
            source,
            "--main",
            domain,
            "--init",
            "claimProgressStart",
            "--step",
            "crossSharedStep",
            "--invariant",
            "sharedSafety",
            "--seed",
            "42",
            "--max-samples",
            "200",
            "--max-steps",
            "64",
            "--verbosity",
            "1",
        ]);
    }
}
