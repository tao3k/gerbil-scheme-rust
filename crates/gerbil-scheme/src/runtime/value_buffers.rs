// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use super::{NativeError, NativeResult};
use gerbil_scheme_sys::GerbilStatus;

/// Keep initialization separate from capacity: no zero-fill before a bulk copy.
pub(super) fn copy_reused_bytevector(
    length: NativeResult<usize>,
    bytes: &mut Vec<u8>,
    operation: &'static str,
    copy: impl FnOnce(*mut u8, usize) -> GerbilStatus,
) -> NativeResult<()> {
    let length = match length.into_result() {
        Ok(length) => length,
        Err(error) => return NativeResult::err(error),
    };
    bytes.clear();
    if length == 0 {
        return NativeResult::ok(());
    }
    bytes.reserve(length);
    let destination = bytes.spare_capacity_mut().as_mut_ptr().cast::<u8>();
    let status = copy(destination, length);
    if status != GerbilStatus::Ok {
        return NativeResult::err(NativeError::Status {
            operation,
            code: status as i32,
        });
    }
    // SAFETY: the checked bulk ABI initialized all length bytes synchronously.
    // Capacity is sufficient and no borrowed pointer survives this call.
    unsafe { bytes.set_len(length) };
    NativeResult::ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/reused_buffer.rs"]
mod tests;
