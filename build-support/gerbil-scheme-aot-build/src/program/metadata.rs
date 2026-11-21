// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Gambit's public linker input format, preserving lazy module initialization.
use std::path::Path;
use std::process::Command;

pub(super) fn scheme_string(path: &Path) -> String {
    format!(
        "\"{}\"",
        path.to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    )
}

pub(super) fn add_link_input(
    command: &mut Command,
    inputs: &mut Vec<String>,
    path: &Path,
    flagged: bool,
    deferred: bool,
) {
    if flagged {
        inputs.push(format!(
            "({} (preload . {}))",
            scheme_string(path),
            if deferred { "#f" } else { "#t" }
        ));
    } else {
        command.arg(path);
    }
}
