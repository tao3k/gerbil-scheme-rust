// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Safe Rust APIs for invoking Gerbil Scheme.
//!
//! The initial safe boundary is process-backed: it invokes a configured `gxi`
//! executable and returns checked output.  The API is intentionally independent
//! from Marlin and other application runtimes so an embedded backend can be
//! added without changing downstream domain semantics.

use std::env;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[cfg(feature = "native")]
pub use gerbil_scheme_sys::{
    GERBIL_SCHEME_RUST_ABI_ID, GERBIL_SCHEME_RUST_ABI_VERSION, GerbilStatus,
};

#[cfg(feature = "native")]
mod native;

#[cfg(feature = "native")]
pub use native::{
    SchemeBorrowedBytevector, SchemeBorrowedVector, SchemeKeyword, SchemeList, SchemeNil,
    SchemePair, SchemePairParts, SchemeScalar, SchemeSymbol, SchemeVoid,
};

#[cfg(feature = "native")]
pub use native::{
    GerbilI64Callback, GerbilI64CallbackAbi, GerbilRuntime, GerbilRuntimeReceipt, GerbilUtf8,
    GerbilValue, GerbilValueProvenance, NativeError, NativeResult,
};

/// Environment variable selecting the Gerbil interpreter.
pub const GERBIL_GXI_ENV: &str = "GERBIL_GXI";

pub mod native_environment;

/// Environment variable selecting the Gerbil compiler.
pub const GERBIL_GXC_ENV: &str = "GERBIL_GXC";

/// Environment variable selecting the Gambit compiler.
pub const GERBIL_GSC_ENV: &str = "GERBIL_GSC";

/// Executables required by the supported Gerbil binding paths.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilToolchain {
    gxi: PathBuf,
    gxc: PathBuf,
    gsc: PathBuf,
}

impl GerbilToolchain {
    /// Construct an explicit Gerbil toolchain.
    #[must_use]
    pub fn new(gxi: impl Into<PathBuf>, gxc: impl Into<PathBuf>, gsc: impl Into<PathBuf>) -> Self {
        Self {
            gxi: gxi.into(),
            gxc: gxc.into(),
            gsc: gsc.into(),
        }
    }

    /// Discover tools from explicit environment variables, falling back to
    /// their conventional command names on `PATH`.
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(
            env::var_os(GERBIL_GXI_ENV).unwrap_or_else(|| OsString::from("gxi")),
            env::var_os(GERBIL_GXC_ENV).unwrap_or_else(|| OsString::from("gxc")),
            env::var_os(GERBIL_GSC_ENV).unwrap_or_else(|| OsString::from("gsc")),
        )
    }

    /// Path or command used for `gxi`.
    #[must_use]
    pub fn gxi(&self) -> &Path {
        &self.gxi
    }

    /// Path or command used for `gxc`.
    #[must_use]
    pub fn gxc(&self) -> &Path {
        &self.gxc
    }

    /// Path or command used for `gsc`.
    #[must_use]
    pub fn gsc(&self) -> &Path {
        &self.gsc
    }

    /// Resolve the paired Gambit `gsc` compiler for this Gerbil toolchain.
    ///
    /// This checks PATH for a Gambit `gsc`, then falls back to a `gsc` next to
    /// the resolved `gxi`.
    #[must_use]
    pub fn resolved_gsc(&self) -> PathBuf {
        default_gambit_gsc_program_for_gxi(self.gxi())
    }

    /// Resolve a conventional Gerbil tool name through this toolchain.
    ///
    /// Program names `gxi`, `gxc`, and `gsc` are mapped to the configured
    /// toolchain entries. Paths or other program names are returned unchanged.
    #[must_use]
    pub fn program(&self, program: impl AsRef<Path>) -> PathBuf {
        let program = program.as_ref();
        let Some(program_name) = program.file_name().and_then(|name| name.to_str()) else {
            return program.to_path_buf();
        };

        if program
            .parent()
            .is_some_and(|parent| !parent.as_os_str().is_empty())
        {
            return program.to_path_buf();
        }

        match program_name {
            "gxi" => self.gxi().to_path_buf(),
            "gxc" => self.gxc().to_path_buf(),
            "gsc" => self.gsc().to_path_buf(),
            _ => program.to_path_buf(),
        }
    }
}

