//! Actual C ABI dispatch; the official builder and `GxTest` stay in this process.
#[path = "package_tools/supervisor.rs"]
mod supervisor;
use gerbil_scheme::{GerbilRuntime, PackageCallOutcome};
use gerbil_scheme_sys::{GerbilBorrowedUtf8, GerbilStatus, gerbil_scheme_rust_package_build};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn build_test_and_failures_share_the_existing_owner() {
    if supervisor::run() {
        return;
    }
    let root = fixture();
    let environment: std::collections::BTreeMap<_, _> = std::env::vars_os().collect();
    let previous_prefix = std::env::var_os("GERBIL_PATH");
    let previous_cores = std::env::var_os("GERBIL_BUILD_CORES");
    let runtime = initialize();
    let directory = root.to_str().unwrap();
    configure(&runtime, &root);
    let cancelled = AtomicBool::new(true);
    assert_eq!(
        runtime
            .build_package_cancellable(directory, &cancelled)
            .unwrap(),
        PackageCallOutcome::CancelledBeforeEntry
    );
    assert!(!root.join("build-owner.txt").exists());
    let cwd = std::env::current_dir().unwrap();
    assert_compiled_package(&runtime, &root);
    assert_eq!(
        fs::read_to_string(root.join("build-owner.txt")).unwrap(),
        std::process::id().to_string()
    );
    assert_eq!(
        runtime.test_package(directory, "passing-test.ss").unwrap(),
        0
    );
    assert_eq!(
        fs::read_to_string(root.join("test-owner.txt")).unwrap(),
        std::process::id().to_string()
    );
    assert_eq!(
        runtime.test_package(directory, "failing-test.ss").unwrap(),
        42
    );
    assert_eq!(
        runtime.test_package(directory, "passing-test.ss").unwrap(),
        0
    );
    assert_eq!(
        runtime
            .build_package(root.join("missing").to_str().unwrap())
            .unwrap(),
        70
    );
    assert_eq!(std::env::current_dir().unwrap(), cwd);
    entered_cancellation(&runtime, &root);
    assert_eq!(
        runtime.test_package(directory, "passing-test.ss").unwrap(),
        0
    );
    println!("PACKAGE-STEP stage entered");
    let stage = root.join("program");
    assert_eq!(
        runtime
            .stage_package_program(directory, "answer.ss", stage.to_str().unwrap())
            .unwrap(),
        0
    );
    println!("PACKAGE-STEP stage returned");
    assert!(stage.join("program.json").is_file());
    assert!(stage.join("program__exe.scm").is_file());
    assert_eq!(runtime.run_package_main(directory, &["meta"]).unwrap(), 0);
    assert_eq!(
        runtime
            .run_package_main(directory, &["unknown-build-action"])
            .unwrap(),
        70
    );
    assert_eq!(std::env::var_os("GERBIL_PATH"), previous_prefix);
    assert_eq!(std::env::var_os("GERBIL_BUILD_CORES"), previous_cores);
    let invalid = [0xffu8];
    let mut result = -123;
    // SAFETY: readable span and writable output; invalid UTF-8 must be rejected.
    let status = unsafe {
        gerbil_scheme_rust_package_build(
            GerbilBorrowedUtf8 {
                ptr: invalid.as_ptr().cast(),
                len: invalid.len(),
            },
            &raw mut result,
        )
    };
    assert_eq!(status, GerbilStatus::InvalidValue);
    assert_eq!(result, -123);
    drop(runtime);
    assert_environment_after_close(&environment);
    // SAFETY: valid transport; finalized runtime must reject without altering the result.
    assert_eq!(
        unsafe { gerbil_scheme_rust_package_build(directory.into(), &raw mut result) },
        GerbilStatus::RuntimeFinalized
    );
    assert_eq!(result, -123);
    println!("PACKAGE-C-ABI-CLEANUP-OK owner={}", std::process::id());
    assert_eq!(std::env::var_os("GERBIL_PATH"), previous_prefix);
    assert_eq!(std::env::var_os("GERBIL_BUILD_CORES"), previous_cores);
}

