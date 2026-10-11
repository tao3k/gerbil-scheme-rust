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
    use std::os::unix::fs::symlink;
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
    let command = launcher.with_file_name(format!("gsc-progress-{digest}.command"));
    let shim = fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/gsc-progress-launcher.sh"),
    )
    .map_err(|error| format!("resolve package launcher shim: {error}"))?;
    if fs::metadata(&shim)
        .map_err(|error| format!("inspect package launcher shim: {error}"))?
        .permissions()
        .mode()
        & 0o111
        == 0
    {
        return Err("package launcher shim is not executable".to_owned());
    }
    if launcher.exists() {
        let stored = fs::read(&command)
            .map_err(|error| format!("read package launcher command: {error}"))?;
        if stored != script.as_bytes() || fs::read_link(&launcher).ok().as_ref() != Some(&shim) {
            return Err("content-addressed package launcher drift".to_owned());
        }
        return Ok(launcher);
    }
    // Closing our writer is insufficient: a concurrent fork can retain a
    // duplicate until its exec. Never execute the generated writable inode.
    // Publish it only as shell input, then symlink the source-owned executable.
    let staging = out_dir.join(format!(
        ".gsc-progress-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        fs::write(&staging, script.as_bytes())?;
        fs::set_permissions(&staging, fs::Permissions::from_mode(0o600))?;
        match fs::hard_link(&staging, &command) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if fs::read(&command)? == script.as_bytes() {
                    Ok(())
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "content-addressed package launcher drift",
                    ))
                }
            }
            Err(error) => Err(error),
        }
    })();
    let cleanup = match fs::remove_file(&staging) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    };
    if let Err(error) = result.and(cleanup) {
        return Err(format!("publish package GSC launcher: {error}"));
    }
    match symlink(&shim, &launcher) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if fs::read_link(&launcher).ok().as_ref() != Some(&shim) {
                return Err("content-addressed package launcher drift".to_owned());
            }
        }
        Err(error) => return Err(format!("publish package launcher shim: {error}")),
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
