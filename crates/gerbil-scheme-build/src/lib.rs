// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Build support for Gerbil Scheme modules consumed by Rust.

use gerbil_scheme::GerbilToolchain;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

/// Opaque Gerbil source transport.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilSource {
    module: String,
    text: String,
}

impl GerbilSource {
    /// Construct a checked source transport.
    /// # Errors
    ///
    /// Returns [`BuildError::InvalidModule`] when `module` is not a portable
    /// Gerbil module name.
    pub fn new(module: impl Into<String>, text: impl Into<String>) -> Result<Self, BuildError> {
        let module = module.into();
        if module.is_empty()
            || !module
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(BuildError::InvalidModule(module));
        }
        Ok(Self {
            module,
            text: text.into(),
        })
    }

    /// Logical module name, without an extension.
    #[must_use]
    pub fn module(&self) -> &str {
        &self.module
    }

    /// Complete source text.  The build layer transports but does not parse it.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Kind of artifact produced by the build support layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GerbilArtifactKind {
    /// Gerbil module source emitted to the build directory.
    ModuleSource,
    /// Gambit Scheme source emitted by `gxc -S`.
    GambitSource,
}

/// Checked artifact with explicit provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilCompiledArtifact {
    kind: GerbilArtifactKind,
    path: PathBuf,
}

impl GerbilCompiledArtifact {
    /// Artifact kind.
    #[must_use]
    pub fn kind(&self) -> GerbilArtifactKind {
        self.kind
    }

    /// Artifact path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Compiler façade that treats source as opaque Gerbil text.
#[derive(Clone, Debug)]
pub struct GerbilCompiler {
    toolchain: GerbilToolchain,
}

impl GerbilCompiler {
    /// Bind to an explicit toolchain.
    #[must_use]
    pub fn new(toolchain: GerbilToolchain) -> Self {
        Self { toolchain }
    }

    /// Emit one checked source module without invoking the compiler.
    /// # Errors
    ///
    /// Returns [`BuildError`] when the output directory or source file cannot
    /// be created.
    pub fn emit_source(
        &self,
        source: &GerbilSource,
        output_dir: impl AsRef<Path>,
    ) -> Result<GerbilCompiledArtifact, BuildError> {
        let output_dir = output_dir.as_ref();
        fs::create_dir_all(output_dir).map_err(BuildError::Io)?;
        let path = output_dir.join(format!("{}.ss", source.module()));
        fs::write(&path, source.text()).map_err(BuildError::Io)?;
        Ok(GerbilCompiledArtifact {
            kind: GerbilArtifactKind::ModuleSource,
            path,
        })
    }

    /// Compile one module to Gambit Scheme source with `gxc -S`.
    /// # Errors
    ///
    /// Returns [`BuildError`] when source emission fails, `gxc` cannot be
    /// started, or the compiler exits unsuccessfully.
    pub fn compile_to_gambit(
        &self,
        source: &GerbilSource,
        output_dir: impl AsRef<Path>,
    ) -> Result<GerbilCompiledArtifact, BuildError> {
        let output_dir = output_dir.as_ref();
        let emitted = self.emit_source(source, output_dir)?;
        let output = Command::new(self.toolchain.gxc())
            .args([
                OsStr::new("-S"),
                OsStr::new("-d"),
                output_dir.as_os_str(),
                emitted.path().as_os_str(),
            ])
            .output()
            .map_err(|source| BuildError::Spawn {
                program: self.toolchain.gxc().to_path_buf(),
                source,
            })?;
        if !output.status.success() {
            return Err(BuildError::Exit {
                operation: "gxc -S",
                status: output.status,
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        let gambit_source = find_module_extension(output_dir, source.module(), "scm")?;
        Ok(GerbilCompiledArtifact {
            kind: GerbilArtifactKind::GambitSource,
            path: gambit_source,
        })
    }
}

impl Default for GerbilCompiler {
    fn default() -> Self {
        Self::new(GerbilToolchain::from_env())
    }
}

/// Failure at the checked build boundary.
#[derive(Debug)]
pub enum BuildError {
    /// Module names are intentionally restricted to portable file names.
    InvalidModule(String),
    /// File-system operation failed.
    Io(std::io::Error),
    /// A configured tool could not be started.
    Spawn {
        /// Program that failed to start.
        program: PathBuf,
        /// Operating-system error.
        source: std::io::Error,
    },
    /// A compiler process returned a non-zero status.
    Exit {
        /// Build operation being performed.
        operation: &'static str,
        /// Process exit status.
        status: ExitStatus,
        /// Captured standard output.
        stdout: String,
        /// Captured standard error.
        stderr: String,
    },
    /// The compiler did not emit exactly one expected artifact.
    ArtifactCount {
        /// Extension being selected.
        extension: &'static str,
        /// Matching paths.
        matches: Vec<PathBuf>,
    },
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidModule(module) => {
                write!(f, "invalid portable Gerbil module name {module:?}")
            }
            Self::Io(source) => write!(f, "Gerbil build I/O failed: {source}"),
            Self::Spawn { program, source } => {
                write!(f, "failed to start {}: {source}", program.display())
            }
            Self::Exit {
                operation,
                status,
                stderr,
                ..
            } => write!(f, "{operation} exited with {status}: {}", stderr.trim()),
            Self::ArtifactCount { extension, matches } => write!(
                f,
                "expected one .{extension} artifact, found {}: {matches:?}",
                matches.len()
            ),
        }
    }
}

impl Error for BuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(source) | Self::Spawn { source, .. } => Some(source),
            Self::InvalidModule(_) | Self::Exit { .. } | Self::ArtifactCount { .. } => None,
        }
    }
}

fn find_module_extension(
    output_dir: &Path,
    module: &str,
    extension: &'static str,
) -> Result<PathBuf, BuildError> {
    let expected_file_name = format!("{module}.{extension}");
    let mut matches = fs::read_dir(output_dir)
        .map_err(BuildError::Io)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.file_name() == Some(OsStr::new(&expected_file_name)))
        .collect::<Vec<_>>();
    matches.sort();
    if matches.len() == 1 {
        Ok(matches.remove(0))
    } else {
        Err(BuildError::ArtifactCount { extension, matches })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must follow Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("gerbil-scheme-rust-{name}-{nonce}"))
    }

    #[test]
    fn rejects_non_portable_module_names() {
        let error = GerbilSource::new("../escape", "(displayln 1)")
            .expect_err("path traversal must be rejected");
        assert!(matches!(error, BuildError::InvalidModule(_)));
    }

    #[test]
    fn emits_opaque_source() {
        let output_dir = scratch_dir("emit");
        let source = GerbilSource::new("answer", "(displayln 42)\n").expect("valid source");
        let compiler = GerbilCompiler::default();
        let artifact = compiler
            .emit_source(&source, &output_dir)
            .expect("source emission must succeed");
        assert_eq!(artifact.kind(), GerbilArtifactKind::ModuleSource);
        assert_eq!(
            fs::read_to_string(artifact.path()).expect("emitted source must be readable"),
            source.text()
        );
        fs::remove_dir_all(output_dir).expect("scratch directory must be removable");
    }
}
