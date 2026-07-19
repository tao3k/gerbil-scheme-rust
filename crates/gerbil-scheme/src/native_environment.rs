//! Native Gerbil tool environment helpers.

use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

/// Environment variable used by Gerbil tools to locate their installation.
pub const GERBIL_HOME_ENV: &str = "GERBIL_HOME";

/// Gambit option environment variable used to override runtime paths.
pub const GAMBOPT_ENV: &str = "GAMBOPT";

/// Native Gerbil tool environment inferred from an installed tool path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilNativeToolEnvironment {
    prefix: PathBuf,
    gambopt: OsString,
    remove_sdkroot: bool,
}

impl GerbilNativeToolEnvironment {
    /// Infer a native tool environment from a Gerbil or Gambit executable path.
    #[must_use]
    pub fn infer(program: impl AsRef<Path>) -> Option<Self> {
        let program = program.as_ref();
        let prefix = infer_gerbil_tool_prefix(program)?;
        Some(Self {
            gambopt: merged_gambopt(&prefix),
            remove_sdkroot: should_remove_sdkroot(program),
            prefix,
        })
    }

    /// Gerbil installation prefix.
    #[must_use]
    pub fn prefix(&self) -> &Path {
        &self.prefix
    }

    /// Merged Gambit runtime options for this prefix.
    #[must_use]
    pub fn gambopt(&self) -> &OsString {
        &self.gambopt
    }

    /// Whether `SDKROOT` should be removed for this command.
    #[must_use]
    pub fn remove_sdkroot(&self) -> bool {
        self.remove_sdkroot
    }

    /// Apply this environment to a command.
    pub fn apply_to_command(&self, command: &mut Command) {
        if self.remove_sdkroot {
            command.env_remove("SDKROOT");
        }
        command.env(GERBIL_HOME_ENV, &self.prefix);
        command.env(GAMBOPT_ENV, &self.gambopt);
    }
}

/// Infer and apply the native Gerbil tool environment to a command.
pub fn configure_gerbil_native_tool_command(command: &mut Command, program: impl AsRef<Path>) {
    if let Some(environment) = GerbilNativeToolEnvironment::infer(program) {
        environment.apply_to_command(command);
    }
}

fn infer_gerbil_tool_prefix(program: &Path) -> Option<PathBuf> {
    let prefix = program.parent()?.parent()?;
    let bin = prefix.join("bin");
    let lib = prefix.join("lib");
    let include = prefix.join("include");

    if !bin.is_dir()
        || !lib.join("gerbil").is_dir()
        || !include.join("gambit.h").is_file()
        || !has_gambit_runtime_library(&lib)
    {
        return None;
    }

    Some(prefix.to_path_buf())
}

fn merged_gambopt(prefix: &Path) -> OsString {
    let mut value = OsString::from(format!(
        "~~bin={},~~lib={},~~include={}",
        prefix.join("bin").display(),
        prefix.join("lib").display(),
        prefix.join("include").display()
    ));

    if let Some(existing) = env::var_os(GAMBOPT_ENV).filter(|existing| !existing.is_empty()) {
        value.push(",");
        value.push(existing);
    }

    value
}

fn has_gambit_runtime_library(lib: &Path) -> bool {
    ["libgambit.a", "libgambit.dylib", "libgambit.so"]
        .iter()
        .any(|file_name| lib.join(file_name).is_file())
}

#[cfg(target_os = "macos")]
fn should_remove_sdkroot(program: &Path) -> bool {
    let resolved_program = std::fs::canonicalize(program).unwrap_or_else(|_| program.to_path_buf());
    resolved_program
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        == Some("gsc")
        && !resolved_program.starts_with("/nix/store")
        && env::var_os("SDKROOT")
            .as_deref()
            .map(Path::new)
            .is_some_and(|sdkroot| sdkroot.starts_with("/nix/store"))
}

#[cfg(not(target_os = "macos"))]
fn should_remove_sdkroot(_program: &Path) -> bool {
    false
}
