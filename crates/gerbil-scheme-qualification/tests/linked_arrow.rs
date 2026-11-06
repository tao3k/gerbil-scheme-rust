// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
#![cfg(feature = "arrow-aot")]

use arrow_array::builder::{
    Int32Builder, MapBuilder, StringBuilder, StringDictionaryBuilder, UnionBuilder,
};
use arrow_array::ffi::{FFI_ArrowArray, FFI_ArrowSchema};
use arrow_array::types::{
    ArrowPrimitiveType, Decimal256Type, Float16Type, Float64Type, Int8Type, Int32Type, Int64Type,
    IntervalDayTimeType, IntervalMonthDayNanoType,
};
use arrow_array::{
    Array, ArrayRef, BinaryArray, BinaryViewArray, BooleanArray, Date32Array, Date64Array,
    Decimal32Array, Decimal64Array, Decimal128Array, Decimal256Array, DurationSecondArray,
    FixedSizeBinaryArray, FixedSizeListArray, Float16Array, Float32Array, Float64Array, Int8Array,
    Int16Array, Int32Array, Int64Array, IntervalDayTimeArray, IntervalMonthDayNanoArray,
    IntervalYearMonthArray, LargeBinaryArray, LargeListArray, LargeListViewArray, LargeStringArray,
    ListArray, ListViewArray, NullArray, RecordBatch, RunArray, StringArray, StringViewArray,
    StructArray, Time32SecondArray, Time64MicrosecondArray, TimestampNanosecondArray, UInt8Array,
    UInt16Array, UInt32Array, UInt64Array,
};
use gerbil_scheme::{ArrowBatchExport, GerbilRuntime, SchemeScalar};
use std::collections::HashMap;
use std::sync::Arc;

unsafe extern "C" {
    fn gerbil_arrow_qualification_read(
        array: *const FFI_ArrowArray,
        schema: *const FFI_ArrowSchema,
    ) -> i64;
    fn gerbil_arrow_qualification_produce(
        array: *mut FFI_ArrowArray,
        schema: *mut FFI_ArrowSchema,
    ) -> i32;
    fn gerbil_arrow_qualification_produce_batch(
        array: *mut FFI_ArrowArray,
        schema: *mut FFI_ArrowSchema,
    ) -> i32;
    fn gerbil_arrow_qualification_fail(
        array: *mut FFI_ArrowArray,
        schema: *mut FFI_ArrowSchema,
    ) -> i32;
    fn gerbil_arrow_qualification_releases() -> i32;
}

#[test]
fn live_gerbil_owner_consumes_and_produces_arrow_c_data() {
    let program = gerbil_scheme_qualification::linked_arrow_program();
    let runtime = GerbilRuntime::initialize_program(program).unwrap();

    let batch = qualification_batch();
    let batch = ArrowBatchExport::new(&batch).unwrap();
    // SAFETY: the fixture borrows both descriptors, catches Scheme exceptions,
    // and returns one new strong root. Producer outputs use C-owned allocations.
    let consume = unsafe { runtime.bind_arrow_export(gerbil_arrow_qualification_read) }.unwrap();
    let produce =
        unsafe { runtime.bind_arrow_producer(gerbil_arrow_qualification_produce) }.unwrap();
    let fail = unsafe { runtime.bind_arrow_producer(gerbil_arrow_qualification_fail) }.unwrap();
    assert_eq!(
        consume
            .call(&batch)
            .into_result()
            .unwrap()
            .as_ref()
            .scalar()
            .unwrap(),
        SchemeScalar::Boolean(true)
    );
    assert!(fail.call().is_err());
    // SAFETY: this process-lived C accessor reads an atomic counter.
    assert_eq!(unsafe { gerbil_arrow_qualification_releases() }, 1);
    let (output, field) = produce.call().unwrap().into_array().unwrap();
    assert_eq!(field.name(), "scheme");
    assert_eq!(
        output
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .value(0),
        42
    );
    // The schema can be released immediately; the buffers retain the array.
    // SAFETY: this process-lived C accessor reads an atomic counter.
    assert_eq!(unsafe { gerbil_arrow_qualification_releases() }, 2);
    let scheme_batch = read_scheme_batch(&runtime);
    drop(runtime);
    std::thread::spawn(move || drop((output, scheme_batch)))
        .join()
        .unwrap();
    // SAFETY: this process-lived C accessor reads an atomic counter.
    assert_eq!(unsafe { gerbil_arrow_qualification_releases() }, 3);
}

