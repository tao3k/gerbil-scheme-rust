// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Generic Gerbil package native AOT compile/staging support.
//!
//! This module intentionally models only Gerbil package mechanics:
//! `gxpkg env gxc -target C -s -S -O`, local builder artifact lookup, and
//! staging generated Gambit Scheme files.  Downstream runtimes should keep
//! their domain receipts, link-unit status, archive naming, and cargo
//! directives outside this crate.

/// Environment variable selecting the Gerbil package tool.
pub const GERBIL_GXPKG_ENV: &str = "GERBIL_GXPKG";

/// Gerbil tools required for package-native AOT compilation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilBuildToolchain {
    gxpkg: std::path::PathBuf,
    gxc: std::path::PathBuf,
}

impl GerbilBuildToolchain {
    /// Construct an explicit Gerbil package build toolchain.
    #[must_use]
    pub fn new(gxpkg: impl Into<std::path::PathBuf>, gxc: impl Into<std::path::PathBuf>) -> Self {
        Self {
            gxpkg: gxpkg.into(),
            gxc: gxc.into(),
        }
    }

    /// Discover package build tools from environment variables with
    /// conventional command-name fallbacks.
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(
            std::env::var_os(GERBIL_GXPKG_ENV).unwrap_or_else(|| std::ffi::OsString::from("gxpkg")),
            std::env::var_os(gerbil_scheme::GERBIL_GXC_ENV)
                .unwrap_or_else(|| std::ffi::OsString::from("gxc")),
        )
    }

    /// Path or command used for `gxpkg`.
    #[must_use]
    pub fn gxpkg(&self) -> &std::path::Path {
        &self.gxpkg
    }

    /// Path or command used for `gxc`.
    #[must_use]
    pub fn gxc(&self) -> &std::path::Path {
        &self.gxc
    }
}

impl Default for GerbilBuildToolchain {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Neutral command receipt for Gerbil package native AOT compile/staging.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilNativeAotCommandReceipt {
    /// Process status code, or `None` when the process could not be spawned.
    pub status_code: Option<i32>,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr or spawn/staging error text.
    pub stderr: String,
}

/// Generic package-native AOT compile/staging plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilPackageNativeAotCompilePlan<'a> {
    /// Gerbil package directory containing `gerbil.pkg`.
    pub package_dir: &'a std::path::Path,
    /// Directory where generated `*.scm` artifacts should be staged.
    pub stage_dir: &'a std::path::Path,
    /// Primary package-relative source passed to `gxc`.
    pub source_path: &'a str,
    /// Package-relative sources used only for artifact staging.
    pub dependency_source_paths: &'a [&'a str],
    /// Generated primary `*.scm` filename to stage.
    pub staged_scm_file: &'a str,
    /// Generated dependency `*.scm` filenames to stage.
    pub staged_dependency_scm_files: &'a [&'a str],
}

/// Run `gxpkg env gxc -target C -s -S -O <source_path>` in a Gerbil package.
#[must_use]
pub fn run_package_native_aot_compile(
    toolchain: &GerbilBuildToolchain,
    package_dir: &std::path::Path,
    source_path: &str,
) -> GerbilNativeAotCommandReceipt {
    command_receipt_from_output(
        std::process::Command::new(toolchain.gxpkg())
            .current_dir(package_dir)
            .arg("env")
            .arg(toolchain.gxc())
            .arg("-target")
            .arg("C")
            .arg("-s")
            .arg("-S")
            .arg("-O")
            .arg(source_path)
            .output(),
    )
}

/// Compile and stage package-native AOT artifacts.
#[must_use]
pub fn compile_package_native_aot_artifact(
    toolchain: &GerbilBuildToolchain,
    plan: &GerbilPackageNativeAotCompilePlan<'_>,
) -> GerbilNativeAotCommandReceipt {
    for dependency_source_path in plan.dependency_source_paths {
        let dependency_receipt =
            run_package_native_aot_compile(toolchain, plan.package_dir, dependency_source_path);
        if dependency_receipt.status_code != Some(0) {
            return dependency_receipt;
        }
    }

    let compile_receipt =
        run_package_native_aot_compile(toolchain, plan.package_dir, plan.source_path);
    if compile_receipt.status_code != Some(0) {
        return compile_receipt;
    }

    let package_manifest = plan.package_dir.join("gerbil.pkg");
    let staged_scm_files = std::iter::once(plan.staged_scm_file)
        .chain(plan.staged_dependency_scm_files.iter().copied())
        .collect::<Vec<_>>();
    stage_local_builder_compiled_scms(
        &package_manifest,
        plan.stage_dir,
        &staged_scm_files,
        &compile_receipt,
    )
}

