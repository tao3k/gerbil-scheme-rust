// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use super::build_signal_archive;
use std::{path::Path, process::Output};
use tokio::process::Command;

async fn bounded(command: &mut Command) -> Output {
    command.kill_on_drop(true);
    tokio::time::timeout(std::time::Duration::from_secs(15), command.output())
        .await
        .expect("native child ownership control exceeded 15 seconds")
        .expect("run native child ownership control")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn paired_archive_preserves_host_child_wait_and_concurrent_policy() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = std::env::temp_dir().join(format!(
        "gerbil-child-scope-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let compiler = std::env::var_os("CC").unwrap_or_else(|| "cc".into());
    for (source, object) in [
        ("host_signal_action_test.c", "action.o"),
        ("host_child_signal_caller_test.c", "caller.o"),
    ] {
        let output = bounded(
            Command::new(&compiler)
                .args([
                    "-std=c11",
                    "-D_DARWIN_C_SOURCE",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-c",
                ])
                .arg(workspace.join("ffi").join(source))
                .arg("-o")
                .arg(root.join(object)),
        )
        .await;
        assert!(output.status.success(), "compile SDK control: {output:?}");
    }
    let original = root.join("libgambit.a");
    let output = bounded(
        Command::new("ar")
            .arg("rcs")
            .arg(&original)
            .arg(root.join("action.o"))
            .arg(root.join("caller.o")),
    )
    .await;
    assert!(output.status.success(), "archive SDK control: {output:?}");
    let scoped = root.join("scoped");
    let mut build = cc::Build::new();
    let target = if cfg!(target_os = "macos") {
        format!("{}-apple-darwin", std::env::consts::ARCH)
    } else {
        format!("{}-unknown-linux-gnu", std::env::consts::ARCH)
    };
    build.target(&target).host(&target).opt_level(0);
    build_signal_archive(&original, &scoped, build).expect("scope paired archive");
    for guarded in [true, false] {
        let binary = root.join(if guarded { "guarded" } else { "unbound" });
        let output = bounded(
            Command::new(&compiler)
                .args([
                    "-std=c11",
                    "-D_DARWIN_C_SOURCE",
                    "-pthread",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-I",
                ])
                .arg(workspace.join("ffi"))
                .arg(workspace.join("ffi/host_child_scope_test.c"))
                .arg(if guarded {
                    scoped.join("libgerbil_scheme_rust_gambit.a")
                } else {
                    original.clone()
                })
                .arg("-o")
                .arg(&binary),
        )
        .await;
        assert!(output.status.success(), "link child control: {output:?}");
        let output = bounded(&mut Command::new(&binary)).await;
        assert_eq!(
            output.status.code(),
            Some(if guarded { 0 } else { 42 }),
            "{output:?}"
        );
        let marker = if guarded {
            "HOST-CHILD-SCOPE-OK"
        } else {
            "HOST-CHILD-POLICY-CHANGED"
        };
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(marker),
            "{output:?}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
