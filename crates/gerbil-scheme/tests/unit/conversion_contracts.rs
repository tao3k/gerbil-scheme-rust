// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use gerbil_scheme::{GerbilRuntime, GerbilStatus};

pub(super) fn exercises_utf8_bulk_conversion(runtime: &GerbilRuntime) {
    exercises_reused_vec_output(runtime);
    exercises_independent_utf8_roots(runtime);
    for text in [
        String::new(),
        "ASCII\0data".into(),
        "汉字😀e\u{301}".into(),
        "汉字😀\0".repeat(8193),
    ] {
        let value = runtime.string_from_utf8(&text).expect("root UTF-8 string");
        assert_eq!(value.len().into_result().unwrap(), text.chars().count());
        assert_eq!(value.to_string().into_result().unwrap(), text);
        assert_eq!(value.to_string().into_result().unwrap(), text);
        let snapshot = value.to_utf8_bytes().into_result().unwrap();
        drop(value);
        // Rooted encoding has independent lifetime, including empty, NUL and
        // multi-byte Unicode values. Further allocation may collect the source.
        let other = runtime.string_from_utf8("independent allocation").unwrap();
        let mut output = Vec::new();
        snapshot.copy_to_vec(&mut output).into_result().unwrap();
        assert_eq!(output, text.as_bytes());
        assert_eq!(snapshot.to_vec().into_result().unwrap(), text.as_bytes());
        drop(other);
    }
    let invalid = runtime.bytevector_from_bytes(&[0xFF, 0xC0, 0x80]).unwrap();
    let mut sentinel = gerbil_scheme_sys::GerbilRootId(987_654);
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_utf8_to_string(
                invalid.root_id(),
                &raw mut sentinel,
            )
        },
        GerbilStatus::InvalidValue
    );
    assert_eq!(sentinel, gerbil_scheme_sys::GerbilRootId(987_654));
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_string_to_utf8(
                invalid.root_id(),
                &raw mut sentinel,
            )
        },
        GerbilStatus::InvalidValue
    );
    assert_eq!(sentinel, gerbil_scheme_sys::GerbilRootId(987_654));
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_utf8_to_string(
                invalid.root_id(),
                std::ptr::null_mut(),
            )
        },
        GerbilStatus::NullPointer
    );
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_utf8_to_string(
                gerbil_scheme_sys::GerbilRootId(0),
                &raw mut sentinel,
            )
        },
        GerbilStatus::InvalidValue
    );
}

fn exercises_reused_vec_output(runtime: &GerbilRuntime) {
    let mut output = Vec::with_capacity(100_000);
    let pointer = output.as_ptr();
    let capacity = output.capacity();
    for length in [0, 1, 63, 64, 65, 8193, 100_000, 17, 0] {
        let expected: Vec<_> = (0..=u8::MAX).cycle().take(length).collect();
        let rooted = runtime.bytevector_from_bytes(&expected).unwrap();
        rooted.copy_to_vec(&mut output).into_result().unwrap();
        assert_eq!(output, expected);
        assert_eq!(output.as_ptr(), pointer);
        assert_eq!(output.capacity(), capacity);
    }
    let borrowed = runtime.fixture_bytevector_value().unwrap();
    let borrowed = borrowed.as_bytevector().into_result().unwrap();
    borrowed.copy_to_vec(&mut output).into_result().unwrap();
    assert_eq!(output, [255, 127, 11, 1, 0]);
    assert_eq!(output.as_ptr(), pointer);
    let large = runtime.bytevector_from_bytes(&vec![0xA5; 100_001]).unwrap();
    large.copy_to_vec(&mut output).into_result().unwrap();
    assert_eq!(output, vec![0xA5; 100_001]);
}

fn exercises_independent_utf8_roots(runtime: &GerbilRuntime) {
    use gerbil_scheme_sys::{
        GerbilRootId, gerbil_scheme_rust_root_bytevector_copy, gerbil_scheme_rust_root_release,
        gerbil_scheme_rust_root_string_to_utf8, gerbil_scheme_rust_root_utf8_to_string,
    };
    let expected = "owned\0汉字😀".as_bytes();
    let input = runtime.bytevector_from_bytes(expected).unwrap();
    let mut string = GerbilRootId(0);
    // SAFETY: these calls stay on the live owner. Each successful conversion
    // transfers an independent root; every token is released exactly once.
    assert_eq!(
        unsafe { gerbil_scheme_rust_root_utf8_to_string(input.root_id(), &raw mut string) },
        GerbilStatus::Ok
    );
    drop(input);
    let mut bytes = GerbilRootId(0);
    assert_eq!(
        unsafe { gerbil_scheme_rust_root_string_to_utf8(string, &raw mut bytes) },
        GerbilStatus::Ok
    );
    assert_eq!(
        unsafe { gerbil_scheme_rust_root_release(string) },
        GerbilStatus::Ok
    );
    let mut output = vec![0; expected.len()];
    assert_eq!(
        unsafe {
            gerbil_scheme_rust_root_bytevector_copy(bytes, output.as_mut_ptr(), output.len())
        },
        GerbilStatus::Ok
    );
    assert_eq!(output, expected);
    let mut sentinel = GerbilRootId(987_654);
    assert_eq!(
        unsafe { gerbil_scheme_rust_root_string_to_utf8(string, &raw mut sentinel) },
        GerbilStatus::InvalidValue
    );
    assert_eq!(sentinel, GerbilRootId(987_654));
    assert_eq!(
        unsafe { gerbil_scheme_rust_root_release(bytes) },
        GerbilStatus::Ok
    );
    assert_eq!(
        unsafe { gerbil_scheme_rust_root_release(bytes) },
        GerbilStatus::InvalidValue
    );
}

pub(super) fn checks_borrowed_bytevector_bulk_copy(runtime: &GerbilRuntime, value: usize) {
    let mut output = [0_u8; 5];
    // SAFETY: this fixture remains live on the runtime owner, and output is
    // disjoint writable storage of exactly the requested size.
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_bytevector_copy(
                value,
                output.as_mut_ptr(),
                output.len(),
            )
        },
        GerbilStatus::Ok
    );
    assert_eq!(output, [255, 127, 11, 1, 0]);
    let mut sentinel = [0xA5_u8; 4];
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_bytevector_copy(
                value,
                sentinel.as_mut_ptr(),
                sentinel.len(),
            )
        },
        GerbilStatus::InvalidValue
    );
    assert_eq!(sentinel, [0xA5; 4]);
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_bytevector_copy(
                value,
                std::ptr::null_mut(),
                5,
            )
        },
        GerbilStatus::NullPointer
    );
    let non_bytevector = runtime.fixture_fixnum_value().expect("live fixnum control");
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_bytevector_copy(
                non_bytevector.as_raw(),
                sentinel.as_mut_ptr(),
                sentinel.len(),
            )
        },
        GerbilStatus::InvalidValue
    );
    assert_eq!(sentinel, [0xA5; 4]);
}
