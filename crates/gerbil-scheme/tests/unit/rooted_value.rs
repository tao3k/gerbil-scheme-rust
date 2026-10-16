use gerbil_scheme::{
    ByteOrder, BytestringDelimiter, GerbilRuntime, IntegerDecoding, IntegerEncoding, IntegerWidth,
    RootedSchemeValue, RootedSchemeValueKind,
};
#[path = "object_contracts.rs"]
mod object_contracts;

#[test]
fn rooted_scheme_value_preserves_typed_projections_and_single_owner_drop() {
    let runtime = GerbilRuntime::initialize().expect("initialize live Gerbil runtime");
    let exact: RootedSchemeValue<'_> = runtime
        .exact_integer_from_i64(-23)
        .expect("root exact integer")
        .into();

    let fixture = runtime
        .fixture_bytevector_value()
        .expect("export bytevector fixture");
    let borrowed = fixture
        .as_bytevector()
        .into_result()
        .expect("project bytevector fixture");
    let string: RootedSchemeValue<'_> = borrowed
        .to_bytestring(BytestringDelimiter::SPACE)
        .into_result()
        .expect("root Scheme string")
        .into();

    let width = IntegerWidth::new(2).expect("two-byte integer width");
    let encoding = IntegerEncoding::fixed(ByteOrder::Big, width);
    let decoding = IntegerDecoding::entire(ByteOrder::Big);
    let bytevector: RootedSchemeValue<'_> = runtime
        .uint_to_bytevector(258, encoding)
        .expect("root Scheme bytevector")
        .into();

    let values = [exact, string, bytevector];
    assert_eq!(values[0].kind(), RootedSchemeValueKind::ExactInteger);
    assert_eq!(values[1].kind(), RootedSchemeValueKind::String);
    assert_eq!(values[2].kind(), RootedSchemeValueKind::Bytevector);

    assert_eq!(
        values[0]
            .as_exact_integer()
            .expect("typed exact integer")
            .to_i64()
            .into_result()
            .expect("project exact integer"),
        -23,
    );
    assert!(values[0].as_string().is_none());
    assert_eq!(
        values[1]
            .as_string()
            .expect("typed Scheme string")
            .to_string()
            .into_result()
            .expect("copy Scheme string"),
        "FF 7F 0B 01 00",
    );
    assert!(values[1].as_bytevector().is_none());
    assert_eq!(
        values[2]
            .as_bytevector()
            .expect("typed Scheme bytevector")
            .to_uint(decoding)
            .into_result()
            .expect("decode Scheme bytevector"),
        258,
    );
    assert!(values[2].as_exact_integer().is_none());

    let mut raw_root = gerbil_scheme_sys::GerbilRootId(0);
    let create_status = unsafe {
        gerbil_scheme_sys::gerbil_scheme_rust_i64_to_exact_integer_root(7, &raw mut raw_root)
    };
    assert_eq!(create_status, gerbil_scheme_sys::GerbilStatus::Ok);
    assert!(raw_root.is_valid());
    assert_eq!(
        unsafe { gerbil_scheme_sys::gerbil_scheme_rust_root_release(raw_root) },
        gerbil_scheme_sys::GerbilStatus::Ok,
    );
    assert_eq!(
        unsafe { gerbil_scheme_sys::gerbil_scheme_rust_root_release(raw_root) },
        gerbil_scheme_sys::GerbilStatus::InvalidValue,
        "the native root table must reject a second release",
    );
    population_survives_out_of_order_release();
    caller_owned_utf8_span_contracts(&runtime);
    scalar_batch_contracts(&runtime);
    object_contracts::run(&runtime);
}

