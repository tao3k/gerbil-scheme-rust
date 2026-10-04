//! Actual native process forwarding and failure propagation.
use super::run_process;
use std::process::Command;

#[test]
fn quiet_failure_retains_both_diagnostic_streams() {
    let error = run_process(
        Command::new("sh").args(["-c", "echo output; echo diagnostic >&2; exit 7"]),
        "compile fixture",
        false,
    )
    .unwrap_err();
    assert!(error.contains("compile fixture"));
    assert!(error.contains("output"));
    assert!(error.contains("diagnostic"));
    assert!(error.contains('7'));
}

#[test]
fn live_failure_preserves_nonzero_status() {
    let error = run_process(
        Command::new("sh").args(["-c", "echo diagnostic >&2; exit 7"]),
        "compile fixture",
        true,
    )
    .unwrap_err();
    assert!(error.contains("compile fixture"));
    assert!(error.contains('7'));
    assert!(error.contains("streamed"));
}

#[test]
#[ignore = "six-second real child-output regression; run under five-second idle supervision"]
fn live_child_forwards_real_events_before_exit() {
    run_process(
        Command::new("sh").args([
            "-c",
            "echo fixture-start; sleep 3; echo fixture-middle; sleep 3; echo fixture-end",
        ]),
        "compile fixture",
        true,
    )
    .unwrap();
}

#[test]
fn verbose_gambit_runtime_options_precede_compiler_options_and_preserve_heap_defaults() {
    let quiet = super::gambit_progress_command_with_mode(std::path::Path::new("gsc"), false);
    assert_eq!(quiet.get_args().count(), 0);
    let mut live = super::gambit_progress_command_with_mode(std::path::Path::new("gsc"), true);
    live.args(["-c", "input.scm"]);
    let args = live
        .get_args()
        .map(|arg| arg.to_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(args, ["-:1n,2n,d5qQ", "-verbose", "-c", "input.scm"]);
}
