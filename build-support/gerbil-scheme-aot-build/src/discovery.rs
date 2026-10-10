// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Gambit library discovery for real executables and Gerbil shell wrappers.

use std::fs;
use std::path::{Path, PathBuf};

/// Located Gambit library and the native link-search directory that owns it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GambitLinkSearchDiscovery {
    /// Directory supplied to Cargo/rustc as a native link search path.
    pub search_dir: PathBuf,
    /// Concrete Gambit static or shared library found in the directory.
    pub library_path: PathBuf,
}

/// Finds the Gambit library associated with a selected `gsc` executable.
#[must_use]
pub fn discover_gambit_link_search_dir_from_gsc(gsc: &Path) -> Option<GambitLinkSearchDiscovery> {
    gambit_link_search_dir_candidates(gsc)
        .into_iter()
        .find_map(|search_dir| {
            find_gambit_library(&search_dir).map(|library_path| GambitLinkSearchDiscovery {
                search_dir,
                library_path,
            })
        })
}

fn gambit_link_search_dir_candidates(gsc: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    push_gsc_prefix_lib_candidate(&mut candidates, gsc);
    if let Ok(canonical_gsc) = fs::canonicalize(gsc) {
        push_gsc_prefix_lib_candidate(&mut candidates, &canonical_gsc);
    }
    if let Ok(wrapper) = fs::read_to_string(gsc) {
        for prefix in gsc_wrapper_prefixes(&wrapper) {
            push_unique_candidate(&mut candidates, prefix.join("lib"));
        }
    }
    candidates
}

fn push_gsc_prefix_lib_candidate(candidates: &mut Vec<PathBuf>, gsc: &Path) {
    if let Some(prefix) = gsc.parent().and_then(Path::parent) {
        push_unique_candidate(candidates, prefix.join("lib"));
    }
}

fn push_unique_candidate(candidates: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !candidates.iter().any(|existing| existing == &candidate) {
        candidates.push(candidate);
    }
}

fn gsc_wrapper_prefixes(wrapper: &str) -> impl Iterator<Item = PathBuf> + '_ {
    wrapper.lines().flat_map(|line| {
        let line = line.trim();
        [gerbil_home_prefix(line), exec_gsc_prefix(line)]
            .into_iter()
            .flatten()
    })
}

fn gerbil_home_prefix(line: &str) -> Option<PathBuf> {
    let value = line
        .strip_prefix("export ")
        .unwrap_or(line)
        .strip_prefix("GERBIL_HOME=")?;
    shell_word(value).map(PathBuf::from)
}

fn exec_gsc_prefix(line: &str) -> Option<PathBuf> {
    let program = PathBuf::from(shell_word(line.strip_prefix("exec ")?)?);
    if program.file_name()? != "gsc" {
        return None;
    }
    program
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}

fn shell_word(value: &str) -> Option<String> {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix('"') {
        return rest.split('"').next().map(str::to_owned);
    }
    if let Some(rest) = value.strip_prefix('\'') {
        return rest.split('\'').next().map(str::to_owned);
    }
    value.split_whitespace().next().map(str::to_owned)
}

fn find_gambit_library(search_dir: &Path) -> Option<PathBuf> {
    for file_name in ["libgambit.a", "libgambit.dylib", "libgambit.so"] {
        let library = search_dir.join(file_name);
        if library.is_file() {
            return Some(library);
        }
    }
    fs::read_dir(search_dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.is_file()
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("libgambit.so."))
        })
}
