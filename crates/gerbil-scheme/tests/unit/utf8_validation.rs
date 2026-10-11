// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use super::validated_string;
use crate::runtime::NativeError;

fn parity(bytes: &[u8]) {
    let standard = String::from_utf8(bytes.to_vec());
    let actual = validated_string(bytes.to_vec());
    assert_eq!(
        actual.is_ok(),
        standard.is_ok(),
        "acceptance mismatch: {bytes:?}"
    );
    if let Ok(expected) = standard {
        assert_eq!(actual.unwrap(), expected);
    } else {
        assert!(
            matches!(actual, Err(NativeError::Status { operation: "gerbil_scheme_rust_root_string_to_utf8", code })
            if code == gerbil_scheme_sys::GerbilStatus::InvalidValue as i32)
        );
    }
}

#[test]
fn validation_matches_std_for_all_one_and_two_byte_inputs() {
    parity(&[]);
    for first in 0..=u8::MAX {
        parity(&[first]);
        for second in 0..=u8::MAX {
            parity(&[first, second]);
        }
    }
}

#[test]
fn validation_matches_std_at_simd_boundaries_and_malformed_tails() {
    let sequences: &[&[u8]] = &[
        b"\0",
        b"ASCII",
        "汉😀e\u{301}".as_bytes(),
        &[0x80],
        &[0xc0, 0x80],
        &[0xc1, 0xbf],
        &[0xc2],
        &[0xe0, 0x80, 0x80],
        &[0xed, 0xa0, 0x80],
        &[0xed, 0xbf, 0xbf],
        &[0xf0, 0x80, 0x80, 0x80],
        &[0xf4, 0x90, 0x80, 0x80],
        &[0xf5, 0x80, 0x80, 0x80],
        &[0xff],
        &[0xf0, 0x9f, 0x98],
    ];
    for prefix in 0..=129 {
        for sequence in sequences {
            for suffix in [0, 1, 15, 16, 31, 32, 63, 64, 65, 127, 128, 8192] {
                let mut bytes = vec![b'a'; prefix];
                bytes.extend_from_slice(sequence);
                bytes.extend(std::iter::repeat_n(b'b', suffix));
                parity(&bytes);
            }
        }
    }
}

#[test]
fn validation_preserves_all_unicode_scalars_and_owned_allocation() {
    let mut text = String::new();
    for scalar in 0..=0x0010_ffff {
        if let Some(character) = char::from_u32(scalar) {
            text.push(character);
        }
        if text.len() >= 4096 {
            parity(text.as_bytes());
            text.clear();
        }
    }
    parity(text.as_bytes());
    let bytes = "changed\0汉字😀".repeat(683).into_bytes();
    let pointer = bytes.as_ptr();
    let capacity = bytes.capacity();
    let text = validated_string(bytes).unwrap();
    assert_eq!(text.as_ptr(), pointer);
    assert_eq!(text.capacity(), capacity);
}
