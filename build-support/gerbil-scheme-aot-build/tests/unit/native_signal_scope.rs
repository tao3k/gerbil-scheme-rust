// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use super::build_signal_archive;
use std::{os::unix::process::ExitStatusExt, path::Path, process::Output};
use tokio::process::Command;

async fn bounded(command: &mut Command) -> Output {
    command.kill_on_drop(true);
    tokio::time::timeout(std::time::Duration::from_secs(15), command.output())
        .await
        .expect("native signal control exceeded 15 seconds")
        .expect("run native signal control")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn native_bundle_preserves_other_threads_and_host_signal_updates() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = std::env::temp_dir().join(format!(
        "gerbil-signal-bundle-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    for (source, object, binding) in [
        ("host_signal_process_scope_test.c", "runtime.o", "41"),
        ("host_signal_action_test.c", "gambit.o", "41"),
        ("host_signal_cleanup_test.c", "cleanup.o", "41"),
        ("host_signal_binding_test.c", "host-binding.o", "41"),
        ("host_signal_binding_test.c", "fallback-binding.o", "99"),
    ] {
        let output = bounded(
            Command::new("cc")
                .args(["-std=c11", "-pthread", "-Wall", "-Wextra", "-Werror", "-I"])
                .arg(workspace.join("ffi"))
                .arg(format!("-DGERBIL_TEST_PROGRAM_BINDING={binding}"))
                .arg("-c")
                .arg(workspace.join("ffi").join(source))
                .arg("-o")
                .arg(root.join(object)),
        )
        .await;
        assert!(
            output.status.success(),
            "compile signal fixture: {output:?}"
        );
    }
    let library = root.join("libgambit.a");
    let output = bounded(
        Command::new("ar")
            .arg("rcs")
            .arg(&library)
            .arg(root.join("gambit.o"))
            .arg(root.join("cleanup.o"))
            .arg(root.join("fallback-binding.o")),
    )
    .await;
    assert!(output.status.success(), "fixture archive: {output:?}");
    let scoped = root.join("scoped");
    let target = format!("{}-unknown-linux-gnu", std::env::consts::ARCH);
    let mut build = cc::Build::new();
    build.target(&target).host(&target).opt_level(0);
    build_signal_archive(&library, &scoped, build).expect("prepare paired native archive");
    for guarded in [true, false] {
        let archive = if guarded {
            scoped.join("libgerbil_scheme_rust_gambit.a")
        } else {
            library.clone()
        };
        let binary = root.join(if guarded {
            "guarded.test"
        } else {
            "owner-only.test"
        });
        // No wrapping flag in the final host link: the archive owns its scope.
        // Pull the caller before the owner, as a program rlib does in Cargo.
        // The owner must not carry another strong copy of the caller's functions.
        let output = bounded(
            Command::new("cc")
                .arg("-pthread")
                .arg("-Wl,--undefined=gerbil_test_native_signal_cleanup")
                .arg(root.join("host-binding.o"))
                .arg(&archive)
                .arg(root.join("runtime.o"))
                .arg(&archive)
                .arg("-o")
                .arg(&binary),
        )
        .await;
        assert!(output.status.success(), "host link: {output:?}");
        let output = bounded(&mut Command::new(&binary)).await;
        if guarded {
            assert!(output.status.success(), "process scope: {output:?}");
            assert!(
                String::from_utf8(output.stdout)
                    .unwrap()
                    .contains("HOST-SIGNAL-PROCESS-SCOPE-OK")
            );
        } else {
            // The identical owner mask still leaves a concurrent writer exposed.
            assert_eq!(
                output.status.signal(),
                Some(13),
                "owner-only control: {output:?}"
            );
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}
