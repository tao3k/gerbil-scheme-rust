//! Root ownership, zero words, binary integers and complete collection copies.
use gerbil_scheme::{
    BigIntegerBuffer, BinaryInteger, CollectionKind, GerbilRuntime, RootedObjectBatch, SchemeScalar,
};
pub(super) fn run(runtime: &GerbilRuntime) {
    eprintln!("OBJECT-CONTRACT phase=count");
    let before = count();
    eprintln!("OBJECT-CONTRACT phase=zero");
    zero(runtime);
    eprintln!("OBJECT-CONTRACT phase=integers");
    integers(runtime);
    eprintln!("OBJECT-CONTRACT phase=collections");
    collections(runtime);
    numeric(runtime);
    rejected_spans();
    assert_eq!(
        count(),
        before,
        "no root leak after the complete ownership matrix"
    );
}
fn numeric_check<T: gerbil_scheme::NumericElement>(runtime: &GerbilRuntime, data: &[T]) {
    fn bytes<T>(data: &[T]) -> &[u8] {
        // SAFETY: callers use only sealed numeric primitives, with no padding.
        unsafe { std::slice::from_raw_parts(data.as_ptr().cast(), std::mem::size_of_val(data)) }
    }
    let before = count();
    let root = runtime.numeric_vector(data).unwrap();
    let mut output = gerbil_scheme::NumericBuffer::<T>::default();
    root.as_ref().copy_numeric(&mut output).unwrap();
    assert_eq!(bytes(output.as_slice()), bytes(data));
    let collection = runtime
        .collection_from_objects(CollectionKind::Vector, &[&root])
        .unwrap();
    let mut snapshot = RootedObjectBatch::default();
    collection.copy_into(&mut snapshot).unwrap();
    drop(collection);
    drop(root);
    assert_eq!(
        unsafe { gerbil_scheme_sys::gerbil_scheme_rust_force_gc_raw() },
        0
    );
    snapshot.get(0).unwrap().copy_numeric(&mut output).unwrap();
    assert_eq!(bytes(output.as_slice()), bytes(data));
    let capacity = output.reserved_bytes();
    snapshot.get(0).unwrap().copy_numeric(&mut output).unwrap();
    assert_eq!(output.reserved_bytes(), capacity);
    let wrong = runtime
        .root_value(runtime.fixnum_value(0).unwrap())
        .unwrap();
    assert!(wrong.as_ref().copy_numeric(&mut output).is_err());
    assert_eq!(bytes(output.as_slice()), bytes(data));
    drop(wrong);
    drop(snapshot);
    assert_eq!(count(), before);
}
fn numeric(runtime: &GerbilRuntime) {
    use numeric_check as check;
    numeric_growth(runtime);
    for len in [0, 1, 3, 1024] {
        check(runtime, &vec![u16::MAX; len]);
        check(runtime, &vec![u32::MAX; len]);
        check(runtime, &vec![u64::MAX; len]);
        check(runtime, &vec![f32::from_bits(0x7fc0_1234); len]);
        check(runtime, &vec![f64::from_bits(0x7ff8_0000_1234_5678); len]);
    }
    check(
        runtime,
        &[
            0.0_f32,
            -0.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::MIN_POSITIVE,
        ],
    );
    check(
        runtime,
        &[
            0.0_f64,
            -0.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MIN_POSITIVE,
        ],
    );
    // Raw ABI: wrong-family, out-of-range and null spans publish no bytes.
    let raw = unsafe {
        gerbil_scheme_sys::gerbil_scheme_rust_numeric_create(0, [1_u16, 65535].as_ptr().cast(), 2)
    };
    assert!(raw > 0);
    let mut bytes = [0xa5_u8; 8];
    for (kind, offset, len) in [(1, 0, 1), (0, 2, 1), (0, u64::MAX, 1)] {
        assert_eq!(
            unsafe {
                gerbil_scheme_sys::gerbil_scheme_rust_numeric_copy(
                    raw,
                    -1,
                    kind,
                    bytes.as_mut_ptr(),
                    offset,
                    len,
                )
            },
            -1
        );
        assert_eq!(bytes, [0xa5; 8]);
    }
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_numeric_copy(
                raw,
                -1,
                0,
                std::ptr::null_mut(),
                0,
                1,
            )
        },
        -1
    );
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_numeric_copy(
                raw,
                -1,
                0,
                bytes[1..].as_mut_ptr(),
                0,
                2,
            )
        },
        0
    );
    let expected: Vec<_> = [1_u16, u16::MAX]
        .into_iter()
        .flat_map(u16::to_ne_bytes)
        .collect();
    assert_eq!(&bytes[1..5], expected);
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_release(gerbil_scheme_sys::GerbilRootId(raw))
        },
        gerbil_scheme_sys::GerbilStatus::Ok
    );
}
fn numeric_growth(runtime: &GerbilRuntime) {
    let mut output = gerbil_scheme::NumericBuffer::<u64>::default();
    for len in [0, 1, 3, 1024, 2, 0, 32] {
        let input: Vec<_> = (0..len).map(|index| u64::MAX - index).collect();
        let root = runtime.numeric_vector(&input).unwrap();
        root.as_ref().copy_numeric(&mut output).unwrap();
        assert_eq!(output.as_slice(), input);
        let wrong = runtime.numeric_vector(&[1_u32, 2]).unwrap();
        let before = output.reserved_bytes();
        assert!(wrong.as_ref().copy_numeric(&mut output).is_err());
        assert_eq!(output.as_slice(), input);
        assert_eq!(output.reserved_bytes(), before);
    }
}
fn rejected_spans() {
    use gerbil_scheme_sys::{self as sys, GerbilRootId};
    let before = count();
    // SAFETY: owner is live; empty input is never dereferenced.
    let root = unsafe { sys::gerbil_scheme_rust_collection_create_raw(std::ptr::null(), 0, 0) };
    assert!(root > 0);
    // A mismatched capacity must neither publish roots nor leak them.
    assert_eq!(
        unsafe { sys::gerbil_scheme_rust_collection_snapshot_raw(root, 0, 1) },
        0
    );
    let mut output = sys::GerbilScalarProjection {
        tag: 123,
        reserved: 456,
        bits: 789,
    };
    assert_eq!(
        unsafe { sys::gerbil_scheme_rust_object_scalar_indexed(root, 0, &raw mut output) },
        3
    );
    assert_eq!((output.tag, output.reserved, output.bits), (123, 456, 789));
    assert_eq!(count(), before + 1);
    // Reject duplicate tokens transactionally; the first token remains live.
    let duplicate = [GerbilRootId(root), GerbilRootId(root)];
    assert_eq!(
        unsafe { sys::gerbil_scheme_rust_roots_release_raw(duplicate.as_ptr(), 2) },
        3
    );
    assert_eq!(count(), before + 1);
    assert_eq!(
        unsafe { sys::gerbil_scheme_rust_roots_release_raw(duplicate.as_ptr(), 1) },
        0
    );
    assert_eq!(count(), before);
}
fn count() -> i64 {
    // SAFETY: caller's live owner spans every test in this module.
    unsafe { gerbil_scheme_sys::gerbil_scheme_rust_root_count_raw() }
}
fn zero(runtime: &GerbilRuntime) {
    for number in [0, 1, -1, 42, -42] {
        let value = runtime.fixnum_value(number).unwrap();
        assert_eq!(value.as_fixnum().into_result().unwrap(), number);
        assert_eq!(
            value
                .as_exact_integer()
                .into_result()
                .unwrap()
                .to_i64()
                .into_result()
                .unwrap(),
            i64::try_from(number).unwrap()
        );
        let root = runtime.root_value(value).unwrap();
        assert_eq!(
            root.as_ref().scalar().unwrap(),
            SchemeScalar::Fixnum(number)
        );
    }
    assert!(runtime.fixnum_value(isize::MAX).is_err());
    assert!(runtime.fixnum_value(isize::MIN).is_err());
    let root = runtime
        .root_value(runtime.fixture_false_value().unwrap())
        .unwrap();
    assert_eq!(
        root.as_ref().scalar().unwrap(),
        SchemeScalar::Boolean(false)
    );
    drop(root); // A false Scheme value must not look like a missing registry entry.
}
fn integers(runtime: &GerbilRuntime) {
    assert!(BinaryInteger::new(true, vec![]).is_err());
    assert!(BinaryInteger::new(false, vec![0]).is_err());
    let mut output = BigIntegerBuffer::default();
    for byte in [1, 127, 128, 255] {
        for negative in [false, true] {
            let value = BinaryInteger::new(negative, vec![byte]).unwrap();
            let root = runtime.big_integer(&value).unwrap();
            root.as_ref().copy_big_integer(&mut output).unwrap();
            assert_eq!(output.value(), &value);
            let number = isize::from(byte) * if negative { -1 } else { 1 };
            assert_eq!(
                root.as_ref().scalar().unwrap(),
                SchemeScalar::Fixnum(number)
            );
        }
    }
    for size in [0, 1, 7, 8, 9, 16, 17, 32, 64, 257, 1024] {
        eprintln!("OBJECT-CONTRACT integer_bytes={size}");
        for negative in [false, true] {
            if size == 0 && negative {
                continue;
            }
            let bytes: Vec<u8> = (1..=u8::MAX).cycle().take(size).collect();
            let value = BinaryInteger::new(negative, bytes).unwrap();
            let root = runtime.big_integer(&value).unwrap();
            root.as_ref().copy_big_integer(&mut output).unwrap();
            assert_eq!(output.value(), &value);
            // SAFETY: live owner; retained roots must survive normal SDK GC.
            assert_eq!(
                unsafe { gerbil_scheme_sys::gerbil_scheme_rust_force_gc_raw() },
                0
            );
            root.as_ref().copy_big_integer(&mut output).unwrap();
            assert_eq!(output.value(), &value);
        }
    }
    let unchanged = output.value().clone();
    let wrong = runtime
        .root_value(runtime.fixture_true_value().unwrap())
        .unwrap();
    assert!(wrong.as_ref().copy_big_integer(&mut output).is_err());
    assert_eq!(output.value(), &unchanged);
}
fn collections(runtime: &GerbilRuntime) {
    for kind in [
        CollectionKind::Vector,
        CollectionKind::List,
        CollectionKind::Pair,
    ] {
        eprintln!("OBJECT-CONTRACT collection={kind:?} phase=input");
        let zero = runtime
            .root_value(runtime.fixnum_value(0).unwrap())
            .unwrap();
        eprintln!("OBJECT-CONTRACT collection={kind:?} phase=zero-rooted");
        let false_value = runtime
            .root_value(runtime.fixture_false_value().unwrap())
            .unwrap();
        eprintln!("OBJECT-CONTRACT collection={kind:?} phase=false-rooted");
        let collection = runtime
            .collection_from_objects(kind, &[&zero, &false_value])
            .unwrap();
        eprintln!("OBJECT-CONTRACT collection={kind:?} phase=copy");
        drop(zero);
        drop(false_value);
        let mut batch = RootedObjectBatch::default();
        let before_copy = count();
        collection.copy_into(&mut batch).unwrap();
        assert_eq!(
            count(),
            before_copy + 1,
            "one root for the complete snapshot"
        );
        let reserve = batch.reserved_bytes();
        for _ in 0..50 {
            collection.copy_into(&mut batch).unwrap();
            assert_eq!(count(), before_copy + 1);
        }
        assert_eq!(batch.reserved_bytes(), reserve);
        assert_eq!(
            batch.get(0).unwrap().scalar().unwrap(),
            SchemeScalar::Fixnum(0)
        );
        assert_eq!(
            batch.get(1).unwrap().scalar().unwrap(),
            SchemeScalar::Boolean(false)
        );
        assert!(batch.get(2).is_none());
        assert!(collection.element(2).is_err());
        let element = collection.element(0).unwrap();
        assert_eq!(element.as_ref().scalar().unwrap(), SchemeScalar::Fixnum(0));
        drop(element);
        drop(collection);
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_force_gc_raw();
        }
        assert_eq!(
            batch.get(1).unwrap().scalar().unwrap(),
            SchemeScalar::Boolean(false)
        );
    }
    let empty = runtime
        .collection_from_objects(CollectionKind::Vector, &[])
        .unwrap();
    assert!(empty.is_empty());
    let mut output = RootedObjectBatch::default();
    empty.copy_into(&mut output).unwrap();
    assert!(output.is_empty());
    assert!(
        runtime
            .collection_from_objects(CollectionKind::Pair, &[])
            .is_err()
    );
    let nested = runtime
        .collection_from_objects(CollectionKind::Vector, &[])
        .unwrap()
        .into_object();
    let parent = runtime
        .collection_from_objects(CollectionKind::List, &[&nested])
        .unwrap();
    drop(nested);
    parent.copy_into(&mut output).unwrap();
    assert!(output.get(0).unwrap().scalar().is_err()); // Nested collections are not scalars.
    let number = BinaryInteger::new(true, vec![255; 257]).unwrap();
    let integer = runtime.big_integer(&number).unwrap();
    let collection = runtime
        .collection_from_objects(CollectionKind::Vector, &[&integer])
        .unwrap();
    collection.copy_into(&mut output).unwrap();
    drop(integer);
    drop(collection);
    unsafe {
        gerbil_scheme_sys::gerbil_scheme_rust_force_gc_raw();
    }
    let mut copied = BigIntegerBuffer::default();
    output
        .get(0)
        .unwrap()
        .copy_big_integer(&mut copied)
        .unwrap();
    assert_eq!(copied.value(), &number);
}
