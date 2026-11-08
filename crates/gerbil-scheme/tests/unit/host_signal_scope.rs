// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Real closed-pipe control for the private native cleanup guard.
use std::{os::unix::process::ExitStatusExt, path::Path, process::Output, time::Duration};
use tokio::process::Command;

pub(super) fn check() {
    // Finish child supervision before the one-shot VM changes process signals.
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(check_cleanup());
}

async fn check_cleanup() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = std::env::temp_dir().join(format!("gerbil-host-signals-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    for guarded in [true, false] {
        let binary = root.join(if guarded { "guarded" } else { "unguarded" });
        let mut compiler = Command::new("cc");
        compiler
            .args(["-std=c11", "-pthread", "-Wall", "-Wextra", "-Werror", "-I"])
            .arg(workspace.join("ffi"))
            .arg(workspace.join("ffi/host_signal_scope_test.c"))
            .arg("-o")
            .arg(&binary);
        if !guarded {
            compiler.arg("-DGERBIL_TEST_UNGUARDED_SIGPIPE");
        }
        let compiled = bounded_output(&mut compiler).await;
        assert!(compiled.status.success(), "C fixture: {compiled:?}");
        let output = bounded_output(&mut Command::new(&binary)).await;
        if guarded {
            assert!(output.status.success(), "guarded cleanup: {output:?}");
            assert!(
                String::from_utf8(output.stdout)
                    .unwrap()
                    .contains("HOST-SIGPIPE-SCOPE-OK original-mask original-pending closed-pipe")
            );
        } else {
            assert_eq!(output.status.signal(), Some(13), "control: {output:?}");
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

async fn bounded_output(command: &mut Command) -> Output {
    command.kill_on_drop(true);
    tokio::time::timeout(Duration::from_secs(15), command.output())
        .await
        .expect("native signal fixture exceeded 15 seconds")
        .expect("run native signal fixture")
}