/// Resolve a configured Gerbil executable through PATH when it is a program name.
#[must_use]
pub fn resolve_gerbil_executable(program: impl AsRef<Path>) -> Option<PathBuf> {
    let program = program.as_ref();
    if should_check_gerbil_program_directly(program) {
        return program.is_file().then(|| program.to_path_buf());
    }

    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|dir| dir.join(program))
            .find(|candidate| candidate.is_file())
    })
}

/// Returns whether a `gsc` executable appears to be the Gambit compiler.
#[must_use]
pub fn is_gambit_gsc_program(program: impl AsRef<Path>) -> bool {
    let Ok(output) = std::process::Command::new(program.as_ref())
        .arg("-v")
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    stdout.lines().chain(stderr.lines()).any(|line| {
        let mut characters = line.trim().chars();
        characters.next() == Some('v')
            && characters
                .next()
                .is_some_and(|character| character.is_ascii_digit())
    })
}

/// Resolve the Gambit `gsc` compiler paired with a Gerbil `gxi` executable.
#[must_use]
pub fn default_gambit_gsc_program_for_gxi(gxi: impl AsRef<Path>) -> PathBuf {
    let path_program = PathBuf::from("gsc");
    if let Some(program) =
        resolve_gerbil_executable(&path_program).filter(|program| is_gambit_gsc_program(program))
    {
        return program;
    }

    resolve_gerbil_executable(gxi)
        .and_then(|gxi| gxi.parent().map(|bin| bin.join("gsc")))
        .filter(|candidate| candidate.is_file() && is_gambit_gsc_program(candidate))
        .unwrap_or(path_program)
}

fn should_check_gerbil_program_directly(program: &Path) -> bool {
    if program.has_root() {
        return true;
    }
    let mut components = program.components();
    let Some(first) = components.next() else {
        return false;
    };
    matches!(
        first,
        std::path::Component::CurDir
            | std::path::Component::ParentDir
            | std::path::Component::Prefix(_)
    ) || components.next().is_some()
}

impl Default for GerbilToolchain {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Safe process-backed Gerbil runtime binding.
#[derive(Clone, Debug)]
pub struct Gerbil {
    toolchain: GerbilToolchain,
    current_dir: Option<PathBuf>,
    env: Vec<(OsString, OsString)>,
}

impl Gerbil {
    /// Bind to an explicit toolchain.
    #[must_use]
    pub fn new(toolchain: GerbilToolchain) -> Self {
        Self {
            toolchain,
            current_dir: None,
            env: Vec::new(),
        }
    }

    /// Bind using environment discovery.
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(GerbilToolchain::from_env())
    }

    /// Set the working directory used for future invocations.
    #[must_use]
    pub fn current_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(path.into());
        self
    }

    /// Add one environment binding used for future invocations.
    #[must_use]
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// The bound toolchain.
    #[must_use]
    pub fn toolchain(&self) -> &GerbilToolchain {
        &self.toolchain
    }

    /// Query the Gerbil runtime version.
    /// # Errors
    ///
    /// Returns [`GerbilError`] when the interpreter cannot be started or exits
    /// unsuccessfully.
    pub fn version(&self) -> Result<String, GerbilError> {
        let output = self.run(self.toolchain.gxi(), [OsStr::new("--version")])?;
        checked_stdout(
            "gxi --version",
            output.status,
            &output.stdout,
            &output.stderr,
        )
    }

    /// Evaluate one Gerbil expression and capture its standard output.
    ///
    /// The expression is Gerbil source supplied by the caller.  It is passed as
    /// one argument to `gxi -e` and is not interpreted by a shell.
    /// # Errors
    ///
    /// Returns [`GerbilError`] when the interpreter cannot be started or the
    /// expression exits unsuccessfully.
    pub fn eval(&self, expression: &str) -> Result<String, GerbilError> {
        let output = self.run(
            self.toolchain.gxi(),
            [OsStr::new("-e"), OsStr::new(expression)],
        )?;
        checked_stdout("gxi -e", output.status, &output.stdout, &output.stderr)
    }

    /// Evaluate an expression that writes one signed integer.
    /// # Errors
    ///
    /// Returns [`GerbilError`] for interpreter failures and when the result is
    /// not a valid `i64`.
    pub fn eval_i64(&self, expression: &str) -> Result<i64, GerbilError> {
        let stdout = self.eval(expression)?;
        stdout
            .trim()
            .parse::<i64>()
            .map_err(|source| GerbilError::InvalidInteger { stdout, source })
    }

    fn run<I, S>(&self, program: &Path, args: I) -> Result<std::process::Output, GerbilError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new(program);
        command.args(args).envs(self.env.iter().cloned());
        if let Some(current_dir) = &self.current_dir {
            command.current_dir(current_dir);
        }
        command.output().map_err(|source| GerbilError::Spawn {
            program: program.to_path_buf(),
            source,
        })
    }
}

