//! Actual launcher forwarding, immutable publication and failure propagation.
use super::prepare_gsc_progress_launcher;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/unit/scenarios/package-gsc-launcher/inputs/gsc'fixture")
}

fn output() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    for _ in 0..32 {
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "package-launcher-{}-{nonce}-{sequence}",
            std::process::id()
        ));
        match std::fs::create_dir(&root) {
            Ok(()) => return root,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("create launcher fixture directory: {error}"),
        }
    }
    panic!("create unique launcher fixture directory");
}

#[test]
fn launcher_preserves_arguments_and_nonzero_exit_with_quoted_compiler_path() {
    let root = output();
    let launcher = prepare_gsc_progress_launcher(&fixture(), &root, true).unwrap();
    let receipt = root.join("args");
    let status = Command::new(&launcher)
        .args(["-target", "C", "source file.scm"])
        .env("GSC_ARGV_RECEIPT", &receipt)
        .env("GSC_EXIT_STATUS", "7")
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(7));
    assert_eq!(
        std::fs::read_to_string(receipt).unwrap(),
        "-cc-options\n-Q -fopt-info-all\n-target\nC\nsource file.scm\n"
    );
    assert_eq!(
        prepare_gsc_progress_launcher(&fixture(), &root, true).unwrap(),
        launcher
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn quiet_or_non_gcc_package_keeps_original_compiler() {
    let root = output();
    assert_eq!(
        prepare_gsc_progress_launcher(&fixture(), &root, false).unwrap(),
        fixture()
    );
    assert!(root.is_dir());
    let compiler = root.join("gsc");
    std::fs::write(&compiler, "fixture").unwrap();
    std::fs::write(root.join("gambuild-C"), "BUILD_FEATURE_C_COMP=\"clang\"\n").unwrap();
    assert_eq!(
        prepare_gsc_progress_launcher(&compiler, &root.join("launchers"), true).unwrap(),
        compiler
    );
    assert!(!root.join("launchers").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn parallel_publication_executes_only_closed_launcher_inodes() {
    let root = output();
    std::thread::scope(|scope| {
        for index in 0..8 {
            let root = &root;
            scope.spawn(move || {
                let launcher = prepare_gsc_progress_launcher(&fixture(), root, true).unwrap();
                assert!(
                    Command::new(launcher)
                        .arg("input.scm")
                        .env("GSC_ARGV_RECEIPT", root.join(format!("args-{index}")))
                        .status()
                        .unwrap()
                        .success()
                );
            });
        }
    });
    std::fs::remove_dir_all(root).unwrap();
}
