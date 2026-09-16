// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use gerbil_scheme::{
    ByteOrder, BytestringDelimiter, GERBIL_SCHEME_RUST_ABI_ID, GERBIL_SCHEME_RUST_ABI_VERSION,
    GerbilRuntime, GerbilRuntimeReceipt, GerbilStatus, GerbilValueProvenance, IntegerDecoding,
    IntegerEncoding, IntegerWidth, NativeError,
};

#[test]
fn calls_scalar_export_in_process() {
    let runtime = GerbilRuntime::initialize().expect("initialize in-process Gerbil runtime");
    assert!(matches!(
        GerbilRuntime::initialize(),
        Err(NativeError::AlreadyInitialized)
    ));
    assert_eq!(
        runtime.abi_version().unwrap(),
        GERBIL_SCHEME_RUST_ABI_VERSION
    );
    let receipt = runtime.receipt().expect("runtime receipt");
    assert_eq!(receipt.abi_id, GERBIL_SCHEME_RUST_ABI_ID);
    assert_eq!(receipt.abi_version, GERBIL_SCHEME_RUST_ABI_VERSION);
    assert_eq!(receipt.header_path, "include/gerbil_scheme_rust.h");
    assert_eq!(
        receipt.native_module_path,
        GerbilRuntimeReceipt::NATIVE_MODULE_PATH
    );
    assert_eq!(runtime.add_i64(40, 2).unwrap(), 42);
    exports_scheme_objects_and_traverses_pairs(&runtime);
    exercises_integer_bytevector_conversions(&runtime);
    reports_overflow_and_finalized_runtime_boundaries(runtime);
}

fn exercises_integer_bytevector_conversions(runtime: &GerbilRuntime) {
    exercises_integer_bytevector_decoding(runtime);
    exercises_integer_bytevector_minimal_encoding(runtime);
    exercises_integer_bytevector_width_policy(runtime);
}

fn exercises_integer_bytevector_decoding(runtime: &GerbilRuntime) {
    let fixture = runtime
        .fixture_bytevector_value()
        .expect("export bytevector fixture");
    let fixture = fixture
        .as_bytevector()
        .into_result()
        .expect("project bytevector fixture");
    assert_eq!(
        fixture
            .to_uint(IntegerDecoding::prefix(ByteOrder::Big, integer_width(1)))
            .as_result(),
        Ok(&255)
    );
    assert_eq!(
        fixture
            .to_sint(IntegerDecoding::prefix(ByteOrder::Big, integer_width(1)))
            .as_result(),
        Ok(&-1)
    );

    let big_one = runtime
        .uint_to_bytevector(1, IntegerEncoding::fixed(ByteOrder::Big, integer_width(2)))
        .expect("encode big-endian uint");
    assert_eq!(big_one.to_vec().into_result(), Ok(vec![0, 1]));
    assert_eq!(
        big_one.to_uint(IntegerDecoding::default()).as_result(),
        Ok(&1)
    );

    let little_one = runtime
        .uint_to_bytevector(
            1,
            IntegerEncoding::fixed(ByteOrder::Little, integer_width(2)),
        )
        .expect("encode little-endian uint");
    assert_eq!(little_one.to_vec().into_result(), Ok(vec![1, 0]));
    assert_eq!(
        little_one
            .to_uint(IntegerDecoding::entire(ByteOrder::Little))
            .as_result(),
        Ok(&1)
    );

    let native_one = runtime
        .uint_to_bytevector(
            1,
            IntegerEncoding::fixed(ByteOrder::Native, integer_width(2)),
        )
        .expect("encode native-endian uint");
    assert_eq!(
        native_one.to_vec().into_result(),
        Ok(if cfg!(target_endian = "little") {
            vec![1, 0]
        } else {
            vec![0, 1]
        })
    );
}

