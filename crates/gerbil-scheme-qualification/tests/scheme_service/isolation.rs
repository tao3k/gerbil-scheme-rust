//! Isolate the process-wide census while other native cases remain concurrent.
// A process-wide FD census must not include resources from parallel libtest cases.
// The child keeps the same runtime workers, owner population and progress guard.
pub(super) fn isolate_fd_case(name: &'static str) -> bool {
    const KEY: &str = "GERBIL_SCHEME_FD_CASE";
    if std::env::var(KEY).as_deref() == Ok(name) {
        return false;
    }
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env(KEY, name)
        .status()
        .expect("start isolated native FD qualification");
    assert!(
        status.success(),
        "isolated native FD qualification failed: {status}"
    );
    true
}