fn scalar_batch_contracts(runtime: &GerbilRuntime) {
    use gerbil_scheme::{GerbilValue, ScalarBatch, SchemeScalar};
    let values = [
        runtime.fixture_true_value().unwrap(),
        runtime.fixture_false_value().unwrap(),
        runtime.fixture_fixnum_value().unwrap(),
        runtime.fixture_char_ascii_value().unwrap(),
        runtime.fixture_char_bmp_value().unwrap(),
        runtime.fixture_char_non_bmp_value().unwrap(),
        runtime.fixture_flonum_finite_value().unwrap(),
        runtime.fixture_flonum_nan_value().unwrap(),
        runtime.fixture_flonum_pos_inf_value().unwrap(),
        runtime.fixture_flonum_neg_inf_value().unwrap(),
        runtime.fixture_flonum_neg_zero_value().unwrap(),
    ];
    let mut batch = ScalarBatch::default();
    let mut output = vec![SchemeScalar::Boolean(false)];
    batch.project(runtime, &values, &mut output).unwrap();
    assert_eq!(output.len(), values.len());
    for (input, actual) in values.iter().zip(&output) {
        match actual {
            SchemeScalar::Boolean(v) => assert_eq!(*v, input.as_boolean().into_result().unwrap()),
            SchemeScalar::Fixnum(v) => assert_eq!(*v, input.as_fixnum().into_result().unwrap()),
            SchemeScalar::Char(v) => assert_eq!(*v, input.as_char().into_result().unwrap()),
            SchemeScalar::Flonum(v) => assert_eq!(
                v.to_bits(),
                input.as_flonum().into_result().unwrap().to_bits()
            ),
        }
    }
    let reserve = batch.reserved_bytes();
    for _ in 0..100 {
        batch.project(runtime, &values, &mut output).unwrap();
        assert_eq!(batch.reserved_bytes(), reserve);
    }
    let unchanged = vec![SchemeScalar::Char('x')];
    output.clone_from(&unchanged);
    let wrong = [values[0], runtime.fixture_pair_value().unwrap()];
    assert!(batch.project(runtime, &wrong, &mut output).is_err());
    assert_eq!(output, unchanged);
    for unsupported in [
        runtime.fixture_null_value().unwrap(),
        runtime.fixture_void_value().unwrap(),
        runtime.fixture_bytevector_value().unwrap(),
        runtime.fixture_proper_list_value().unwrap(),
        runtime.fixture_improper_list_value().unwrap(),
        runtime
            .fixture_exact_integer_large_positive_value()
            .unwrap(),
        runtime
            .fixture_exact_integer_large_negative_value()
            .unwrap(),
    ] {
        assert!(
            batch
                .project(runtime, &[values[0], unsupported], &mut output)
                .is_err()
        );
        assert_eq!(output, unchanged, "no partial publication or type coercion");
    }
    let untrusted = GerbilValue::from_raw(values[0].as_raw()).unwrap();
    assert!(batch.project(runtime, &[untrusted], &mut output).is_err());
    assert_eq!(output, unchanged);
    batch.project(runtime, &[], &mut output).unwrap();
    assert!(output.is_empty());
    raw_scalar_batch_contracts(runtime);
}

fn raw_scalar_batch_contracts(runtime: &GerbilRuntime) {
    use gerbil_scheme::GerbilValue;
    use gerbil_scheme_sys::{GerbilScalarProjection, GerbilStatus};
    let wrong = [
        runtime.fixture_true_value().unwrap(),
        runtime.fixture_pair_value().unwrap(),
    ];
    let sentinel = GerbilScalarProjection {
        tag: 99,
        reserved: 99,
        bits: 99,
    };
    let mut raw_output = [sentinel; 2];
    let raw_values = wrong.map(GerbilValue::as_raw);
    // SAFETY: valid live exports, owner thread, disjoint writable output.
    let status = unsafe {
        gerbil_scheme_sys::gerbil_scheme_rust_scalar_batch(
            raw_values.as_ptr(),
            2,
            raw_output.as_mut_ptr(),
        )
    };
    assert_eq!(status, GerbilStatus::InvalidValue);
    assert_eq!(raw_output, [sentinel; 2]);
    // Official ___FIX(0) is zero: a Scheme word is not a nullable pointer.
    let zero = 0_usize;
    let status = unsafe {
        gerbil_scheme_sys::gerbil_scheme_rust_scalar_batch(
            &raw const zero,
            1,
            raw_output.as_mut_ptr(),
        )
    };
    assert_eq!(status, GerbilStatus::Ok);
    assert_eq!(
        raw_output[0],
        GerbilScalarProjection {
            tag: 2,
            reserved: 0,
            bits: 0
        }
    );
    // SAFETY: empty spans are explicitly allowed; null nonempty input rejected.
    unsafe {
        assert_eq!(
            gerbil_scheme_sys::gerbil_scheme_rust_scalar_batch(
                std::ptr::null(),
                0,
                std::ptr::null_mut()
            ),
            GerbilStatus::Ok
        );
        assert_eq!(
            gerbil_scheme_sys::gerbil_scheme_rust_scalar_batch(
                std::ptr::null(),
                1,
                raw_output.as_mut_ptr()
            ),
            GerbilStatus::NullPointer
        );
    }
}