fn exercises_integer_bytevector_minimal_encoding(runtime: &GerbilRuntime) {
    let minimal_uint = runtime
        .uint_to_bytevector(258, IntegerEncoding::default())
        .expect("encode minimal uint");
    assert_eq!(minimal_uint.to_vec().into_result(), Ok(vec![1, 2]));
    assert_eq!(
        minimal_uint.to_uint(IntegerDecoding::default()).as_result(),
        Ok(&258)
    );

    let negative = runtime
        .sint_to_bytevector(-23, IntegerEncoding::default())
        .expect("encode minimal negative sint");
    assert_eq!(negative.to_vec().into_result(), Ok(vec![233]));
    assert_eq!(
        negative.to_sint(IntegerDecoding::default()).as_result(),
        Ok(&-23)
    );

    let positive = runtime
        .sint_to_bytevector(233, IntegerEncoding::default())
        .expect("encode minimal positive sint");
    assert_eq!(positive.to_vec().into_result(), Ok(vec![0, 233]));
    assert_eq!(
        positive.to_sint(IntegerDecoding::default()).as_result(),
        Ok(&233)
    );
}

fn exercises_integer_bytevector_width_policy(runtime: &GerbilRuntime) {
    assert!(matches!(
        runtime.uint_to_bytevector(
            258,
            IntegerEncoding::fixed(ByteOrder::Big, integer_width(1))
        ),
        Err(NativeError::UnsignedIntegerWidth {
            value: 258,
            width: 1,
        })
    ));
    assert!(matches!(
        runtime.sint_to_bytevector(
            233,
            IntegerEncoding::fixed(ByteOrder::Big, integer_width(1))
        ),
        Err(NativeError::SignedIntegerWidth {
            value: 233,
            width: 1,
        })
    ));

    let truncated_unsigned = runtime
        .uint_to_bytevector(
            258,
            IntegerEncoding::fixed(ByteOrder::Big, integer_width(1)).truncating(),
        )
        .expect("explicitly truncate uint");
    assert_eq!(truncated_unsigned.to_vec().into_result(), Ok(vec![2]));

    let truncated_signed = runtime
        .sint_to_bytevector(
            233,
            IntegerEncoding::fixed(ByteOrder::Big, integer_width(1)).truncating(),
        )
        .expect("explicitly truncate sint");
    assert_eq!(truncated_signed.to_vec().into_result(), Ok(vec![233]));
    assert_eq!(
        truncated_signed
            .to_sint(IntegerDecoding::default())
            .as_result(),
        Ok(&-23)
    );

    let empty = runtime
        .bytevector_from_bytestring("", BytestringDelimiter::Compact)
        .expect("root empty bytevector");
    assert_eq!(
        empty.to_uint(IntegerDecoding::default()).as_result(),
        Ok(&0)
    );
    assert_eq!(
        empty.to_sint(IntegerDecoding::default()).as_result(),
        Ok(&0)
    );

    let nine_bytes = runtime
        .bytevector_from_bytestring("000000000000000000", BytestringDelimiter::Compact)
        .expect("root nine-byte vector");
    assert_eq!(
        nine_bytes.to_uint(IntegerDecoding::default()).status(),
        Some(GerbilStatus::InvalidValue)
    );
}

fn integer_width(width: u8) -> IntegerWidth {
    IntegerWidth::new(width).expect("test width is in 1..=8")
}

fn exports_scheme_objects_and_traverses_pairs(runtime: &GerbilRuntime) {
    exports_runtime_sentinel(runtime);
    exports_null_object(runtime);
    exports_void_object(runtime);
    exports_boolean_objects(runtime);
    exports_fixnum_object(runtime);
    exports_char_objects(runtime);
    exports_flonum_objects(runtime);
    exports_bytevector_object(runtime);
    exports_pair_object(runtime);
    exports_proper_list_object(runtime);
    exports_improper_list_object(runtime);
}

fn exports_runtime_sentinel(runtime: &GerbilRuntime) {
    let value = runtime.runtime_sentinel_value().unwrap();
    assert_eq!(value.provenance(), GerbilValueProvenance::RuntimeSentinel);
    assert_fail_closed_value(value);
}

