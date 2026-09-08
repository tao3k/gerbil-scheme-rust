// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Checked status conversion; discriminants remain in the C ABI layout owner.

use crate::abi::GerbilStatus;

impl GerbilStatus {
    /// Returns the stable integer representation used by the C ABI.
    #[must_use]
    pub const fn code(self) -> i32 {
        self as i32
    }

    /// Decodes a status returned by the C ABI.
    ///
    /// Unknown values are preserved by returning `None`, allowing newer Gerbil
    /// runtimes to extend the status space without making this binding unsound.
    #[must_use]
    pub const fn from_code(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::Ok),
            1 => Some(Self::NullPointer),
            2 => Some(Self::AbiMismatch),
            3 => Some(Self::InvalidValue),
            4 => Some(Self::RuntimeUnavailable),
            5 => Some(Self::Panic),
            6 => Some(Self::AlreadyInitialized),
            7 => Some(Self::NotInitialized),
            8 => Some(Self::RuntimeFinalized),
            _ => None,
        }
    }
}