fn caller_owned_utf8_span_contracts(runtime: &GerbilRuntime) {
    use gerbil_scheme_sys::{GerbilRootId, GerbilStatus};
    let text = runtime.string_from_utf8("a\0汉字😀").unwrap();
    let string_root = acquire_string_root(runtime, "a\0汉字😀");
    let wrong = runtime.bytevector_from_bytes(b"wrong").unwrap();
    let mut output = [0xa5; 20];
    let mut written = usize::MAX;
    for (root, capacity, expected) in [
        (string_root, 19, GerbilStatus::InvalidValue),
        (string_root, usize::MAX, GerbilStatus::InvalidValue),
        (wrong.root_id(), 20, GerbilStatus::InvalidValue),
        (GerbilRootId(0), 20, GerbilStatus::InvalidValue),
    ] {
        // SAFETY: negative capacity/type controls must reject before writes.
        assert_eq!(
            unsafe {
                gerbil_scheme_sys::gerbil_scheme_rust_root_string_encode_into(
                    root,
                    output.as_mut_ptr(),
                    capacity,
                    &raw mut written,
                )
            },
            expected
        );
        assert_eq!(written, usize::MAX);
        assert_eq!(output, [0xa5; 20]);
    }
    // SAFETY: null control is rejected before any output access.
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_string_encode_into(
                string_root,
                std::ptr::null_mut(),
                20,
                &raw mut written,
            )
        },
        GerbilStatus::NullPointer
    );
    // SAFETY: valid exclusive span and separate output count on the live owner.
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_string_encode_into(
                string_root,
                output.as_mut_ptr(),
                output.len(),
                &raw mut written,
            )
        },
        GerbilStatus::Ok
    );
    assert_eq!(&output[..written], "a\0汉字😀".as_bytes());
    assert!(output[written..].iter().all(|byte| *byte == 0xa5));
    let owned = text.to_string().into_result().unwrap();
    assert_eq!(owned, "a\0汉字😀");
    assert!(owned.capacity() >= 20);
    let empty_root = acquire_string_root(runtime, "");
    // SAFETY: zero-length null span admits no write.
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_string_encode_into(
                empty_root,
                std::ptr::null_mut(),
                0,
                &raw mut written,
            )
        },
        GerbilStatus::Ok
    );
    assert_eq!(written, 0);
    for root in [string_root, empty_root] {
        // SAFETY: release exactly these test-owned independent roots once.
        assert_eq!(
            unsafe { gerbil_scheme_sys::gerbil_scheme_rust_root_release(root) },
            GerbilStatus::Ok
        );
        written = usize::MAX;
        let before = output;
        // SAFETY: released token is rejected without output access/publication.
        assert_eq!(
            unsafe {
                gerbil_scheme_sys::gerbil_scheme_rust_root_string_encode_into(
                    root,
                    output.as_mut_ptr(),
                    output.len(),
                    &raw mut written,
                )
            },
            GerbilStatus::InvalidValue
        );
        assert_eq!(written, usize::MAX);
        assert_eq!(output, before);
    }
}

fn acquire_string_root(runtime: &GerbilRuntime, text: &str) -> gerbil_scheme_sys::GerbilRootId {
    let input = runtime.bytevector_from_bytes(text.as_bytes()).unwrap();
    let mut root = gerbil_scheme_sys::GerbilRootId(0);
    // SAFETY: live owner/input and writable independent root; caller releases it.
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_utf8_to_string(
                input.root_id(),
                &raw mut root,
            )
        },
        gerbil_scheme_sys::GerbilStatus::Ok
    );
    root
}

// Run under the existing live owner: the process-global runtime cannot be
// independently initialized by another parallel test.
fn population_survives_out_of_order_release() {
    use gerbil_scheme_sys::{GerbilRootId, GerbilStatus};
    let mut roots = Vec::new();
    for index in 0..10_000_i64 {
        let mut root = GerbilRootId(0);
        let value = i64::MIN + index;
        // SAFETY: the enclosing test retains the live owner on this thread.
        assert_eq!(
            unsafe {
                gerbil_scheme_sys::gerbil_scheme_rust_i64_to_exact_integer_root(
                    value,
                    &raw mut root,
                )
            },
            GerbilStatus::Ok
        );
        roots.push((root, value));
    }
    for (index, &(root, _)) in roots.iter().enumerate() {
        if index % 2 == 1 {
            assert_eq!(
                unsafe { gerbil_scheme_sys::gerbil_scheme_rust_root_release(root) },
                GerbilStatus::Ok
            );
        }
    }
    for (index, &(root, expected)) in roots.iter().enumerate().rev() {
        let mut value = 0;
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_exact_integer_to_i64(root, &raw mut value)
        };
        if index % 2 == 0 {
            assert_eq!(status, GerbilStatus::Ok);
            assert_eq!(value, expected);
            assert_eq!(
                unsafe { gerbil_scheme_sys::gerbil_scheme_rust_root_release(root) },
                GerbilStatus::Ok
            );
        } else {
            assert_eq!(status, GerbilStatus::InvalidValue);
        }
        assert_eq!(
            unsafe { gerbil_scheme_sys::gerbil_scheme_rust_root_release(root) },
            GerbilStatus::InvalidValue
        );
    }
}