fn exports_null_object(runtime: &GerbilRuntime) {
    let scheme_null = runtime
        .fixture_null_value()
        .expect("export Scheme null object through native runtime");
    assert_ne!(scheme_null.as_raw(), 0);
    assert_scheme_object_export(scheme_null);
    assert_eq!(scheme_null.is_pair().as_result(), Ok(&false));
    assert_eq!(scheme_null.is_list().as_result(), Ok(&true));
    assert_eq!(scheme_null.is_null().as_result(), Ok(&true));
    assert_eq!(scheme_null.is_void().as_result(), Ok(&false));
    let scheme_nil = scheme_null.as_nil().into_result().expect("project nil");
    assert_eq!(scheme_nil.as_raw(), scheme_null.as_raw());
    assert_eq!(
        scheme_null.as_void().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(scheme_null.is_boolean().as_result(), Ok(&false));
    assert_eq!(
        scheme_null.as_boolean().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_fail_closed_traversal(scheme_null);
}

fn exports_void_object(runtime: &GerbilRuntime) {
    let scheme_void = runtime
        .fixture_void_value()
        .expect("export Scheme void object through native runtime");
    assert_ne!(scheme_void.as_raw(), 0);
    assert_scheme_object_export(scheme_void);
    assert_eq!(scheme_void.is_pair().as_result(), Ok(&false));
    assert_eq!(scheme_void.is_list().as_result(), Ok(&false));
    assert_eq!(scheme_void.is_null().as_result(), Ok(&false));
    assert_eq!(
        scheme_void.as_nil().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(scheme_void.is_void().as_result(), Ok(&true));
    let projected_void = scheme_void
        .as_void()
        .into_result()
        .expect("project Scheme void");
    assert_eq!(projected_void.as_raw(), scheme_void.as_raw());
    assert_eq!(scheme_void.is_boolean().as_result(), Ok(&false));
    assert_eq!(
        scheme_void.as_boolean().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_fail_closed_traversal(scheme_void);
}

fn exports_boolean_objects(runtime: &GerbilRuntime) {
    let scheme_true = runtime
        .fixture_true_value()
        .expect("export Scheme true object through native runtime");
    assert_scheme_object_export(scheme_true);
    assert_eq!(scheme_true.is_pair().as_result(), Ok(&false));
    assert_eq!(scheme_true.is_list().as_result(), Ok(&false));
    assert_eq!(scheme_true.is_null().as_result(), Ok(&false));
    assert_eq!(scheme_true.is_boolean().as_result(), Ok(&true));
    assert_eq!(scheme_true.as_boolean().as_result(), Ok(&true));
    assert_fail_closed_traversal(scheme_true);
    assert_untrusted_raw_fail_closed(scheme_true);

    let scheme_false = runtime
        .fixture_false_value()
        .expect("export Scheme false object through native runtime");
    assert_scheme_object_export(scheme_false);
    assert_eq!(scheme_false.is_pair().as_result(), Ok(&false));
    assert_eq!(scheme_false.is_list().as_result(), Ok(&false));
    assert_eq!(scheme_false.is_null().as_result(), Ok(&false));
    assert_eq!(scheme_false.is_boolean().as_result(), Ok(&true));
    assert_eq!(scheme_false.as_boolean().as_result(), Ok(&false));
    assert_fail_closed_traversal(scheme_false);
    assert_untrusted_raw_fail_closed(scheme_false);
}

fn exports_fixnum_object(runtime: &GerbilRuntime) {
    let scheme_fixnum = runtime
        .fixture_fixnum_value()
        .expect("export Scheme fixnum object through native runtime");
    assert_scheme_object_export(scheme_fixnum);
    assert_eq!(scheme_fixnum.is_pair().as_result(), Ok(&false));
    assert_eq!(scheme_fixnum.is_list().as_result(), Ok(&false));
    assert_eq!(scheme_fixnum.is_null().as_result(), Ok(&false));
    assert_eq!(scheme_fixnum.is_boolean().as_result(), Ok(&false));
    assert_eq!(
        scheme_fixnum.as_boolean().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(scheme_fixnum.is_fixnum().as_result(), Ok(&true));
    assert_eq!(scheme_fixnum.as_fixnum().as_result(), Ok(&42));
    assert_eq!(scheme_fixnum.as_fixnum_i64().as_result(), Ok(&42));
    assert_fail_closed_traversal(scheme_fixnum);
    assert_untrusted_raw_fail_closed(scheme_fixnum);
}

fn exports_char_objects(runtime: &GerbilRuntime) {
    let ascii = runtime
        .fixture_char_ascii_value()
        .expect("export ASCII Scheme character through native runtime");
    assert_char_fixture(ascii, 'A');

    let bmp = runtime
        .fixture_char_bmp_value()
        .expect("export BMP Scheme character through native runtime");
    assert_char_fixture(bmp, 'λ');

    let non_bmp = runtime
        .fixture_char_non_bmp_value()
        .expect("export non-BMP Scheme character through native runtime");
    assert_char_fixture(non_bmp, '🙂');
}

fn assert_char_fixture(value: gerbil_scheme::GerbilValue<'_>, expected: char) {
    assert_scheme_object_export(value);
    assert_eq!(value.is_pair().as_result(), Ok(&false));
    assert_eq!(value.is_list().as_result(), Ok(&false));
    assert_eq!(value.is_null().as_result(), Ok(&false));
    assert_eq!(value.is_boolean().as_result(), Ok(&false));
    assert_eq!(
        value.as_boolean().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(value.is_fixnum().as_result(), Ok(&false));
    assert_eq!(value.as_fixnum().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.is_char().as_result(), Ok(&true));
    assert_eq!(value.as_char().as_result(), Ok(&expected));
    assert_fail_closed_traversal(value);
    assert_untrusted_raw_fail_closed(value);
}

fn exports_flonum_objects(runtime: &GerbilRuntime) {
    let finite = runtime
        .fixture_flonum_finite_value()
        .expect("export finite Scheme flonum through native runtime");
    assert_flonum_fixture(finite, |value| {
        assert_eq!(value.to_bits(), 42.5f64.to_bits());
    });

    let nan = runtime
        .fixture_flonum_nan_value()
        .expect("export NaN Scheme flonum through native runtime");
    assert_flonum_fixture(nan, |value| assert!(value.is_nan()));

    let pos_inf = runtime
        .fixture_flonum_pos_inf_value()
        .expect("export positive-infinity Scheme flonum through native runtime");
    assert_flonum_fixture(pos_inf, |value| {
        assert!(value.is_infinite());
        assert!(value.is_sign_positive());
    });

    let neg_inf = runtime
        .fixture_flonum_neg_inf_value()
        .expect("export negative-infinity Scheme flonum through native runtime");
    assert_flonum_fixture(neg_inf, |value| {
        assert!(value.is_infinite());
        assert!(value.is_sign_negative());
    });

    let neg_zero = runtime
        .fixture_flonum_neg_zero_value()
        .expect("export negative-zero Scheme flonum through native runtime");
    assert_flonum_fixture(neg_zero, |value| {
        assert_eq!(value.to_bits(), (-0.0f64).to_bits());
        assert!(value.is_sign_negative());
    });
}

fn assert_flonum_fixture(
    value: gerbil_scheme::GerbilValue<'_>,
    assert_projection: impl FnOnce(f64),
) {
    assert_scheme_object_export(value);
    assert_eq!(value.is_pair().as_result(), Ok(&false));
    assert_eq!(value.is_list().as_result(), Ok(&false));
    assert_eq!(value.is_null().as_result(), Ok(&false));
    assert_eq!(value.is_boolean().as_result(), Ok(&false));
    assert_eq!(
        value.as_boolean().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(value.is_fixnum().as_result(), Ok(&false));
    assert_eq!(value.as_fixnum().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.is_char().as_result(), Ok(&false));
    assert_eq!(value.as_char().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.is_flonum().as_result(), Ok(&true));
    assert_projection(*value.as_flonum().as_result().expect("project flonum"));
    assert_fail_closed_traversal(value);
    assert_untrusted_raw_fail_closed(value);
}

fn exports_bytevector_object(runtime: &GerbilRuntime) {
    let scheme_bytevector = runtime
        .fixture_bytevector_value()
        .expect("export Scheme bytevector object through native runtime");
    assert_ne!(scheme_bytevector.as_raw(), 0);
    assert_scheme_object_export(scheme_bytevector);
    assert_eq!(scheme_bytevector.is_pair().as_result(), Ok(&false));
    assert_eq!(scheme_bytevector.is_list().as_result(), Ok(&false));
    assert_eq!(scheme_bytevector.is_null().as_result(), Ok(&false));
    assert_eq!(scheme_bytevector.is_void().as_result(), Ok(&false));
    assert_eq!(
        scheme_bytevector.as_nil().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(
        scheme_bytevector.as_void().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(scheme_bytevector.is_bytevector().as_result(), Ok(&true));
    let projected = scheme_bytevector
        .as_bytevector()
        .into_result()
        .expect("project Scheme bytevector");
    assert_eq!(projected.as_raw(), scheme_bytevector.as_raw());
    assert_eq!(projected.len().as_result(), Ok(&5));
    assert_eq!(projected.is_empty().as_result(), Ok(&false));
    assert_eq!(projected.u8_at(0).as_result(), Ok(&255));
    assert_eq!(projected.u8_at(4).as_result(), Ok(&0));
    assert_eq!(
        projected.u8_at(5).status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(
        projected.to_vec().into_result().expect("copy bytevector"),
        vec![255, 127, 11, 1, 0]
    );
    let spaced = projected
        .to_bytestring(BytestringDelimiter::SPACE)
        .into_result()
        .expect("convert bytevector to spaced bytestring");
    assert_eq!(spaced.len().as_result(), Ok(&14));
    assert_eq!(spaced.char_at(0).as_result(), Ok(&'F'));
    assert_eq!(
        spaced.to_string().into_result().expect("copy bytestring"),
        "FF 7F 0B 01 00"
    );
    drop(spaced);

    let compact = projected
        .to_bytestring(BytestringDelimiter::Compact)
        .into_result()
        .expect("convert bytevector to compact bytestring");
    assert_eq!(
        compact
            .to_string()
            .into_result()
            .expect("copy compact bytestring"),
        "FF7F0B0100"
    );
    drop(compact);

    let parsed = runtime
        .bytevector_from_bytestring("FF AB 00", BytestringDelimiter::SPACE)
        .expect("parse spaced bytestring");
    assert_eq!(parsed.len().as_result(), Ok(&3));
    assert_eq!(parsed.u8_at(1).as_result(), Ok(&171));
    assert_eq!(
        parsed
            .to_vec()
            .into_result()
            .expect("copy parsed bytevector"),
        vec![255, 171, 0]
    );
    drop(parsed);

    let compact_parsed = runtime
        .bytevector_from_bytestring("010203", BytestringDelimiter::Compact)
        .expect("parse compact bytestring");
    assert_eq!(
        compact_parsed
            .to_vec()
            .into_result()
            .expect("copy compact parsed bytevector"),
        vec![1, 2, 3]
    );
    drop(compact_parsed);

    assert_eq!(
        runtime
            .bytevector_from_bytestring("GG", BytestringDelimiter::Compact)
            .expect_err("invalid hexadecimal bytestring must fail")
            .status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(scheme_bytevector.is_boolean().as_result(), Ok(&false));
    assert_eq!(
        scheme_bytevector.as_boolean().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_fail_closed_traversal(scheme_bytevector);
}

fn exports_pair_object(runtime: &GerbilRuntime) {
    let pair = runtime
        .fixture_pair_value()
        .expect("export Scheme pair object through native runtime");
    assert_scheme_object_export(pair);
    assert_eq!(pair.is_pair().as_result(), Ok(&true));
    assert_eq!(pair.is_list().as_result(), Ok(&false));
    assert_eq!(pair.is_null().as_result(), Ok(&false));
    let pair_head = pair.pair_car().into_result().expect("project pair car");
    let pair_tail = pair.pair_cdr().into_result().expect("project pair cdr");
    let pair_parts = pair.pair_parts().into_result().expect("project pair parts");
    assert_scheme_object_export(pair_head);
    assert_scheme_object_export(pair_tail);
    assert_eq!(pair_parts.car, pair_head);
    assert_eq!(pair_parts.cdr, pair_tail);
    assert_untrusted_raw_fail_closed(pair);
}

fn exports_proper_list_object(runtime: &GerbilRuntime) {
    let proper_list = runtime
        .fixture_proper_list_value()
        .expect("export proper Scheme list object through native runtime");
    assert_scheme_object_export(proper_list);
    assert_eq!(proper_list.is_pair().as_result(), Ok(&true));
    assert_eq!(proper_list.is_list().as_result(), Ok(&true));
    assert_eq!(proper_list.is_null().as_result(), Ok(&false));
    let proper_tail = proper_list
        .pair_cdr()
        .into_result()
        .expect("project proper-list cdr");
    assert_scheme_object_export(proper_tail);
    let proper_parts = proper_list
        .pair_parts()
        .into_result()
        .expect("project proper-list pair parts");
    assert_scheme_object_export(proper_parts.car);
    assert_eq!(proper_parts.cdr, proper_tail);
}

fn exports_improper_list_object(runtime: &GerbilRuntime) {
    let improper_list = runtime
        .fixture_improper_list_value()
        .expect("export improper Scheme list object through native runtime");
    assert_scheme_object_export(improper_list);
    assert_eq!(improper_list.is_pair().as_result(), Ok(&true));
    assert_eq!(improper_list.is_list().as_result(), Ok(&false));
    assert_eq!(improper_list.is_null().as_result(), Ok(&false));
    let improper_head = improper_list
        .pair_car()
        .into_result()
        .expect("project improper-list car");
    let improper_parts = improper_list
        .pair_parts()
        .into_result()
        .expect("project improper-list pair parts");
    assert_scheme_object_export(improper_head);
    assert_eq!(improper_parts.car, improper_head);
    assert_scheme_object_export(improper_parts.cdr);
}

fn reports_overflow_and_finalized_runtime_boundaries(runtime: GerbilRuntime) {
    assert_eq!(
        runtime.add_i64(i64::MAX, 1),
        Err(NativeError::IntegerOverflow {
            left: i64::MAX,
            right: 1,
        })
    );
    drop(runtime);

    assert!(matches!(
        GerbilRuntime::initialize(),
        Err(NativeError::RuntimeFinalized)
    ));
}

fn assert_scheme_object_export(value: gerbil_scheme::GerbilValue<'_>) {
    assert_eq!(
        value.provenance(),
        GerbilValueProvenance::SchemeObjectExport
    );
}

fn assert_fail_closed_value(value: gerbil_scheme::GerbilValue<'_>) {
    assert_eq!(value.is_pair().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.is_list().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.is_null().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.is_void().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(
        value.is_bytevector().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(
        value.is_boolean().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(
        value.as_boolean().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(value.is_fixnum().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(
        value.is_exact_integer().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(value.as_nil().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.as_void().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(
        value.as_bytevector().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(value.as_fixnum().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(
        value.as_exact_integer().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(
        value.as_fixnum_i64().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(value.is_char().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.as_char().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.is_flonum().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.as_flonum().status(), Some(GerbilStatus::InvalidValue));
    assert_fail_closed_traversal(value);
}

fn assert_untrusted_raw_fail_closed(value: gerbil_scheme::GerbilValue<'_>) {
    // This deliberately re-wraps an exported non-zero handle as UntrustedRaw
    // without dereferencing it, so the test proves provenance gates reject raw
    // handles even when their bits came from live Scheme objects.
    let untrusted =
        gerbil_scheme::GerbilValue::from_raw(value.as_raw()).expect("wrap exported handle as raw");
    assert_eq!(untrusted.provenance(), GerbilValueProvenance::UntrustedRaw);
    assert_fail_closed_value(untrusted);
}

fn assert_fail_closed_traversal(value: gerbil_scheme::GerbilValue<'_>) {
    assert_eq!(value.pair_car().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.pair_cdr().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(
        value.pair_parts().status(),
        Some(GerbilStatus::InvalidValue)
    );
}
