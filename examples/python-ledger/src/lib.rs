// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Installed ledger extension with explicitly compiled execution backends.
mod api;
#[cfg(feature = "rust-backend")]
mod rust_backend;
