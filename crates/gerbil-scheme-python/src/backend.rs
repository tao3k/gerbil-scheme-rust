// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Explicit provider selection. No parser, compiler or fallback is involved.
use std::{error::Error, fmt};

/// An explicitly requested backend was not compiled or is not recognized.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendError {
    requested: String,
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "execution backend {:?} is unknown or unavailable; select an installed scheme or rust provider explicitly",
            self.requested
        )
    }
}
impl Error for BackendError {}

/// Select exactly the requested provider. A missing provider is an error, even
/// when the other backend is available. Providers are library-owned, not Python
/// import strings or arbitrary commands.
///
/// # Errors
/// Rejects unknown names and unavailable providers without invoking either.
pub fn select_backend<T>(
    name: &str,
    scheme: Option<T>,
    rust: Option<T>,
) -> Result<T, BackendError> {
    match name {
        "scheme" => scheme,
        "rust" => rust,
        _ => None,
    }
    .ok_or_else(|| BackendError {
        requested: name.into(),
    })
}
