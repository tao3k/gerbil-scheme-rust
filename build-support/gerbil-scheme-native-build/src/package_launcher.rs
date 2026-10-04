// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! SDK-selected diagnostics for Gerbil's dynamic package compiler calls.
use std::path::{Path, PathBuf};

/// Retains the configured compiler and arguments, adding GCC diagnostics in
/// verbose package builds, including macro modules whose options Gerbil drops.
pub fn prepare_gsc_progress_launcher(
    gsc: &Path,
    out_dir: &Path,
    live: bool,
) -> Result<PathBuf, String> {
    if !live || !crate::program::configured_gambit_gcc(gsc) {
        return Ok(gsc.to_path_buf());
    }
    write_launcher(gsc, out_dir)
}

#[cfg(unix)]
fn write_launcher(gsc: &Path, out_dir: &Path) -> Result<PathBuf, String> {
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    let gsc = fs::canonicalize(gsc).map_err(|error| format!("resolve package GSC: {error}"))?;
    let gsc = gsc.to_str().ok_or("package GSC path is not UTF-8")?;
    let quoted = gsc.replace('\'', "'\"'\"'");
    let script = format!("#!/bin/sh\nexec '{quoted}' -cc-options '-Q -fopt-info-all' \"$@\"\n");
    let digest = format!("{:x}", Sha256::digest(script.as_bytes()));
    fs::create_dir_all(out_dir)
        .map_err(|error| format!("create package launcher directory: {error}"))?;
    let launcher = out_dir.join(format!("gsc-progress-{digest}"));
    if launcher.exists() {
        let stored =
            fs::read(&launcher).map_err(|error| format!("read package launcher: {error}"))?;
        if stored != script.as_bytes() {
            return Err("content-addressed package launcher drift".to_owned());
        }
        return Ok(launcher);
    }
    // Publish only a closed, executable inode. Concurrent users never execute
    // the staging file or observe a file still open for writing (ETXTBSY).
    let staging = out_dir.join(format!(
        ".gsc-progress-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        fs::write(&staging, script.as_bytes())?;
        fs::set_permissions(&staging, fs::Permissions::from_mode(0o755))?;
        fs::rename(&staging, &launcher)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&staging);
        return Err(format!("publish package GSC launcher: {error}"));
    }
    Ok(launcher)
}

#[cfg(not(unix))]
fn write_launcher(_gsc: &Path, _out_dir: &Path) -> Result<PathBuf, String> {
    Err("verbose GCC package launcher requires a Unix host".to_owned())
}

#[cfg(all(test, unix))]
#[path = "../tests/unit/package_launcher_scenario.rs"]
mod tests;
