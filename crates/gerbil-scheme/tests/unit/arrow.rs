// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use super::ArrowBatchExport;
use arrow_array::builder::StringDictionaryBuilder;
use arrow_array::types::Int8Type;
use arrow_array::{Array, Int64Array, RecordBatch, RecordBatchOptions};
use arrow_schema::{DataType, Field, Schema};
use std::collections::HashMap;
use std::sync::Arc;

#[test]
fn shares_column_storage_after_the_record_batch_is_dropped() {
    let column = Int64Array::from(vec![Some(1), None, Some(i64::MAX)]).slice(1, 2);
    let data_pointer = column.values().as_ptr();
    let schema = Schema::new(vec![Field::new("value", DataType::Int64, true)])
        .with_metadata(HashMap::from([("source".to_owned(), "test".to_owned())]));
    let batch = RecordBatch::try_new(Arc::new(schema), vec![Arc::new(column)]).unwrap();
    let export = ArrowBatchExport::new(&batch).unwrap();
    drop(batch);
    let imported = export.into_batch().unwrap();
    assert_eq!(imported.schema().metadata()["source"], "test");
    let column = imported
        .column(0)
        .as_any()
        .downcast_ref::<Int64Array>()
        .unwrap();
    assert_eq!(column.values().as_ptr(), data_pointer);
    assert!(column.is_null(0));
    assert_eq!(column.value(1), i64::MAX);
}

#[test]
fn preserves_empty_batch_row_count() {
    let options = RecordBatchOptions::new().with_row_count(Some(3));
    let batch =
        RecordBatch::try_new_with_options(Arc::new(Schema::empty()), vec![], &options).unwrap();
    let imported = ArrowBatchExport::new(&batch).unwrap().into_batch().unwrap();
    assert_eq!(imported.num_rows(), 3);
    assert_eq!(imported.num_columns(), 0);
}

#[test]
fn retains_ordered_dictionary_field_metadata() {
    let mut values = StringDictionaryBuilder::<Int8Type>::new();
    values.append_value("ordered");
    let values = values.finish();
    let field = Field::new("labels", values.data_type().clone(), false)
        .with_dict_is_ordered(true)
        .with_metadata(HashMap::from([("source".to_owned(), "fixture".to_owned())]));
    let batch =
        RecordBatch::try_new(Arc::new(Schema::new(vec![field])), vec![Arc::new(values)]).unwrap();
    let imported = ArrowBatchExport::new(&batch).unwrap().into_batch().unwrap();
    assert_eq!(imported.schema().field(0).dict_is_ordered(), Some(true));
    assert_eq!(imported.schema().field(0).metadata()["source"], "fixture");
}