fn fixture() -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("gerbil-package-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    println!("PACKAGE-FIXTURE={}", root.display());
    fs::write(root.join("gerbil.pkg"), "(package: package-fixture)\n").unwrap();
    fs::write(root.join("build.ss"), "(import :std/build-script)\n(call-with-output-file (list path: \"build-owner.txt\" truncate: #t) (lambda (p) (write (##os-getpid) p)))\n(defbuild-script '(\"answer\"))\n").unwrap();
    fs::write(
        root.join("answer.ss"),
        "(export answer answer-literal main)\n(def (answer) 42)\n(defrule (answer-literal) 42)\n(def (main . _) (void))\n",
    )
    .unwrap();
    fs::write(root.join("passing-test.ss"), "(import :std/test :package-fixture/answer)\n(export passing-test)\n(def passing-test (test-suite \"C ABI package owner\" (test-case \"same process\" (call-with-output-file (list path: \"test-owner.txt\" truncate: #t) (lambda (p) (write (##os-getpid) p))) (check-equal? (answer) 42) (check-equal? (answer-literal) 42))))\n").unwrap();
    fs::write(root.join("failing-test.ss"), "(import :std/test :package-fixture/answer)\n(export failing-test)\n(def failing-test (test-suite \"Retained failure\" (test-case \"negative control\" (check-equal? (answer) 41))))\n").unwrap();
    root
}

fn assert_compiled_package(runtime: &GerbilRuntime, root: &std::path::Path) {
    assert_eq!(runtime.build_package(root.to_str().unwrap()).unwrap(), 0);
    for artifact in ["answer.ssi", "answer.o1", "answer~0.o1", "answer~1.o1"] {
        let path = root.join("prefix/lib/package-fixture").join(artifact);
        assert!(
            fs::metadata(&path).unwrap().len() > 0,
            "missing compiled artifact: {}",
            path.display()
        );
    }
}

fn initialize() -> GerbilRuntime {
    GerbilRuntime::initialize().unwrap_or_else(|failure| {
        unsafe extern "C" {
            fn gerbil_scheme_instance_setup_diagnostic(out: *mut i64) -> i32;
        }
        let mut diagnostic = [0i64; 4];
        // SAFETY: the C helper writes exactly four diagnostic words after setup.
        let status = unsafe { gerbil_scheme_instance_setup_diagnostic(diagnostic.as_mut_ptr()) };
        panic!("runtime setup failed: {failure:?}; diagnostic={diagnostic:?}; status={status}");
    })
}

fn entered_cancellation(runtime: &GerbilRuntime, root: &std::path::Path) {
    let directory = root.to_str().unwrap();
    fs::write(root.join("cancel-test.ss"), "(import :std/test)\n(export cancel-test)\n(def cancel-test (test-suite \"Cancellation drain\" (test-case \"entered\" (call-with-output-file \"entered\" (lambda (p) (display \"entered\" p))) (let loop () (unless (file-exists? \"release\") (thread-sleep! 0.001) (loop))) (check #t => #t))))\n").unwrap();
    let cancellation = AtomicBool::new(false);
    std::thread::scope(|scope| {
        let root = &root;
        let cancellation = &cancellation;
        let observer = scope.spawn(move || {
            let start = Instant::now();
            while !root.join("entered").exists() {
                assert!(
                    start.elapsed() < Duration::from_secs(15),
                    "Scheme did not enter before the unchanged gate"
                );
                std::thread::sleep(Duration::from_millis(1));
            }
            cancellation.store(true, Ordering::Release);
            fs::write(root.join("release"), "drain").unwrap();
        });
        assert_eq!(
            runtime
                .test_package_cancellable(directory, "cancel-test.ss", cancellation)
                .unwrap(),
            PackageCallOutcome::Completed {
                code: 0,
                cancelled: true
            }
        );
        observer.join().unwrap();
    });
}

fn configure(runtime: &GerbilRuntime, root: &std::path::Path) {
    let directory = root.to_str().unwrap();
    let prefix = root.join("prefix");
    assert_eq!(
        runtime
            .configure_package_paths(
                prefix.to_str().unwrap(),
                prefix.join("lib").to_str().unwrap()
            )
            .unwrap(),
        0
    );
    assert_eq!(
        runtime
            .configure_package_paths(root.join("other-prefix").to_str().unwrap(), directory)
            .unwrap(),
        70
    );
}

fn assert_environment_after_close(
    expected: &std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>,
) {
    // Ordinary host allocations must remain safe after VM cleanup. Cover the
    // environment pointer-table allocation sizes without dereferencing a saved pointer.
    let retained: Vec<_> = (expected.len()..expected.len() * 2 + 16)
        .map(|size| vec![0x55u8; size * std::mem::size_of::<usize>()])
        .collect();
    assert!(
        &std::env::vars_os().collect::<std::collections::BTreeMap<_, _>>() == expected,
        "host environment changed after VM cleanup"
    );
    std::hint::black_box(retained);
}
