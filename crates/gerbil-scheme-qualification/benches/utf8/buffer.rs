//! Prototype: encode into exclusively owned Rust capacity, never a Scheme view.
use super::Root;
use gerbil_scheme::GerbilRuntime;
use gerbil_scheme_sys::{GerbilRootId, GerbilStatus};

unsafe extern "C" {
    fn gerbil_utf8_buffer_encode(root: i64, output: *mut u8, capacity: u64) -> i64;
    fn gerbil_utf8_buffer_mutate(root: i64, index: u64, codepoint: u32);
    fn gerbil_scheme_rust_root_string_encode_into_raw(
        root: i64,
        output: *mut u8,
        capacity: u64,
    ) -> i64;
}

pub(super) fn root_text(runtime: &GerbilRuntime, text: &str) -> Root {
    let bytes = runtime.bytevector_from_bytes(text.as_bytes()).unwrap();
    let mut root = GerbilRootId(0);
    // SAFETY: live owner-local input and writable output; independent root.
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_utf8_to_string(
                bytes.root_id(),
                &raw mut root,
            )
        },
        GerbilStatus::Ok
    );
    Root(root)
}

pub(super) fn convert(value: &Root) -> String {
    convert_owned(value, 4)
}

pub(super) fn production(value: &Root) -> String {
    convert_owned(value, 5)
}

/// Same raw-entry framing as the frozen control; public ABI contracts run separately.
pub(super) fn codec(value: &Root) -> String {
    convert_owned(value, 7)
}

/// Test-only allocation policy; shrinking remains inside the timed conversion.
pub(super) fn compact(value: &Root) -> String {
    let mut text = production(value);
    text.shrink_to_fit();
    text
}

fn convert_owned(value: &Root, mode: i32) -> String {
    let mut characters: usize = 0;
    // SAFETY: input root remains live on its foreign-entry owner.
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_string_length(value.0, &raw mut characters)
        },
        GerbilStatus::Ok
    );
    let capacity = characters.checked_mul(4).expect("capacity overflow");
    let mut bytes = Vec::<u8>::with_capacity(capacity);
    // SAFETY: exclusive Rust allocation stays live/stable throughout the
    // synchronous call. No foreign pointer or Scheme body is retained.
    let written = if mode == 5 {
        let mut written = 0;
        // SAFETY: identical stable exclusive capacity, through the actual ABI.
        assert_eq!(
            unsafe {
                gerbil_scheme_sys::gerbil_scheme_rust_root_string_encode_into(
                    value.0,
                    bytes.as_mut_ptr(),
                    capacity,
                    &raw mut written,
                )
            },
            GerbilStatus::Ok
        );
        written
    } else {
        let encode = if mode == 7 {
            gerbil_scheme_rust_root_string_encode_into_raw
        } else {
            gerbil_utf8_buffer_encode
        };
        let written = unsafe {
            encode(
                value.0.0,
                bytes.as_mut_ptr(),
                u64::try_from(capacity).unwrap(),
            )
        };
        usize::try_from(written).expect("encoder must fail closed")
    };
    assert!(written <= capacity);
    // SAFETY: success initialized exactly this prefix; nothing is published
    // after an error. The spare capacity remains uninitialized and inaccessible.
    unsafe { bytes.set_len(written) };
    simdutf8::basic::from_utf8(&bytes).expect("complete UTF-8 validation");
    // SAFETY: entire exclusively owned prefix was validated above.
    unsafe { String::from_utf8_unchecked(bytes) }
}

