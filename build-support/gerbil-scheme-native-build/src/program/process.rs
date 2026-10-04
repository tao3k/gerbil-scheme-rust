// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Owned native subprocess diagnostics and compiler-written progress.
use crate::gerbil_command;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
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
        let status = command
            .status()
            .map_err(|error| format!("{operation}: {error}"))?;
        return if status.success() {
            Ok(())
        } else {
            Err(format!(
                "{operation}: {status}; diagnostics streamed to inherited stdout/stderr"
            ))
        };
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
    writeln!(
        output,
        "native compiler artifact={} bytes={size}",
        path.display()
    )
    .and_then(|()| output.flush())
    .map_err(|error| format!("write compiler artifact progress: {error}"))
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
