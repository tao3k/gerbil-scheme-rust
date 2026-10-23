//! Finite protocol checks, separate from native execution acceptance.
use std::{
    path::PathBuf,
    process::{Command, Output},
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn run(arguments: &[&str]) -> Output {
    Command::new("quint")
        .args(arguments)
        .current_dir(root())
        .output()
        .expect("installed Quint CLI")
}
fn success(arguments: &[&str]) {
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
    capacity_protocol(source);
    shared_protocol(source);
    owner_completion_scenarios_and_control();
    progress_protocol(source);
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
    for domain in ["service_completion_small", "service_completion_wide"] {
        success(&[
            "test",
            source,
            "--main",
            domain,
            "--match",
            "(completion|coalesced|nativeCycle).*Test",
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
    native_cycle_scenarios(source);
}
fn native_cycle_scenarios(source: &str) {
    for domain in ["service_completion_small", "service_completion_wide"] {
        success(&[
            "run",
            source,
            "--main",
            domain,
            "--init",
            "nativeCycleStart",
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
