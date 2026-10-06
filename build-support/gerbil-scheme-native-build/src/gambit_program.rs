// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Shared runtime and build-time Gambit executable discovery.
use std::{
    env,
    path::{Path, PathBuf},
};

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
        let line = line.trim();
        let mut characters = line.chars();
        if characters.next() == Some('v')
            && characters
                .next()
                .is_some_and(|character| character.is_ascii_digit())
        {
            return true;
        }
        // Development Gambit builds print revision, timestamp, target and their
        // configure command instead of a release vN version.
        let fields: Vec<_> = line.split_whitespace().take(4).collect();
        fields.len() == 4
            && (7..=40).contains(&fields[0].len())
            && fields[0].chars().all(|c| c.is_ascii_hexdigit())
            && fields[1].len() == 14
            && fields[1].chars().all(|c| c.is_ascii_digit())
            && fields[2].contains('-')
            && fields[3].starts_with("\"./configure")
    })
}

/// Resolve the Gambit `gsc` compiler paired with a Gerbil `gxi` executable.
#[must_use]
pub fn default_gambit_gsc_program_for_gxi(gxi: impl AsRef<Path>) -> PathBuf {
    resolve_gerbil_executable(gxi)
        .and_then(paired_gambit_gsc)
        .or_else(|| {
            resolve_gerbil_executable("gsc").filter(|program| is_gambit_gsc_program(program))
        })
        .unwrap_or_else(|| PathBuf::from("gsc"))
}

fn paired_gambit_gsc(gxi: PathBuf) -> Option<PathBuf> {
    [gxi.canonicalize().unwrap_or_else(|_| gxi.clone()), gxi]
        .into_iter()
        .find_map(|selected| compiler_for_selected_gxi(&selected))
}

fn compiler_for_selected_gxi(selected: &Path) -> Option<PathBuf> {
    let sibling = selected.parent()?.join("gsc");
    if sibling.is_file() && is_gambit_gsc_program(&sibling) {
        return Some(sibling);
    }
    let wrapper = std::fs::read_to_string(selected).ok()?;
    wrapper
        .lines()
        .filter_map(|line| literal_gerbil_home(line.trim()))
        .map(|home| home.join("bin/gsc"))
        .find(|candidate| candidate.is_file() && is_gambit_gsc_program(candidate))
}

fn literal_gerbil_home(line: &str) -> Option<PathBuf> {
    let value = line
        .strip_prefix("export ")
        .unwrap_or(line)
        .strip_prefix("GERBIL_HOME=")?
        .trim();
    let literal = if let Some(rest) = value.strip_prefix('"') {
        rest.split('"').next()?
    } else if let Some(rest) = value.strip_prefix('\'') {
        rest.split('\'').next()?
    } else {
        value.split_whitespace().next()?
    };
    if literal.contains(['$', '`']) {
        return None;
    }
    let home = PathBuf::from(literal);
    home.is_absolute().then_some(home)
}

/// Select the native compiler once for compilation and library discovery.
/// Explicit overrides are validated; conventional lookup rejects Ghostscript.
/// # Errors
/// Rejects missing explicit tools or a selected executable that is not Gambit.
pub fn discover_gambit_gsc_from_env() -> Result<PathBuf, String> {
    let program = if let Some(explicit) = env::var_os("GERBIL_GSC") {
        resolve_gerbil_executable(PathBuf::from(explicit))
            .ok_or_else(|| "configured GERBIL_GSC is missing".to_owned())?
    } else if let Some(home) = env::var_os("GERBIL_HOME") {
        let candidate = PathBuf::from(home).join("bin/gsc");
        if candidate.is_file() {
            candidate
        } else {
            default_gambit_gsc_program_for_gxi(
                env::var_os("GERBIL_GXI").unwrap_or_else(|| "gxi".into()),
            )
        }
    } else {
        default_gambit_gsc_program_for_gxi(
            env::var_os("GERBIL_GXI").unwrap_or_else(|| "gxi".into()),
        )
    };
    if !is_gambit_gsc_program(&program) {
        return Err(format!(
            "selected compiler is not Gambit gsc: {}",
            program.display()
        ));
    }
    Ok(program)
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