fn read_scheme_batch(runtime: &GerbilRuntime) -> RecordBatch {
    // SAFETY: the Scheme export uses the SDK's C-owned physical layout
    // producer with correctly sized typed buffers.
    let produce =
        unsafe { runtime.bind_arrow_producer(gerbil_arrow_qualification_produce_batch) }.unwrap();
    let batch = produce.call().unwrap().into_batch().unwrap();
    assert_eq!(batch.num_rows(), 2);
    let list = batch
        .column(0)
        .as_any()
        .downcast_ref::<ListArray>()
        .unwrap();
    assert_eq!(list.value_offsets(), &[0, 2, 3]);
    let values = list.values().as_any().downcast_ref::<Int64Array>().unwrap();
    assert_eq!(values.values().as_ref(), &[7, 8, 9]);
    let flag = batch
        .column(1)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();
    assert!(!flag.value(0));
    assert!(flag.is_null(1));
    assert_eq!(batch.schema().field(2).dict_is_ordered(), Some(true));
    let labels = batch
        .column(2)
        .as_any()
        .downcast_ref::<arrow_array::DictionaryArray<Int8Type>>()
        .unwrap();
    assert_eq!(labels.keys().values().as_ref(), &[1, 0]);
    let words = labels
        .values()
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    assert_eq!(words.value(0), "箭");
    assert_eq!(words.value(1), "x");
    batch
}

fn qualification_batch() -> RecordBatch {
    let mut dict = StringDictionaryBuilder::<Int8Type>::new();
    dict.append_value("dictionary");
    dict.append_null();
    let runs = RunArray::<Int32Type>::try_new(
        &Int32Array::from(vec![2, 4]),
        &StringArray::from(vec!["a", "b"]),
    )
    .unwrap()
    .slice(1, 2);
    let base = [
        (
            "boolean",
            Arc::new(BooleanArray::from(vec![Some(true), Some(false), None]).slice(1, 2))
                as ArrayRef,
        ),
        ("uint64", Arc::new(UInt64Array::from(vec![u64::MAX, 0]))),
        ("utf8", Arc::new(StringArray::from(vec!["箭\0🦀", ""]))),
        (
            "list",
            Arc::new(ListArray::from_iter_primitive::<Int64Type, _, _>([
                Some(vec![Some(42), Some(-2)]),
                None,
            ])),
        ),
        (
            "decimal",
            Arc::new(
                Decimal128Array::from(vec![-123, 456])
                    .with_precision_and_scale(5, 2)
                    .unwrap(),
            ),
        ),
        (
            "view",
            Arc::new(StringViewArray::from(vec![
                "short",
                "a longer, external Arrow view",
            ])),
        ),
        ("dictionary", Arc::new(dict.finish())),
        ("run_end", Arc::new(runs)),
    ];
    let batch = RecordBatch::try_from_iter(base.into_iter().chain(other_columns())).unwrap();
    let schema = batch.schema();
    let mut fields = schema.fields().iter().cloned().collect::<Vec<_>>();
    fields[0] = Arc::new(
        fields[0]
            .as_ref()
            .clone()
            .with_metadata(HashMap::from([("k".to_owned(), "箭\0".to_owned())])),
    );
    let mut schema = schema.as_ref().clone();
    schema.fields = fields.into();
    batch.with_schema(Arc::new(schema)).unwrap()
}

