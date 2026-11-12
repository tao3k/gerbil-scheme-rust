// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Owned compiler subprocess diagnostics and compiler-written progress.
use crate::gerbil_command;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[path = "process_runtime.rs"]
mod process_runtime;

// Verbose native builds expose genuine compiler and collector events. The
// collector policy and heap limits remain Gambit's defaults.
pub(super) const GSC_PROGRESS_OPTIONS: [&str; 2] = ["-:1n,2n,d5qQ", "-verbose"];

pub(super) fn native_progress_enabled() -> bool {
    std::env::var("GERBIL_BUILD_VERBOSE")
        .ok()
        .and_then(|level| level.parse::<u8>().ok())
        .is_some_and(|level| level > 0)
}

pub(super) fn gambit_progress_command(program: &Path) -> Command {
    gambit_progress_command_with_mode(program, native_progress_enabled())
}

fn gambit_progress_command_with_mode(program: &Path, live: bool) -> Command {
    let mut command = gerbil_command(program);
    if live {
        command.args(GSC_PROGRESS_OPTIONS);
    }
    command
}

pub(super) fn run_process(
    command: &mut Command,
    operation: &str,
    live: bool,
) -> Result<(), String> {
    let runtime = process_runtime::acquire()?;
    runtime.block_on(run_process_async(command, operation, live, None))
}

async fn run_process_async(
    command: &mut Command,
    operation: &str,
    live: bool,
    artifacts: Option<&[PathBuf]>,
) -> Result<(), String> {
    let placeholder = Command::new(command.get_program());
    let mut native = tokio::process::Command::from(std::mem::replace(command, placeholder));
    native.kill_on_drop(true);
    let result = run_owned_process(&mut native, operation, live, artifacts).await;
    *command = native.into_std();
    result
}

async fn run_owned_process(
    command: &mut tokio::process::Command,
    operation: &str,
    live: bool,
    artifacts: Option<&[PathBuf]>,
) -> Result<(), String> {
    if live {
        // Gambit changes O_NONBLOCK on inherited stdio, which also changes
        // the parent's shared open-file description. Give it private pipes
        // and forward actual bytes immediately through the owned readers.
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("{operation}: {error}"))?;
        let stdout = child.stdout.take().expect("piped compiler stdout");
        let stderr = child.stderr.take().expect("piped compiler stderr");
        let forward = async {
            tokio::try_join!(
                forward_compiler_output(stdout, tokio::io::stdout()),
                forward_compiler_output(stderr, tokio::io::stderr())
            )
            .map(|_| ())
        };
        let forwarded = if let Some(paths) = artifacts {
            tokio::select! {
                result = forward => result,
                result = observe_compiler_artifacts(paths) => result,
            }
        } else {
            forward.await
        };
        if let Err(error) = forwarded {
            // A forwarding/observation refusal cannot leave the owned child
            // running. Kill and reap before acknowledging the failed call.
            let cleanup = child.kill().await;
            let reaped = child.wait().await;
            return Err(format!(
                "{operation}: forward compiler diagnostics: {error}; cleanup={cleanup:?}; reap={reaped:?}"
            ));
        }
        let status = child
            .wait()
            .await
            .map_err(|error| format!("{operation}: wait: {error}"))?;
        return if status.success() {
            Ok(())
        } else {
            Err(format!(
                "{operation}: {status}; diagnostics streamed to parent stdout/stderr"
            ))
        };
    }
    let output = command
        .output()
        .await
        .map_err(|error| format!("{operation}: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{operation}: {}; {}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// Execute a compiler command with isolated child stdio and immediate diagnostics.
/// Quiet mode retains failed-child output. Live mode preserves real bytes and
/// exit status without sharing the parent's stdio file-status flags.
pub fn run_native_process(
    command: &mut Command,
    operation: &str,
    live: bool,
) -> Result<(), String> {
    run_process(command, operation, live)
}

async fn forward_compiler_output(
    mut input: impl tokio::io::AsyncRead + Unpin,
    mut output: impl AsyncWrite + Unpin,
) -> std::io::Result<()> {
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        match input.read(&mut buffer).await {
            Ok(0) => return Ok(()),
            Ok(count) => {
                write_compiler_chunk(&mut output, &buffer[..count]).await?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            Err(error) => return Err(error),
        }
    }
}

// Cargo consumes build-script output as lines. Flushing a fragment alone does
// not expose it to the outer supervisor. Preserve the payload and delimit each
// received chunk; already complete records remain byte-for-byte unchanged.
// This is diagnostic framing, not a timer, compiler completion or send permit.
async fn write_compiler_chunk(
    output: &mut (impl AsyncWrite + Unpin),
    bytes: &[u8],
) -> std::io::Result<()> {
    if bytes.is_empty() {
        return Ok(());
    }
    write_progress_event(output, bytes).await?;
    if !bytes.ends_with(b"\n") {
        write_progress_event(output, b"\n").await?;
    }
    Ok(())
}

// Read the selected Gerbil toolchain's declared compiler feature, not PATH or CC.

pub(crate) fn configured_gambit_gcc(gsc: &Path) -> bool {
    let Ok(gsc) = fs::canonicalize(gsc) else {
        return false;
    };
    let Some(bin) = gsc.parent() else {
        return false;
    };
    fs::read_to_string(bin.join("gambuild-C")).is_ok_and(|script| {
        script
            .lines()
            .any(|line| line.trim() == "BUILD_FEATURE_C_COMP=\"gcc\"")
    })
}

const COMPILER_ARTIFACT_PROGRESS_BYTES: u64 = 256 * 1024;

#[cfg(test)]
fn compiler_artifact_growth(path: &Path, previous: &mut u64) -> Result<Option<u64>, String> {
    let size = match fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "inspect compiler artifact {}: {error}",
                path.display()
            ));
        }
    };
    Ok(observe_artifact_length(size, previous))
}

