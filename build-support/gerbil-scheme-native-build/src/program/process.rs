// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Owned native subprocess diagnostics and compiler-written progress.
use crate::gerbil_command;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::time::Duration;

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
        // Runtime options must precede compiler options. Line buffering also
        // applies when Cargo redirects stdout/stderr to pipes. q/Q retain
        // nonzero termination rather than entering a REPL on failure.
        command.args(GSC_PROGRESS_OPTIONS);
    }
    command
}

pub(super) fn run_process(
    command: &mut Command,
    operation: &str,
    live: bool,
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
        let stdout = child.stdout.take().expect("piped native stdout");
        let stderr = child.stderr.take().expect("piped native stderr");
        return std::thread::scope(|scope| {
            let out = scope.spawn(|| forward_native_output(stdout, &mut std::io::stdout()));
            let err = scope.spawn(|| forward_native_output(stderr, &mut std::io::stderr()));
            let status = child
                .wait()
                .map_err(|error| format!("{operation}: {error}"));
            let out = out.join().map_err(compiler_monitor_panic_message)?;
            let err = err.join().map_err(compiler_monitor_panic_message)?;
            out.and(err)
                .map_err(|error| format!("{operation}: forward native diagnostics: {error}"))?;
            let status = status?;
            if status.success() {
                Ok(())
            } else {
                Err(format!(
                    "{operation}: {status}; diagnostics streamed to parent stdout/stderr"
                ))
            }
        });
    }
    let output = command
        .output()
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

/// Execute a native command with isolated child stdio and immediate diagnostics.
/// Quiet mode retains failed-child output. Live mode preserves real bytes and
/// exit status without sharing the parent's stdio file-status flags.
pub fn run_native_process(
    command: &mut Command,
    operation: &str,
    live: bool,
) -> Result<(), String> {
    run_process(command, operation, live)
}

fn forward_native_output(
    mut input: impl std::io::Read,
    output: &mut impl std::io::Write,
) -> std::io::Result<()> {
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        match input.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(count) => write_progress_event(output, &buffer[..count])?,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(error) => return Err(error),
        }
    }
}

// Read the immutable SDK's declared compiler feature, not PATH or CC.
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
    if size.saturating_sub(*previous) < COMPILER_ARTIFACT_PROGRESS_BYTES {
        return Ok(None);
    }
    *previous = size;
    Ok(Some(size))
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

fn emit_compiler_artifact_event(
    output: &mut impl std::io::Write,
    path: &Path,
    size: u64,
) -> Result<(), String> {
    // Format once: write_fmt/write_all cannot resume after a nonblocking
    // partial write. Retain the offset until the complete event is delivered.
    let event = format!("native compiler artifact={} bytes={size}\n", path.display());
    write_progress_event(output, event.as_bytes())
        .map_err(|error| format!("write compiler artifact progress: {error}"))
}

fn write_progress_event(output: &mut impl std::io::Write, mut bytes: &[u8]) -> std::io::Result<()> {
    while !bytes.is_empty() {
        match output.write(bytes) {
            Ok(0) => return Err(std::io::ErrorKind::WriteZero.into()),
            Ok(written) => bytes = &bytes[written..],
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                // Wait for the consumer, emitting nothing. The enclosing
                // process supervisor retains its idle and wall budgets.
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(error) => return Err(error),
        }
    }
    loop {
        match output.flush() {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(error) => return Err(error),
        }
    }
}

fn compiler_monitor_panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    let detail = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("non-string panic payload");
    format!("compiler artifact monitor panicked: {detail}")
}

pub(super) fn run_with_compiler_artifacts(
    command: &mut Command,
    operation: &str,
    object: &Path,
) -> Result<(), String> {
    let artifacts = [object.with_extension("i"), object.with_extension("s")];
    // A previous attempt must never qualify as progress for this process.
    remove_compiler_artifacts(&artifacts)?;
    let running = std::sync::atomic::AtomicBool::new(true);
    let result = std::thread::scope(|scope| {
        let monitor = scope.spawn(|| -> Result<(), String> {
            let mut previous = [0_u64; 2];
            while running.load(Ordering::Acquire) {
                for (path, previous) in artifacts.iter().zip(previous.iter_mut()) {
                    if let Some(size) = compiler_artifact_growth(path, previous)? {
                        emit_compiler_artifact_event(&mut std::io::stderr().lock(), path, size)?;
                    }
                }
                // Polling itself emits nothing. Only compiler-written growth
                // produces an event; a stalled compiler still fails supervision.
                std::thread::sleep(Duration::from_millis(100));
            }
            Ok(())
        });
        let result = run_process(command, operation, true);
        running.store(false, Ordering::Release);
        let observed = monitor.join().map_err(compiler_monitor_panic_message)?;
        result.and(observed)
    });
    let cleanup = remove_compiler_artifacts(&artifacts);
    result.and(cleanup)
}

#[cfg(all(test, unix))]
#[path = "../../tests/unit/program_process.rs"]
mod process_tests;
