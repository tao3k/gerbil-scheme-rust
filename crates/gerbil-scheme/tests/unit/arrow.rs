// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use super::ArrowBatchExport;
use arrow_array::ffi::from_ffi;
use arrow_array::{Array, Int64Array, RecordBatch, StructArray};
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
    let ArrowBatchExport { array, schema } = ArrowBatchExport::new(&batch).unwrap();
    drop(batch);
    let imported_schema = Schema::try_from(&schema).unwrap();
    assert_eq!(imported_schema.metadata()["source"], "test");
    // SAFETY: both descriptors came from the same typed batch and this
    // test consumes the array descriptor exactly once.
    let data = unsafe { from_ffi(array, &schema) }.unwrap();
    let structure = StructArray::from(data);
    let column = structure
        .column(0)
        .as_any()
        .downcast_ref::<Int64Array>()
        .unwrap();
    assert_eq!(column.values().as_ptr(), data_pointer);
    assert!(column.is_null(0));
    assert_eq!(column.value(1), i64::MAX);
}