#[allow(clippy::too_many_lines)]
fn other_columns() -> [(&'static str, ArrayRef); 36] {
    let values = [Some(vec![Some(11), None]), Some(vec![Some(21), Some(22)])];
    let half = <Float16Type as ArrowPrimitiveType>::Native::from_f32(-0.5);
    let wide = <Decimal256Type as ArrowPrimitiveType>::Native::from_i128(-(1_i128 << 80));
    let mut dense = UnionBuilder::new_dense();
    let mut sparse = UnionBuilder::new_sparse();
    for builder in [&mut dense, &mut sparse] {
        builder.append::<Int32Type>("int", -9).unwrap();
        builder.append::<Float64Type>("float", 0.5).unwrap();
    }
    let mut map = MapBuilder::new(None, StringBuilder::new(), Int32Builder::new());
    map.keys().append_value("key");
    map.values().append_value(9);
    map.append(true).unwrap();
    map.append(false).unwrap();
    let nested = StructArray::from(
        RecordBatch::try_from_iter([("child", Arc::new(Int32Array::from(vec![1, 2])) as ArrayRef)])
            .unwrap(),
    );
    [
        ("int8", Arc::new(Int8Array::from(vec![i8::MIN, i8::MAX]))),
        (
            "int16",
            Arc::new(Int16Array::from(vec![i16::MIN, i16::MAX])),
        ),
        (
            "int32",
            Arc::new(Int32Array::from(vec![i32::MIN, i32::MAX])),
        ),
        (
            "int64",
            Arc::new(Int64Array::from(vec![i64::MIN, i64::MAX])),
        ),
        ("uint8", Arc::new(UInt8Array::from(vec![u8::MAX, 0]))),
        ("uint16", Arc::new(UInt16Array::from(vec![u16::MAX, 0]))),
        ("uint32", Arc::new(UInt32Array::from(vec![u32::MAX, 0]))),
        ("float16", Arc::new(Float16Array::from(vec![half, half]))),
        ("float32", Arc::new(Float32Array::from(vec![1.25, 0.0]))),
        ("float64", Arc::new(Float64Array::from(vec![-2.5, 0.0]))),
        (
            "decimal32",
            Arc::new(
                Decimal32Array::from(vec![120, 0])
                    .with_precision_and_scale(3, 1)
                    .unwrap(),
            ),
        ),
        (
            "decimal64",
            Arc::new(
                Decimal64Array::from(vec![-250, 0])
                    .with_precision_and_scale(3, -1)
                    .unwrap(),
            ),
        ),
        (
            "decimal256",
            Arc::new(
                Decimal256Array::from(vec![wide, wide])
                    .with_precision_and_scale(50, 0)
                    .unwrap(),
            ),
        ),
        (
            "large_utf8",
            Arc::new(LargeStringArray::from(vec!["大", ""])),
        ),
        (
            "binary",
            Arc::new(BinaryArray::from_iter_values([&[0, 255][..], &[]])),
        ),
        (
            "large_binary",
            Arc::new(LargeBinaryArray::from_iter_values([&[1, 255][..], &[]])),
        ),
        (
            "fixed_binary",
            Arc::new(FixedSizeBinaryArray::try_from_iter([[2, 255], [0, 0]].into_iter()).unwrap()),
        ),
        (
            "binary_view",
            Arc::new(BinaryViewArray::from_iter_values([&[3, 255][..], &[]])),
        ),
        ("date32", Arc::new(Date32Array::from(vec![-10, 10]))),
        ("date64", Arc::new(Date64Array::from(vec![-86_400_000, 0]))),
        ("time32", Arc::new(Time32SecondArray::from(vec![42, 0]))),
        (
            "time64",
            Arc::new(Time64MicrosecondArray::from(vec![43, 0])),
        ),
        (
            "timestamp",
            Arc::new(TimestampNanosecondArray::from(vec![-44, 0]).with_timezone("Asia/Shanghai")),
        ),
        (
            "duration",
            Arc::new(DurationSecondArray::from(vec![-45, 0])),
        ),
        (
            "interval_ym",
            Arc::new(IntervalYearMonthArray::from(vec![-46, 0])),
        ),
        (
            "interval_dt",
            Arc::new(IntervalDayTimeArray::from(
                vec![IntervalDayTimeType::make_value(-3, 4); 2],
            )),
        ),
        (
            "interval_mdn",
            Arc::new(IntervalMonthDayNanoArray::from(
                vec![IntervalMonthDayNanoType::make_value(-3, 4, -5); 2],
            )),
        ),
        (
            "large_list",
            Arc::new(LargeListArray::from_iter_primitive::<Int64Type, _, _>(
                values.clone(),
            )),
        ),
        (
            "list_view",
            Arc::new(ListViewArray::from_iter_primitive::<Int64Type, _, _>(
                values.clone(),
            )),
        ),
        (
            "large_list_view",
            Arc::new(LargeListViewArray::from_iter_primitive::<Int64Type, _, _>(
                values.clone(),
            )),
        ),
        (
            "fixed_list",
            Arc::new(FixedSizeListArray::from_iter_primitive::<Int64Type, _, _>(
                values, 2,
            )),
        ),
        ("struct", Arc::new(nested)),
        ("map", Arc::new(map.finish())),
        ("dense_union", Arc::new(dense.build().unwrap())),
        ("sparse_union", Arc::new(sparse.build().unwrap())),
        ("null", Arc::new(NullArray::new(2))),
    ]
}