impl Default for Gerbil {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Failure at the checked Gerbil process boundary.
#[derive(Debug)]
pub enum GerbilError {
    /// The configured Gerbil executable could not be started.
    Spawn {
        /// Program that failed to start.
        program: PathBuf,
        /// Operating-system error.
        source: std::io::Error,
    },
    /// Gerbil returned a non-zero status.
    Exit {
        /// Binding operation being performed.
        operation: &'static str,
        /// Process exit status.
        status: ExitStatus,
        /// Captured standard output.
        stdout: String,
        /// Captured standard error.
        stderr: String,
    },
    /// Gerbil output was not UTF-8.
    Utf8 {
        /// Binding operation being performed.
        operation: &'static str,
        /// UTF-8 conversion error.
        source: std::string::FromUtf8Error,
    },
    /// A checked integer projection failed.
    InvalidInteger {
        /// Complete captured standard output.
        stdout: String,
        /// Integer parsing error.
        source: std::num::ParseIntError,
    },
}

impl fmt::Display for GerbilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn { program, source } => {
                write!(f, "failed to start {}: {source}", program.display())
            }
            Self::Exit {
                operation,
                status,
                stderr,
                ..
            } => write!(f, "{operation} exited with {status}: {}", stderr.trim()),
            Self::Utf8 { operation, source } => {
                write!(f, "{operation} returned non-UTF-8 output: {source}")
            }
            Self::InvalidInteger { stdout, source } => {
                write!(f, "Gerbil output {stdout:?} is not an i64: {source}")
            }
        }
    }
}

impl Error for GerbilError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Spawn { source, .. } => Some(source),
            Self::Utf8 { source, .. } => Some(source),
            Self::InvalidInteger { source, .. } => Some(source),
            Self::Exit { .. } => None,
        }
    }
}

fn checked_stdout(
    operation: &'static str,
    status: ExitStatus,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<String, GerbilError> {
    let stdout = String::from_utf8(stdout.to_vec())
        .map_err(|source| GerbilError::Utf8 { operation, source })?;
    let stderr = String::from_utf8_lossy(stderr).into_owned();
    if status.success() {
        Ok(stdout)
    } else {
        Err(GerbilError::Exit {
            operation,
            status,
            stdout,
            stderr,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_toolchain_preserves_paths() {
        let toolchain = GerbilToolchain::new("/x/gxi", "/x/gxc", "/x/gsc");
        assert_eq!(toolchain.gxi(), Path::new("/x/gxi"));
        assert_eq!(toolchain.gxc(), Path::new("/x/gxc"));
        assert_eq!(toolchain.gsc(), Path::new("/x/gsc"));
    }

    #[test]
    fn missing_interpreter_is_reported_at_spawn_boundary() {
        let runtime = Gerbil::new(GerbilToolchain::new(
            "/definitely/missing/gxi",
            "gxc",
            "gsc",
        ));
        assert!(matches!(runtime.version(), Err(GerbilError::Spawn { .. })));
    }
}