fn observe_artifact_length(size: u64, previous: &mut u64) -> Option<u64> {
    (size.saturating_sub(*previous) >= COMPILER_ARTIFACT_PROGRESS_BYTES).then(|| {
        *previous = size;
        size
    })
}

fn remove_compiler_artifacts(paths: &[PathBuf]) -> Result<(), String> {
    for path in paths {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "remove compiler artifact {}: {error}",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

async fn emit_compiler_artifact_event(
    output: &mut (impl AsyncWrite + Unpin),
    path: &Path,
    size: u64,
) -> Result<(), String> {
    // Format once: write_fmt/write_all cannot resume after a nonblocking
    // partial write. Retain the offset until the complete event is delivered.
    let event = format!("native compiler artifact={} bytes={size}\n", path.display());
    write_progress_event(output, event.as_bytes())
        .await
        .map_err(|error| format!("write compiler artifact progress: {error}"))
}

async fn write_progress_event(
    output: &mut (impl AsyncWrite + Unpin),
    mut bytes: &[u8],
) -> std::io::Result<()> {
    while !bytes.is_empty() {
        match output.write(bytes).await {
            Ok(0) => return Err(std::io::ErrorKind::WriteZero.into()),
            Ok(written) => bytes = &bytes[written..],
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                // Wait for the consumer, emitting nothing. The enclosing
                // process supervisor retains its idle and wall budgets.
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            Err(error) => return Err(error),
        }
    }
    loop {
        match output.flush().await {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            Err(error) => return Err(error),
        }
    }
}

fn compiler_artifact_paths(object: &Path) -> [PathBuf; 4] {
    ["i", "s", "ira", "reload"].map(|extension| object.with_extension(extension))
}

pub(super) fn gcc_allocation_diagnostics(object: &Path) -> Result<String, String> {
    let mut options = Vec::new();
    for extension in ["ira", "reload"] {
        let path = object.with_extension(extension);
        let name = path
            .to_str()
            .ok_or_else(|| format!("GCC diagnostic path is not UTF-8: {}", path.display()))?;
        // Gambit's -cc-options is parsed by Gambit's shell command. Preserve
        // spaces and apostrophes in the exact object-owned diagnostic path.
        let quoted = name.replace('\'', "'\\''");
        options.push(format!("-fdump-rtl-{extension}-details='{quoted}'"));
    }
    Ok(options.join(" "))
}

pub(super) fn run_with_compiler_artifacts(
    command: &mut Command,
    operation: &str,
    object: &Path,
) -> Result<(), String> {
    let artifacts = compiler_artifact_paths(object);
    // A previous attempt must never qualify as progress for this process.
    remove_compiler_artifacts(&artifacts)?;
    let runtime = process_runtime::acquire()?;
    let result = runtime.block_on(run_process_async(
        command,
        operation,
        true,
        Some(&artifacts),
    ));
    let cleanup = remove_compiler_artifacts(&artifacts);
    result.and(cleanup)
}

async fn observe_compiler_artifacts(paths: &[PathBuf]) -> std::io::Result<()> {
    let mut previous = vec![0; paths.len()];
    let mut interval = tokio::time::interval(Duration::from_millis(100));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        for (path, previous) in paths.iter().zip(previous.iter_mut()) {
            let size = match tokio::fs::metadata(path).await {
                Ok(metadata) => metadata.len(),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            if let Some(size) = observe_artifact_length(size, previous) {
                emit_compiler_artifact_event(&mut tokio::io::stderr(), path, size)
                    .await
                    .map_err(std::io::Error::other)?;
            }
        }
    }
}

#[cfg(all(test, unix))]
#[path = "../../tests/unit/program_process.rs"]
mod process_tests;
