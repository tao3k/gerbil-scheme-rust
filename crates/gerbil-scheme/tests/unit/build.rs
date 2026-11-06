//! Actual C ABI dispatch; the official builder and `GxTest` stay in this process.
#[path = "build/supervisor.rs"]
mod supervisor;
use gerbil_scheme::{GerbilRuntime, PackageCallOutcome};
use gerbil_scheme_sys::{
    GerbilBorrowedUtf8, GerbilStatus, gerbil_scheme_rust_package_build,
    gerbil_scheme_rust_package_clean,
};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn local_package_environment_is_selected_before_loading_build_script() {
    if supervisor::run() {
        return;
    }
    let root = fixture();
    let environment: std::collections::BTreeMap<_, _> = std::env::vars_os().collect();
    assert!(std::env::var_os("GERBIL_PATH").is_none());
    let runtime = initialize();
    // The existing runtime compiler projection lasts until VM teardown. Calls
    // must restore this active-owner state; Drop restores the full caller state.
    let active_environment: std::collections::BTreeMap<_, _> = std::env::vars_os().collect();
    let directory = root.to_str().unwrap();
    let prefix = root.join(".gerbil");
    assert_eq!(
        runtime
            .build_package_cancellable(directory, &AtomicBool::new(true))
            .unwrap(),
        PackageCallOutcome::CancelledBeforeEntry,
    );
    assert!(
        !prefix.exists(),
        "cancelled call must not select/create package environment"
    );
    assert_eq!(runtime.build_package(directory).unwrap(), 0);
    assert!(prefix.join("lib/package-fixture/answer.o1").is_file());
    assert_eq!(
        fs::read_to_string(root.join("build-owner.txt")).unwrap(),
        std::process::id().to_string()
    );
    assert!(
        std::env::vars_os().collect::<std::collections::BTreeMap<_, _>>() == active_environment,
        "package call changed the active owner's environment"
    );
    // Native setup normalizes its current directory (for example /var resolves
    // to /private/var on macOS). Compare the same selected prefix identity.
    let selected_prefix = prefix.canonicalize().unwrap();
    assert_eq!(
        runtime
            .configure_package_paths(
                selected_prefix.to_str().unwrap(),
                selected_prefix.join("lib").to_str().unwrap()
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
    assert_eq!(
        runtime
            .test_package(directory, "t/passing-test.ss")
            .unwrap(),
        0
    );
    assert_ne!(
        runtime
            .test_package(directory, "t/failing-test.ss")
            .unwrap(),
        0
    );
    assert!(
        std::env::vars_os().collect::<std::collections::BTreeMap<_, _>>() == active_environment,
        "package call changed the active owner's environment"
    );
    fs::create_dir(root.join("empty-tests")).unwrap();
    assert_eq!(runtime.test_package(directory, "empty-tests").unwrap(), 70);
    assert_eq!(runtime.clean_package(directory).unwrap(), 0);
    assert!(!prefix.join("lib/package-fixture/answer.o1").exists());
    assert!(prefix.join("lib/unrelated/keep").is_file());
    println!("PACKAGE-STEP runtime cleanup entered");
    drop(runtime);
    println!("PACKAGE-STEP runtime cleanup returned");
    assert_environment_after_close(&environment);
    fs::remove_dir_all(root).unwrap();
    println!(
        "PACKAGE-NATIVE-ENV-OK discovery immutable-prefix failure-restoration cancellation cleanup"
    );
}

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
    assert_eq!(runtime.clean_package(directory).unwrap(), 70);
    assert_eq!(runtime.run_package_main(directory, &["clean"]).unwrap(), 70);
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
    assert_eq!(runtime.build_package(directory).unwrap(), 0);
    clean_and_rebuild(&runtime, directory);
    assert_eq!(
        fs::read_to_string(root.join("build-owner.txt")).unwrap(),
        std::process::id().to_string()
    );
    assert_eq!(
        runtime
            .test_package(directory, "t/passing-test.ss")
            .unwrap(),
        0
    );
    assert_eq!(
        fs::read_to_string(root.join("test-owner.txt")).unwrap(),
        std::process::id().to_string()
    );
    assert_eq!(
        runtime
            .test_package(directory, "t/failing-test.ss")
            .unwrap(),
        42
    );
    assert_eq!(
        runtime
            .test_package(directory, "t/passing-test.ss")
            .unwrap(),
        0
    );
    assert_eq!(
        runtime
            .build_package(root.join("missing").to_str().unwrap())
            .unwrap(),
        70
    );
    assert_eq!(std::env::current_dir().unwrap(), cwd);
    assert_eq!(
        runtime
            .clean_package(root.join("missing").to_str().unwrap())
            .unwrap(),
        70
    );
    entered_cancellation(&runtime, &root);
    assert_eq!(
        runtime
            .test_package(directory, "t/passing-test.ss")
            .unwrap(),
        0
    );
    assert_eq!(
        fs::read_to_string(root.join("test-runs.txt")).unwrap(),
        "3",
        "the three requests must each execute the selected case exactly once"
    );
    assert_eq!(
        fs::read_to_string(root.join("test-imports.txt")).unwrap(),
        "3",
        "each request must evaluate the selected suite module once"
    );
    stage_and_clean(&runtime, &root);
    assert_eq!(std::env::var_os("GERBIL_PATH"), previous_prefix);
    assert_eq!(std::env::var_os("GERBIL_BUILD_CORES"), previous_cores);
    let invalid = [0xffu8];
    assert_directory_rejection(
        GerbilBorrowedUtf8 {
            ptr: invalid.as_ptr().cast(),
            len: invalid.len(),
        },
        GerbilStatus::InvalidValue,
    );
    println!("PACKAGE-STEP runtime cleanup entered");
    drop(runtime);
    println!("PACKAGE-STEP runtime cleanup returned");
    assert_environment_after_close(&environment);
    assert_directory_rejection(directory.into(), GerbilStatus::RuntimeFinalized);
    println!("PACKAGE-C-ABI-CLEANUP-OK owner={}", std::process::id());
    assert_eq!(std::env::var_os("GERBIL_PATH"), previous_prefix);
    assert_eq!(std::env::var_os("GERBIL_BUILD_CORES"), previous_cores);
    fs::remove_dir_all(root).unwrap();
}

fn stage_and_clean(runtime: &GerbilRuntime, root: &std::path::Path) {
    let directory = root.to_str().unwrap();
    println!("PACKAGE-STEP stage entered");
    let stage = root.join("program");
    assert_eq!(
        runtime
            .stage_package_program(
                root.join("answer.ss").to_str().unwrap(),
                stage.to_str().unwrap(),
                &[]
            )
            .unwrap(),
        0
    );
    println!("PACKAGE-STEP stage returned");
    assert_eq!(
        runtime
            .test_package(directory, "t/program-staged-test.ss")
            .unwrap(),
        0
    );
    assert_eq!(runtime.clean_package(directory).unwrap(), 0);
    assert_eq!(
        fs::read_to_string(std::path::Path::new(directory).join("clean-owner.txt")).unwrap(),
        std::process::id().to_string()
    );
    assert_eq!(
        runtime
            .test_package(directory, "t/products-clean-test.ss")
            .unwrap(),
        0
    );
    assert_eq!(
        runtime
            .test_package(directory, "t/program-staged-test.ss")
            .unwrap(),
        0
    );
    assert_eq!(runtime.run_package_main(directory, &["meta"]).unwrap(), 0);
    assert_eq!(
        runtime
            .run_package_main(directory, &["unknown-build-action"])
            .unwrap(),
        70
    );
}

fn assert_directory_rejection(directory: GerbilBorrowedUtf8, expected: GerbilStatus) {
    type Action = unsafe extern "C" fn(GerbilBorrowedUtf8, *mut i32) -> GerbilStatus;
    let actions: [Action; 2] = [
        gerbil_scheme_rust_package_build,
        gerbil_scheme_rust_package_clean,
    ];
    for action in actions {
        let mut code = -123;
        // SAFETY: each test span is backed by a live Rust buffer. Both rejected
        // paths must leave the writable result unchanged without entering Scheme.
        assert_eq!(unsafe { action(directory, &raw mut code) }, expected);
        assert_eq!(code, -123);
    }
}

fn fixture() -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("gerbil-package-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    println!("PACKAGE-FIXTURE={}", root.display());
    let sources = [
        (
            "gerbil.pkg",
            include_str!("../fixtures/package-tools/gerbil.pkg"),
        ),
        (
            "build.ss",
            include_str!("../fixtures/package-tools/build.ss"),
        ),
        (
            "answer.ss",
            include_str!("../fixtures/package-tools/answer.ss"),
        ),
        (
            "package-clean.ss",
            include_str!("../../../../scheme/package-clean.ss"),
        ),
        (
            "t/passing-test.ss",
            include_str!("../fixtures/package-tools/t/passing-test.ss"),
        ),
        (
            "t/failing-test.ss",
            include_str!("../fixtures/package-tools/t/failing-test.ss"),
        ),
        (
            "t/cancel-test.ss",
            include_str!("../fixtures/package-tools/t/cancel-test.ss"),
        ),
        (
            "t/products-built-test.ss",
            include_str!("../fixtures/package-tools/t/products-built-test.ss"),
        ),
        (
            "t/products-clean-test.ss",
            include_str!("../fixtures/package-tools/t/products-clean-test.ss"),
        ),
        (
            "t/program-staged-test.ss",
            include_str!("../fixtures/package-tools/t/program-staged-test.ss"),
        ),
    ];
    for (name, source) in sources {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }
    #[cfg(unix)]
    fs::set_permissions(root.join("build.ss"), fs::Permissions::from_mode(0o755)).unwrap();
    root
}

fn clean_and_rebuild(runtime: &GerbilRuntime, directory: &str) {
    let built = "t/products-built-test.ss";
    let cleaned = "t/products-clean-test.ss";
    assert_eq!(runtime.test_package(directory, built).unwrap(), 0);
    assert_eq!(
        runtime
            .clean_package_cancellable(directory, &AtomicBool::new(true))
            .unwrap(),
        PackageCallOutcome::CancelledBeforeEntry
    );
    assert_eq!(runtime.test_package(directory, built).unwrap(), 0);
    assert_eq!(runtime.clean_package(directory).unwrap(), 0);
    assert_eq!(runtime.test_package(directory, cleaned).unwrap(), 0);
    assert_eq!(runtime.clean_package(directory).unwrap(), 0);
    assert_eq!(runtime.test_package(directory, cleaned).unwrap(), 0);
    assert_eq!(runtime.build_package(directory).unwrap(), 0);
    assert_eq!(runtime.test_package(directory, built).unwrap(), 0);
}

fn initialize() -> GerbilRuntime {
    let start = Instant::now();
    eprintln!("PACKAGE-OWNER initialization entered");
    let runtime = GerbilRuntime::initialize().unwrap_or_else(|failure| {
        unsafe extern "C" {
            fn gerbil_scheme_instance_setup_diagnostic(out: *mut i64) -> i32;
        }
        let mut diagnostic = [0i64; 4];
        // SAFETY: the C helper writes exactly four diagnostic words after setup.
        let status = unsafe { gerbil_scheme_instance_setup_diagnostic(diagnostic.as_mut_ptr()) };
        panic!("runtime setup failed: {failure:?}; diagnostic={diagnostic:?}; status={status}");
    });
    eprintln!(
        "PACKAGE-OWNER initialization returned elapsed={:?}",
        start.elapsed()
    );
    runtime
}

fn entered_cancellation(runtime: &GerbilRuntime, root: &std::path::Path) {
    let directory = root.to_str().unwrap();
    let cancellation = AtomicBool::new(false);
    std::thread::scope(|scope| {
        let root = &root;
        let cancellation = &cancellation;
        let observer = scope.spawn(move || {
            let start = Instant::now();
            while !root.join("entered").exists() || !root.join("output-observed").exists() {
                assert!(
                    start.elapsed() < Duration::from_secs(15),
                    "Scheme entry/output was not observed before the unchanged gate"
                );
                std::thread::sleep(Duration::from_millis(1));
            }
            cancellation.store(true, Ordering::Release);
            fs::write(root.join("release"), "drain").unwrap();
        });
        assert_eq!(
            runtime
                .test_package_cancellable(directory, "t/cancel-test.ss", cancellation)
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
