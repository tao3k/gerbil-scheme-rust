// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use gerbil_scheme::{ExactIntegerTarget, GerbilRuntime, GerbilStatus, NativeError};

#[test]
fn exact_integer_handles_preserve_bignums_and_check_machine_projections() {
    let runtime = GerbilRuntime::initialize().expect("initialize in-process Gerbil runtime");

    let fixnum = runtime
        .fixture_fixnum_value()
        .expect("export fixnum fixture");
    assert_eq!(fixnum.is_exact_integer().as_result(), Ok(&true));
    let fixnum = fixnum
        .as_exact_integer()
        .into_result()
        .expect("project fixnum as exact integer");
    assert_eq!(fixnum.to_i64().as_result(), Ok(&42));
    assert_eq!(fixnum.to_u64().as_result(), Ok(&42));
    assert_eq!(fixnum.to_usize().as_result(), Ok(&42));

    let large_positive = runtime
        .fixture_exact_integer_large_positive_value()
        .expect("export positive bignum fixture");
    assert_eq!(large_positive.is_fixnum().as_result(), Ok(&false));
    let large_positive = large_positive
        .as_exact_integer()
        .into_result()
        .expect("project positive bignum as exact integer");
    assert_out_of_range(large_positive.to_i64(), ExactIntegerTarget::I64);
    assert_out_of_range(large_positive.to_u64(), ExactIntegerTarget::U64);
    assert_out_of_range(large_positive.to_usize(), ExactIntegerTarget::Usize);

    let large_negative = runtime
        .fixture_exact_integer_large_negative_value()
        .expect("export negative bignum fixture");
    assert_eq!(large_negative.is_fixnum().as_result(), Ok(&false));
    let large_negative = large_negative
        .as_exact_integer()
        .into_result()
        .expect("project negative bignum as exact integer");
    assert_out_of_range(large_negative.to_i64(), ExactIntegerTarget::I64);
    assert_out_of_range(large_negative.to_u64(), ExactIntegerTarget::U64);

    let flonum = runtime
        .fixture_flonum_finite_value()
        .expect("export flonum fixture");
    assert_eq!(flonum.is_exact_integer().as_result(), Ok(&false));
    assert_eq!(
        flonum.as_exact_integer().status(),
        Some(GerbilStatus::InvalidValue)
    );

    let untrusted = gerbil_scheme::GerbilValue::from_raw(fixnum.as_raw())
        .expect("wrap known non-zero bits as untrusted raw");
    assert_eq!(
        untrusted.is_exact_integer().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(
        untrusted.as_exact_integer().status(),
        Some(GerbilStatus::InvalidValue)
    );

    let signed_min = runtime
        .exact_integer_from_i64(i64::MIN)
        .expect("root i64::MIN");
    assert_eq!(signed_min.to_i64().as_result(), Ok(&i64::MIN));
    assert_out_of_range(signed_min.to_u64(), ExactIntegerTarget::U64);

    let signed_max = runtime
        .exact_integer_from_i64(i64::MAX)
        .expect("root i64::MAX");
    assert_eq!(signed_max.to_i64().as_result(), Ok(&i64::MAX));
    assert_eq!(signed_max.to_u64().as_result(), Ok(&(i64::MAX as u64)));

    let unsigned_max = runtime
        .exact_integer_from_u64(u64::MAX)
        .expect("root u64::MAX");
    assert_eq!(unsigned_max.to_u64().as_result(), Ok(&u64::MAX));
    assert_out_of_range(unsigned_max.to_i64(), ExactIntegerTarget::I64);

    let usize_max = runtime
        .exact_integer_from_u64(usize::MAX as u64)
        .expect("root usize::MAX");
    assert_eq!(usize_max.to_usize().as_result(), Ok(&usize::MAX));
}

fn assert_out_of_range<T: core::fmt::Debug>(
    result: gerbil_scheme::NativeResult<T>,
    target: ExactIntegerTarget,
) {
    assert!(matches!(
        result.into_result(),
        Err(NativeError::ExactIntegerOutOfRange { target: actual }) if actual == target
    ));
}