pub(super) fn contracts(runtime: &GerbilRuntime) {
    for start in (0..0x11_0000_u32).step_by(4096) {
        let text: String = (start..(start + 4096).min(0x11_0000))
            .filter_map(char::from_u32)
            .collect();
        let value = root_text(runtime, &text);
        assert_eq!(convert(&value), text, "full scalar parity {start}");
        assert_eq!(production(&value), text, "production scalar parity {start}");
        assert_eq!(compact(&value), text, "compact scalar parity {start}");
    }
    for text in [String::new(), "ASCII\0data".into(), "汉字😀\0".repeat(8193)] {
        assert_eq!(convert(&root_text(runtime, &text)), text);
    }
    let value = root_text(runtime, &"a".repeat(4096));
    let snapshot = super::convert(&value, 3);
    for codepoint in [
        0, 127, 128, 2047, 2048, 55_295, 57_344, 65_535, 65_536, 1_114_111,
    ] {
        // SAFETY: test-only mutation on this live owner-local string root.
        unsafe { gerbil_utf8_buffer_mutate(value.0.0, 2047, codepoint) };
        let expected = format!(
            "{}{}{}",
            "a".repeat(2047),
            char::from_u32(codepoint).unwrap(),
            "a".repeat(2048)
        );
        assert_eq!(convert(&value), expected);
        assert_eq!(production(&value), expected);
        assert_eq!(compact(&value), expected);
        assert_eq!(super::convert(&value, 3), expected);
    }
    assert_eq!(snapshot, "a".repeat(4096));
    let mut output = vec![0xa5; 4096 * 4];
    for codepoint in 0xd800..=0xdfff {
        // SAFETY: only a negative fixture uses Gambit's raw surrogate primitive.
        unsafe { gerbil_utf8_buffer_mutate(value.0.0, 0, codepoint) };
        // SAFETY: a valid exclusive span is supplied; rejection precedes writes
        // because the first character is invalid. No output length is admitted.
        assert!(
            unsafe {
                gerbil_utf8_buffer_encode(
                    value.0.0,
                    output.as_mut_ptr(),
                    u64::try_from(output.len()).unwrap(),
                )
            } < 0
        );
        assert!(output.iter().all(|byte| *byte == 0xa5));
        let mut written = usize::MAX;
        // SAFETY: full exclusive span; rejection must not publish a length.
        assert_eq!(
            unsafe {
                gerbil_scheme_sys::gerbil_scheme_rust_root_string_encode_into(
                    value.0,
                    output.as_mut_ptr(),
                    output.len(),
                    &raw mut written,
                )
            },
            GerbilStatus::InvalidValue
        );
        assert_eq!(written, usize::MAX);
        assert!(output.iter().all(|byte| *byte == 0xa5));
    }
    // SAFETY: restore the negative fixture before capacity/type controls.
    unsafe { gerbil_utf8_buffer_mutate(value.0.0, 0, u32::from('a')) };
    let wrong = runtime.bytevector_from_bytes(b"wrong type").unwrap();
    for (root, capacity) in [
        (value.0, 16_383),
        (value.0, u64::MAX),
        (wrong.root_id(), 16_384),
        (GerbilRootId(0), 16_384),
    ] {
        // SAFETY: impossible capacity/type is rejected before touching the
        // valid span. The exaggerated span must never reach a write.
        assert!(unsafe { gerbil_utf8_buffer_encode(root.0, output.as_mut_ptr(), capacity) } < 0);
        assert!(output.iter().all(|byte| *byte == 0xa5));
    }
    // SAFETY: null output is a negative control, rejected before writing.
    assert!(unsafe { gerbil_utf8_buffer_encode(value.0.0, std::ptr::null_mut(), 16_384) } < 0);
    let empty = root_text(runtime, "");
    // SAFETY: empty input produces no writes and accepts the zero-length span.
    assert_eq!(
        unsafe { gerbil_utf8_buffer_encode(empty.0.0, std::ptr::null_mut(), 0) },
        0
    );
    let released = root_text(runtime, "released");
    let token = released.0;
    drop(released);
    // SAFETY: released-token rejection must precede all output access.
    assert!(unsafe { gerbil_utf8_buffer_encode(token.0, output.as_mut_ptr(), 16_384) } < 0);
    assert!(output.iter().all(|byte| *byte == 0xa5));
    let mut written = usize::MAX;
    // SAFETY: late invalid input exercises partial writes without publication.
    unsafe { gerbil_utf8_buffer_mutate(value.0.0, 4095, 0xd800) };
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_string_encode_into(
                value.0,
                output.as_mut_ptr(),
                output.len(),
                &raw mut written,
            )
        },
        GerbilStatus::InvalidValue
    );
    assert_eq!(written, usize::MAX);
    assert_ne!(output[0], 0xa5);
    assert_eq!(runtime.add_i64(40, 2).unwrap(), 42);
    eprintln!(
        "UTF8-BUFFER-CONTRACTS Unicode=1112064 surrogates=2048 mutation=OK bounds=OK roots=OK"
    );
}
