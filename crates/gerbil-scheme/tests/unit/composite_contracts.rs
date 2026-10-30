//! Numeric tower precision, strong table equality and snapshot ownership.
use gerbil_scheme::{
    BigIntegerBuffer, BinaryInteger, GerbilRuntime, IdentifierBuffer, IdentifierKind, NumberKind,
    RootedObjectBatch, RootedSchemeObject, SchemeScalar, TableEquality,
};
use gerbil_scheme_sys as sys;
fn int(runtime: &GerbilRuntime, n: isize) -> RootedSchemeObject<'_> {
    runtime
        .root_value(runtime.fixnum_value(n).unwrap())
        .unwrap()
}
fn scalar(batch: &RootedObjectBatch<'_>, i: usize) -> isize {
    match batch.get(i).unwrap().scalar().unwrap() {
        SchemeScalar::Fixnum(n) => n,
        other => panic!("unexpected {other:?}"),
    }
}
pub(super) fn run(runtime: &GerbilRuntime) {
    let before = unsafe { sys::gerbil_scheme_rust_root_count_raw() };
    numbers(runtime);
    tables(runtime);
    exact_complex(runtime);
    floating_complex(runtime);
    assert_eq!(unsafe { sys::gerbil_scheme_rust_root_count_raw() }, before);
}
fn numbers(runtime: &GerbilRuntime) {
    let six = int(runtime, 6);
    let negative = int(runtime, -4);
    let zero = int(runtime, 0);
    let one = int(runtime, 1);
    let number = runtime
        .number_batch(NumberKind::Rational, &[(&six, &negative), (&zero, &one)])
        .unwrap();
    let mut parts = RootedObjectBatch::default();
    number
        .copy_number_components(NumberKind::Rational, &mut parts)
        .unwrap();
    drop(number);
    drop(six);
    drop(negative);
    assert_eq!(unsafe { sys::gerbil_scheme_rust_force_gc_raw() }, 0);
    assert_eq!((scalar(&parts, 0), scalar(&parts, 1)), (-3, 2));
    assert_eq!((scalar(&parts, 2), scalar(&parts, 3)), (0, 1));
    assert!(
        runtime
            .number_batch(NumberKind::Rational, &[(&one, &zero)])
            .is_err()
    );
    let wrong = runtime
        .intern_identifier(IdentifierKind::Symbol, "not-number")
        .unwrap();
    assert!(
        runtime
            .number_batch(NumberKind::Complex, &[(&wrong, &one)])
            .is_err()
    );
    assert!(
        wrong
            .copy_number_components(NumberKind::Rational, &mut parts)
            .is_err()
    );
    assert!(wrong.copy_table_entries(&mut parts).is_err());
    assert_eq!((scalar(&parts, 0), scalar(&parts, 1)), (-3, 2));
    for kind in [NumberKind::Rational, NumberKind::Complex] {
        let empty = runtime.number_batch(kind, &[]).unwrap();
        empty.copy_number_components(kind, &mut parts).unwrap();
        assert!(parts.is_empty());
    }
    let big = BinaryInteger::new(true, vec![0xff; 64]).unwrap();
    let numerator = runtime.big_integer(&big).unwrap();
    let rational = runtime
        .number_batch(NumberKind::Rational, &[(&numerator, &one)])
        .unwrap();
    rational
        .copy_number_components(NumberKind::Rational, &mut parts)
        .unwrap();
    drop(rational);
    drop(numerator);
    assert_eq!(unsafe { sys::gerbil_scheme_rust_force_gc_raw() }, 0);
    let mut binary = BigIntegerBuffer::default();
    parts.get(0).unwrap().copy_big_integer(&mut binary).unwrap();
    assert_eq!(binary.value(), &big);
    assert_eq!(scalar(&parts, 1), 1);
    let detached = parts.root_element(0).unwrap();
    parts.clear();
    assert_eq!(unsafe { sys::gerbil_scheme_rust_force_gc_raw() }, 0);
    detached.as_ref().copy_big_integer(&mut binary).unwrap();
    assert_eq!(binary.value(), &big);
    let real = int(runtime, 3);
    let imag = int(runtime, -5);
    let complex = runtime
        .number_batch(NumberKind::Complex, &[(&real, &imag)])
        .unwrap();
    complex
        .copy_number_components(NumberKind::Complex, &mut parts)
        .unwrap();
    assert_eq!((scalar(&parts, 0), scalar(&parts, 1)), (3, -5));
    assert!(
        complex
            .copy_number_components(NumberKind::Rational, &mut parts)
            .is_err()
    );
    assert_eq!((scalar(&parts, 0), scalar(&parts, 1)), (3, -5));
    let flonum = runtime
        .root_value(runtime.fixture_flonum_finite_value().unwrap())
        .unwrap();
    let SchemeScalar::Flonum(expected) = flonum.as_ref().scalar().unwrap() else {
        panic!("fixture type");
    };
    let floating = runtime
        .number_batch(NumberKind::Complex, &[(&flonum, &imag)])
        .unwrap();
    floating
        .copy_number_components(NumberKind::Complex, &mut parts)
        .unwrap();
    assert!(
        matches!(parts.get(0).unwrap().scalar().unwrap(), SchemeScalar::Flonum(x) if x.to_bits() == expected.to_bits())
    );
}
fn exact_complex(runtime: &GerbilRuntime) {
    let three = int(runtime, 3);
    let two = int(runtime, 2);
    let five = int(runtime, 5);
    let rational = runtime
        .number_batch(NumberKind::Rational, &[(&three, &two)])
        .unwrap();
    let mut parts = RootedObjectBatch::default();
    rational
        .copy_number_components(NumberKind::Complex, &mut parts)
        .unwrap();
    let real = parts.root_element(0).unwrap();
    let complex = runtime
        .number_batch(NumberKind::Complex, &[(&real, &five)])
        .unwrap();
    complex
        .copy_number_components(NumberKind::Complex, &mut parts)
        .unwrap();
    assert_eq!(scalar(&parts, 1), 5);
    let retained = parts.root_element(0).unwrap();
    drop(rational);
    drop(real);
    drop(complex);
    parts.clear();
    assert_eq!(unsafe { sys::gerbil_scheme_rust_force_gc_raw() }, 0);
    retained
        .copy_number_components(NumberKind::Rational, &mut parts)
        .unwrap();
    assert_eq!((scalar(&parts, 0), scalar(&parts, 1)), (3, 2));
}
fn retained_table(runtime: &GerbilRuntime) {
    let name = runtime
        .intern_identifier(IdentifierKind::Keyword, "retained")
        .unwrap();
    let zero = int(runtime, 0);
    let false_value = runtime
        .root_value(runtime.fixture_false_value().unwrap())
        .unwrap();
    let table = runtime
        .hash_table(
            TableEquality::Equivalent,
            &[(&zero, &false_value), (&false_value, &name)],
        )
        .unwrap();
    let mut entries = RootedObjectBatch::default();
    table.copy_table_entries(&mut entries).unwrap();
    drop(table);
    drop(zero);
    drop(name);
    drop(false_value);
    assert_eq!(unsafe { sys::gerbil_scheme_rust_force_gc_raw() }, 0);
    assert_eq!(entries.len(), 4);
    let mut names = IdentifierBuffer::default();
    let mut saw_zero = false;
    let mut saw_false = false;
    for i in (0..entries.len()).step_by(2) {
        match entries.get(i).unwrap().scalar().unwrap() {
            SchemeScalar::Fixnum(0) => {
                assert!(matches!(
                    entries.get(i + 1).unwrap().scalar().unwrap(),
                    SchemeScalar::Boolean(false)
                ));
                saw_zero = true;
            }
            SchemeScalar::Boolean(false) => {
                entries
                    .get(i + 1)
                    .unwrap()
                    .copy_identifier_names(&mut names)
                    .unwrap();
                assert_eq!(names.get(0), Some((IdentifierKind::Keyword, "retained")));
                saw_false = true;
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    assert!(saw_zero && saw_false);
    let empty = runtime.hash_table(TableEquality::Structural, &[]).unwrap();
    empty.copy_table_entries(&mut entries).unwrap();
    assert!(entries.is_empty());
}
fn tables(runtime: &GerbilRuntime) {
    let a = runtime.uninterned_symbol("same").unwrap();
    let b = runtime.uninterned_symbol("same").unwrap();
    let first = int(runtime, 1);
    let last = int(runtime, 9);
    for equality in [
        TableEquality::Identity,
        TableEquality::Equivalent,
        TableEquality::Structural,
    ] {
        let table = runtime
            .hash_table(equality, &[(&a, &first), (&b, &first), (&a, &last)])
            .unwrap();
        let mut entries = RootedObjectBatch::default();
        table.copy_table_entries(&mut entries).unwrap();
        assert_eq!(entries.len(), 4);
        let mut seen = (false, false);
        for i in (0..entries.len()).step_by(2) {
            let key = entries.get(i).unwrap();
            if key.is_identical(a.as_ref()) {
                assert_eq!(scalar(&entries, i + 1), 9);
                seen.0 = true;
            } else {
                assert!(key.is_identical(b.as_ref()));
                assert_eq!(scalar(&entries, i + 1), 1);
                seen.1 = true;
            }
        }
        assert_eq!(seen, (true, true));
    }
    retained_table(runtime);
    let mut entries = RootedObjectBatch::default();
    let pair1 = runtime
        .collection_from_objects(gerbil_scheme::CollectionKind::Pair, &[&first, &last])
        .unwrap()
        .into_object();
    let pair2 = runtime
        .collection_from_objects(gerbil_scheme::CollectionKind::Pair, &[&first, &last])
        .unwrap()
        .into_object();
    for (equality, count) in [(TableEquality::Identity, 4), (TableEquality::Structural, 2)] {
        let table = runtime
            .hash_table(equality, &[(&pair1, &first), (&pair2, &last)])
            .unwrap();
        table.copy_table_entries(&mut entries).unwrap();
        assert_eq!(entries.len(), count);
        if count == 2 {
            assert_eq!(scalar(&entries, 1), 9);
        }
    }
    let mut length = 123;
    assert_eq!(
        unsafe { sys::gerbil_scheme_rust_composite_snapshot(0, -1, 2, &raw mut length) },
        0
    );
    assert_eq!(length, 123);
    let missing = [0_i64; 2];
    assert_eq!(
        unsafe { sys::gerbil_scheme_rust_table_create(missing.as_ptr(), 1, 0) },
        0
    );
    assert_eq!(
        unsafe { sys::gerbil_scheme_rust_composite_create(missing.as_ptr(), 1, 0) },
        0
    );
}
fn floating_complex(runtime: &GerbilRuntime) {
    let imag = int(runtime, -5);
    for value in [
        runtime.fixture_flonum_finite_value().unwrap(),
        runtime.fixture_flonum_nan_value().unwrap(),
        runtime.fixture_flonum_neg_zero_value().unwrap(),
        runtime.fixture_flonum_pos_inf_value().unwrap(),
        runtime.fixture_flonum_neg_inf_value().unwrap(),
    ] {
        let root = runtime.root_value(value).unwrap();
        let SchemeScalar::Flonum(expected) = root.as_ref().scalar().unwrap() else {
            panic!("fixture type");
        };
        let complex = runtime
            .number_batch(NumberKind::Complex, &[(&root, &imag)])
            .unwrap();
        let mut parts = RootedObjectBatch::default();
        complex
            .copy_number_components(NumberKind::Complex, &mut parts)
            .unwrap();
        let SchemeScalar::Flonum(actual) = parts.get(0).unwrap().scalar().unwrap() else {
            panic!("component type");
        };
        if expected.is_nan() {
            assert!(actual.is_nan());
        } else {
            assert_eq!(actual.to_bits(), expected.to_bits());
        }
        assert!(
            runtime
                .number_batch(NumberKind::Rational, &[(&root, &imag)])
                .is_err()
        );
    }
}
