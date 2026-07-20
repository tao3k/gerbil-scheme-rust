// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Parameterized static archive packaging for Gerbil AOT link units.

use std::path::{Path, PathBuf};

/// One native library required by a Gerbil AOT link plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeLinkLibrary(String);

impl NativeLinkLibrary {
    /// Creates a native library directive value.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the directive value, for example `gambit` or `dylib=m`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Complete neutral input for packaging Gerbil-generated objects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeStaticLinkPlan {
    /// Gerbil module objects produced by the canonical AOT build.
    pub module_objects: Vec<PathBuf>,
    /// Gambit linker object for the module set.
    pub link_object: PathBuf,
    /// Native library search directories required by the result.
    pub link_search_dirs: Vec<PathBuf>,
    /// Native libraries required by the result, in link order.
    pub link_libraries: Vec<NativeLinkLibrary>,
}

/// Cargo directive category emitted by a native link plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoDirectiveKind {
    /// Adds a native library search directory.
    RustcLinkSearch,
    /// Adds a native library.
    RustcLinkLib,
}

/// One Cargo build-script directive without Marlin-specific policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoDirective {
    /// Directive category.
    pub kind: CargoDirectiveKind,
    /// Value written after the Cargo directive name.
    pub value: String,
}

impl CargoDirective {
    /// Creates one Cargo directive.
    #[must_use]
    pub fn new(kind: CargoDirectiveKind, value: impl Into<String>) -> Self {
        Self {
            kind,
            value: value.into(),
        }
    }

    /// Renders the exact build-script output line.
    #[must_use]
    pub fn line(&self) -> String {
        let name = match self.kind {
            CargoDirectiveKind::RustcLinkSearch => "rustc-link-search",
            CargoDirectiveKind::RustcLinkLib => "rustc-link-lib",
        };
        format!("cargo:{name}={}", self.value)
    }
}

/// Receipt for one packaged native archive and its consumer directives.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeArchiveLinkReceipt {
    /// Cargo/rustc link name without platform prefix or suffix.
    pub archive_name: String,
    /// Produced archive path.
    pub archive_file: PathBuf,
    /// Ordered Cargo directives required by downstream consumers.
    pub cargo_directives: Vec<CargoDirective>,
}

/// Packages Gerbil module objects and a Gambit linker object into one archive.
///
/// # Errors
///
/// Returns an error when the plan is invalid, an input object is missing, the
/// output directory cannot be created, or the native archiver fails.
pub fn build_static_archive_from_link_plan(
    archive_name: &str,
    link_plan: &NativeStaticLinkPlan,
    out_dir: &Path,
) -> Result<NativeArchiveLinkReceipt, String> {
    validate_archive_name(archive_name)?;
    validate_link_plan(link_plan)?;
    std::fs::create_dir_all(out_dir).map_err(|error| error.to_string())?;

    let mut build = cc::Build::new();
    build.cargo_metadata(false).out_dir(out_dir);
    for object in &link_plan.module_objects {
        build.object(object);
    }
    build.object(&link_plan.link_object);
    build
        .try_compile(archive_name)
        .map_err(|error| error.to_string())?;

    let archive_file = out_dir.join(static_archive_file_name(archive_name));
    if !archive_file.is_file() {
        return Err(format!(
            "static archive was not produced at {}",
            archive_file.display()
        ));
    }

    Ok(NativeArchiveLinkReceipt {
        archive_name: archive_name.to_owned(),
        archive_file,
        cargo_directives: static_archive_cargo_directives(archive_name, out_dir, link_plan),
    })
}

/// Builds the ordered Cargo directives for a packaged archive.
#[must_use]
pub fn static_archive_cargo_directives(
    archive_name: &str,
    archive_dir: &Path,
    link_plan: &NativeStaticLinkPlan,
) -> Vec<CargoDirective> {
    let mut directives = vec![
        CargoDirective::new(
            CargoDirectiveKind::RustcLinkSearch,
            format!("native={}", archive_dir.display()),
        ),
        CargoDirective::new(
            CargoDirectiveKind::RustcLinkLib,
            format!("static={archive_name}"),
        ),
    ];
    directives.extend(link_plan.link_search_dirs.iter().map(|search_dir| {
        CargoDirective::new(
            CargoDirectiveKind::RustcLinkSearch,
            format!("native={}", search_dir.display()),
        )
    }));
    directives.extend(
        link_plan
            .link_libraries
            .iter()
            .map(|library| CargoDirective::new(CargoDirectiveKind::RustcLinkLib, library.as_str())),
    );
    directives
}

/// Returns the platform archive filename for one rustc link name.
#[must_use]
pub fn static_archive_file_name(archive_name: &str) -> String {
    if cfg!(target_env = "msvc") {
        format!("{archive_name}.lib")
    } else {
        format!("lib{archive_name}.a")
    }
}

fn validate_archive_name(archive_name: &str) -> Result<(), String> {
    if archive_name.is_empty()
        || archive_name
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || character == '_'))
    {
        return Err(format!("invalid native archive name: {archive_name:?}"));
    }
    Ok(())
}

fn validate_link_plan(link_plan: &NativeStaticLinkPlan) -> Result<(), String> {
    if link_plan.module_objects.is_empty() {
        return Err("native link plan has no module objects".to_owned());
    }
    for object in link_plan
        .module_objects
        .iter()
        .chain(std::iter::once(&link_plan.link_object))
    {
        if !object.is_file() {
            return Err(format!("native object is missing: {}", object.display()));
        }
    }
    Ok(())
}
