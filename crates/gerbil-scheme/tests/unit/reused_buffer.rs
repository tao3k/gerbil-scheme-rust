// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use super::{GerbilStatus, NativeError, NativeResult, copy_reused_bytevector};

#[test]
fn reuse_handles_growth_shrink_empty_and_failed_initialization() {
    let mut bytes = Vec::with_capacity(1_000_000);
    let pointer = bytes.as_ptr();
    let capacity = bytes.capacity();
    for length in [1, 63, 64, 65, 8192, 65537, 1_000_000, 17, 0] {
        let mut calls = 0;
        copy_reused_bytevector(NativeResult::ok(length), &mut bytes, "test", |out, n| {
            calls += 1;
            assert_eq!(n, length);
            // SAFETY: the helper provides n writable bytes of spare capacity.
            unsafe { out.write_bytes(0xA5, n) };
            GerbilStatus::Ok
        })
        .into_result()
        .unwrap();
        assert_eq!(calls, usize::from(length != 0));
        assert_eq!(bytes.len(), length);
        assert!(bytes.iter().all(|&byte| byte == 0xA5));
        assert_eq!(bytes.as_ptr(), pointer);
        assert_eq!(bytes.capacity(), capacity);
    }
    bytes.extend_from_slice(&[1, 2, 3]);
    let error = NativeError::Status {
        operation: "length",
        code: -1,
    };
    assert!(
        copy_reused_bytevector(NativeResult::err(error), &mut bytes, "test", |_, _| {
            panic!("invalid length must not copy")
        })
        .into_result()
        .is_err()
    );
    assert_eq!(bytes, [1, 2, 3]);
    assert!(
        copy_reused_bytevector(NativeResult::ok(17), &mut bytes, "test", |out, _| {
            // SAFETY: a deliberately partial write remains within spare capacity.
            unsafe { out.write(0xFF) };
            GerbilStatus::InvalidValue
        })
        .into_result()
        .is_err()
    );
    assert!(bytes.is_empty());
    assert_eq!(bytes.capacity(), capacity);
    let mut small = vec![0; 1];
    copy_reused_bytevector(NativeResult::ok(8193), &mut small, "test", |out, n| {
        // SAFETY: reserve must grow the allocation before invoking the copy.
        unsafe { out.write_bytes(0x5A, n) };
        GerbilStatus::Ok
    })
    .into_result()
    .unwrap();
    assert_eq!(small, vec![0x5A; 8193]);
}