/// Stage generated local-builder `*.scm` files into a caller-owned directory.
#[must_use]
pub fn stage_local_builder_compiled_scms(
    package_manifest: &std::path::Path,
    stage_dir: &std::path::Path,
    staged_scm_files: &[&str],
    compile_receipt: &GerbilNativeAotCommandReceipt,
) -> GerbilNativeAotCommandReceipt {
    if let Err(error) = std::fs::create_dir_all(stage_dir) {
        return GerbilNativeAotCommandReceipt {
            status_code: Some(67),
            stdout: compile_receipt.stdout.clone(),
            stderr: format!(
                "failed to create native AOT staging directory {}: {error}",
                stage_dir.display()
            ),
        };
    }

    for staged_scm_file in staged_scm_files {
        let builder_scm = match find_local_builder_artifact(package_manifest, staged_scm_file) {
            Ok(path) => path,
            Err(error) => {
                return GerbilNativeAotCommandReceipt {
                    status_code: Some(66),
                    stdout: compile_receipt.stdout.clone(),
                    stderr: format!(
                        "native AOT compile succeeded but Gerbil local builder artifact is unavailable: {error}"
                    ),
                };
            }
        };
        let staged_scm = stage_dir.join(staged_scm_file);
        if let Err(error) = std::fs::copy(&builder_scm, &staged_scm) {
            return GerbilNativeAotCommandReceipt {
                status_code: Some(67),
                stdout: compile_receipt.stdout.clone(),
                stderr: format!(
                    "failed to stage Gerbil local builder artifact from {} to {}: {error}",
                    builder_scm.display(),
                    staged_scm.display()
                ),
            };
        }
    }

    GerbilNativeAotCommandReceipt {
        status_code: Some(0),
        stdout: compile_receipt.stdout.clone(),
        stderr: compile_receipt.stderr.clone(),
    }
}

/// Find one generated local-builder artifact under `<package>/.gerbil`.
///
/// # Errors
///
/// Returns an error when the package manifest has no parent directory, the
/// local builder root is missing, the builder tree cannot be read, the artifact
/// is missing, or more than one matching artifact is found.
pub fn find_local_builder_artifact(
    package_manifest: &std::path::Path,
    staged_scm_file: &str,
) -> Result<std::path::PathBuf, String> {
    let Some(package_dir) = package_manifest.parent() else {
        return Err(format!(
            "Gerbil package manifest has no parent directory: {}",
            package_manifest.display()
        ));
    };
    let builder_root = package_dir.join(".gerbil");
    if !builder_root.is_dir() {
        return Err(format!(
            "missing Gerbil local builder root {}",
            builder_root.display()
        ));
    }

    let mut stack = vec![builder_root.clone()];
    let mut matches = Vec::new();
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir)
            .map_err(|error| format!("failed to read {}: {error}", dir.display()))?;
        for entry in entries {
            let path = entry
                .map_err(|error| format!("failed to read entry under {}: {error}", dir.display()))?
                .path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.file_name().and_then(|name| name.to_str()) == Some(staged_scm_file) {
                matches.push(path);
            }
        }
    }
    matches.sort();
    match matches.as_slice() {
        [path] => Ok(path.clone()),
        [] => Err(format!(
            "missing {staged_scm_file} under Gerbil local builder root {}",
            builder_root.display()
        )),
        _ => Err(format!(
            "ambiguous {staged_scm_file} under Gerbil local builder root {}: {matches:?}",
            builder_root.display()
        )),
    }
}

fn command_receipt_from_output(
    output: std::io::Result<std::process::Output>,
) -> GerbilNativeAotCommandReceipt {
    match output {
        Ok(output) => GerbilNativeAotCommandReceipt {
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        },
        Err(error) => GerbilNativeAotCommandReceipt {
            status_code: None,
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
}
