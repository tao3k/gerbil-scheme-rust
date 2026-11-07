// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Call-scoped Arrow C Data input for an already initialized Scheme owner.

use super::types::RootedSchemeOwner;
use super::{GerbilRuntime, NativeError, NativeResult, RootedSchemeObject};
use arrow_array::ffi::{FFI_ArrowArray, FFI_ArrowSchema, from_ffi_and_data_type};
use arrow_array::{Array, ArrayRef, RecordBatch, RecordBatchOptions, StructArray, make_array};
use arrow_schema::{ArrowError, DataType, Field, Schema, UnionFields};
use std::sync::Arc;

/// Owns an Arrow C Data export. The descriptors keep the original column
/// allocations alive. Construct once to reuse the metadata on later calls.
#[derive(Debug)]
pub struct ArrowBatchExport {
    data: ArrowArrayExport,
}

impl ArrowBatchExport {
    /// Export the schema and share the batch's existing column buffers.
    ///
    /// # Errors
    ///
    /// Returns an Arrow error if the schema cannot be represented in C Data.
    pub fn new(batch: &RecordBatch) -> Result<Self, ArrowError> {
        let schema = FFI_ArrowSchema::try_from(batch.schema_ref().as_ref())?;
        let array = FFI_ArrowArray::new(&StructArray::from(batch.clone()).to_data());
        Ok(Self {
            data: ArrowArrayExport { array, schema },
        })
    }
    /// Consume the C Data export as a native batch.
    ///
    /// # Errors
    /// Returns an Arrow error if importing the exported batch fails.
    pub fn into_batch(self) -> Result<RecordBatch, ArrowError> {
        self.data.into_batch()
    }
}

/// One initialized Scheme export that reads an Arrow batch on its owner thread.
#[derive(Clone, Copy, Debug)]
pub struct LinkedArrowExport<'runtime> {
    runtime: &'runtime GerbilRuntime,
    export: unsafe extern "C" fn(*const FFI_ArrowArray, *const FFI_ArrowSchema) -> i64,
}

impl GerbilRuntime {
    /// Bind an application export with read-only, synchronous Arrow C Data input.
    ///
    /// # Errors
    ///
    /// Rejects a call outside the runtime owner thread.
    ///
    /// # Safety
    ///
    /// The export must belong to this runtime's initialized AOT graph and live
    /// for the process. It may read descriptors and buffers only during the
    /// call. It must not modify or release them, keep pointers, reenter Rust,
    /// or finalize the runtime. It must catch Scheme exceptions, returning
    /// zero on failure or transferring a fresh strong result root on success.
    pub unsafe fn bind_arrow_export(
        &self,
        export: unsafe extern "C" fn(*const FFI_ArrowArray, *const FFI_ArrowSchema) -> i64,
    ) -> Result<LinkedArrowExport<'_>, NativeError> {
        self.check_thread()?;
        Ok(LinkedArrowExport {
            runtime: self,
            export,
        })
    }
}

impl<'runtime> LinkedArrowExport<'runtime> {
    /// Borrow the batch for one Scheme call. The batch remains owned by Rust
    /// on both success and error, including a zero result token.
    #[must_use]
    pub fn call(&self, batch: &ArrowBatchExport) -> NativeResult<RootedSchemeObject<'runtime>> {
        self.call_array(&batch.data)
    }

    /// Borrow one Arrow array for a Scheme export using the array view.
    #[must_use]
    pub fn call_array(
        &self,
        array: &ArrowArrayExport,
    ) -> NativeResult<RootedSchemeObject<'runtime>> {
        let result = (|| {
            self.runtime.check_thread()?;
            // SAFETY: binding guarantees the synchronous borrow and unique
            // result root; batch keeps all Arrow allocations alive.
            let root = gerbil_scheme_sys::GerbilRootId(unsafe {
                (self.export)(&raw const array.array, &raw const array.schema)
            });
            RootedSchemeOwner::new(
                gerbil_scheme_sys::GerbilStatus::Ok,
                root,
                "linked Arrow export",
            )
            .map(RootedSchemeObject::from_owner)
        })();
        result.into()
    }
}

/// Owns the C Data descriptors for an Arrow array and its field metadata.
#[derive(Debug)]
pub struct ArrowArrayExport {
    array: FFI_ArrowArray,
    schema: FFI_ArrowSchema,
}

impl ArrowArrayExport {
    /// Share an Arrow array's buffers with the supplied field metadata.
    ///
    /// # Errors
    /// Returns an error for a type mismatch or an unsupported C Data schema.
    pub fn new(array: &dyn Array, field: &Field) -> Result<Self, ArrowError> {
        if array.data_type() != field.data_type()
            || (!field.is_nullable() && array.logical_null_count() != 0)
        {
            return Err(ArrowError::InvalidArgumentError(
                "Arrow array does not match the field".to_owned(),
            ));
        }
        Ok(Self {
            schema: FFI_ArrowSchema::try_from(field)?,
            array: FFI_ArrowArray::new(&array.to_data()),
        })
    }

    /// Consume both descriptors and retain the imported buffers and field metadata.
    ///
    /// # Errors
    /// Returns an Arrow error if the schema or logical array data is invalid.
    pub fn into_array(self) -> Result<(ArrayRef, Field), ArrowError> {
        let field = Field::try_from(&self.schema)?;
        let field = restore_dictionary_order(&self.schema, &field)?;
        // SAFETY: constructors require an owned, conforming C Data export.
        let data = unsafe { from_ffi_and_data_type(self.array, field.data_type().clone()) }?;
        Ok((make_array(data), field))
    }

    /// Consume a struct export as a record batch, preserving schema metadata.
    ///
    /// # Errors
    /// Returns an Arrow error if the export is invalid or cannot represent a batch.
    pub fn into_batch(self) -> Result<RecordBatch, ArrowError> {
        let schema = Schema::try_from(&self.schema)?;
        let fields = self
            .schema
            .children()
            .zip(schema.fields())
            .map(|(c_schema, field)| restore_dictionary_order(c_schema, field))
            .collect::<Result<Vec<_>, _>>()?;
        let data_type = DataType::Struct(fields.into());
        // SAFETY: constructors require an owned, conforming C Data export.
        let data = unsafe { from_ffi_and_data_type(self.array, data_type) }?;
        let structure = StructArray::from(data);
        if structure.null_count() != 0 {
            return Err(ArrowError::InvalidArgumentError(
                "Arrow record batches cannot have null rows".to_owned(),
            ));
        }
        let options = RecordBatchOptions::new().with_row_count(Some(structure.len()));
        let schema = Schema::new(structure.fields().clone()).with_metadata(schema.metadata);
        RecordBatch::try_new_with_options(Arc::new(schema), structure.columns().to_vec(), &options)
    }
}

// arrow-rs imports dictionary order on the schema view, but does not
// carry the flag into Field::try_from. Restore it at every nested field.
fn restore_dictionary_order(schema: &FFI_ArrowSchema, field: &Field) -> Result<Field, ArrowError> {
    let data_type = restore_nested_order(schema, field.data_type())?;
    Ok(field
        .clone()
        .with_data_type(data_type)
        .with_dict_is_ordered(schema.dictionary_ordered()))
}

fn restore_nested_order(
    schema: &FFI_ArrowSchema,
    data_type: &DataType,
) -> Result<DataType, ArrowError> {
    let child = |field: &Field| restore_dictionary_order(schema.child(0), field).map(Arc::new);
    let fields = |fields: &arrow_schema::Fields| {
        schema
            .children()
            .zip(fields)
            .map(|(c_schema, field)| restore_dictionary_order(c_schema, field))
            .collect::<Result<arrow_schema::Fields, _>>()
    };
    Ok(match data_type {
        DataType::List(field) => DataType::List(child(field)?),
        DataType::LargeList(field) => DataType::LargeList(child(field)?),
        DataType::ListView(field) => DataType::ListView(child(field)?),
        DataType::LargeListView(field) => DataType::LargeListView(child(field)?),
        DataType::FixedSizeList(field, size) => DataType::FixedSizeList(child(field)?, *size),
        DataType::Map(field, sorted) => DataType::Map(child(field)?, *sorted),
        DataType::Struct(children) => DataType::Struct(fields(children)?),
        DataType::Union(union, mode) => {
            let children = schema
                .children()
                .zip(union.iter())
                .map(|(c_schema, (_, field))| restore_dictionary_order(c_schema, field))
                .collect::<Result<Vec<_>, _>>()?;
            DataType::Union(
                UnionFields::try_new(union.iter().map(|(id, _)| id), children)?,
                *mode,
            )
        }
        DataType::RunEndEncoded(ends, values) => DataType::RunEndEncoded(
            child(ends)?,
            Arc::new(restore_dictionary_order(schema.child(1), values)?),
        ),
        DataType::Dictionary(keys, values) => {
            let dictionary = schema.dictionary().ok_or_else(|| {
                ArrowError::CDataInterface("missing Arrow dictionary schema".to_owned())
            })?;
            DataType::Dictionary(
                keys.clone(),
                Box::new(restore_nested_order(dictionary, values)?),
            )
        }
        _ => data_type.clone(),
    })
}

/// One initialized Scheme export that fills owned Arrow C Data output descriptors.
#[derive(Clone, Copy, Debug)]
pub struct LinkedArrowProducer<'runtime> {
    runtime: &'runtime GerbilRuntime,
    export: ArrowProducerAbi,
}

type ArrowProducerAbi = unsafe extern "C" fn(*mut FFI_ArrowArray, *mut FFI_ArrowSchema) -> i32;

impl GerbilRuntime {
    /// Bind a native Scheme producer on this runtime owner.
    ///
    /// # Errors
    /// Rejects a call outside the runtime owner thread.
    ///
    /// # Safety
    /// The export must belong to this initialized AOT graph and live for the
    /// process. It must catch Scheme exceptions and never retain the output
    /// pointers or reenter Rust. On success both outputs must be fully
    /// initialized, agreeing Arrow C Data descriptors. On failure each output
    /// must remain empty or be independently safe to release. Release callbacks
    /// and their buffers must remain valid after runtime shutdown and support
    /// release on any thread. Native Scheme producers must use foreign-owned
    /// storage, or a separately synchronized allocator, for this contract.
    pub unsafe fn bind_arrow_producer(
        &self,
        export: ArrowProducerAbi,
    ) -> Result<LinkedArrowProducer<'_>, NativeError> {
        self.check_thread()?;
        Ok(LinkedArrowProducer {
            runtime: self,
            export,
        })
    }
}

impl LinkedArrowProducer<'_> {
    /// Invoke the producer and take ownership of the result.
    ///
    /// # Errors
    /// Returns a native error on a thread, producer status, or missing output.
    pub fn call(&self) -> Result<ArrowArrayExport, NativeError> {
        const OPERATION: &str = "linked Arrow producer";
        self.runtime.check_thread()?;
        let mut array = FFI_ArrowArray::empty();
        let mut schema = FFI_ArrowSchema::empty();
        // SAFETY: binding guarantees an initialized graph and the C Data
        // contract. Both output slots are empty and exclusive to this call.
        let code = unsafe { (self.export)(&raw mut array, &raw mut schema) };
        if code != gerbil_scheme_sys::GerbilStatus::Ok as i32 {
            return Err(NativeError::Status {
                operation: OPERATION,
                code,
            });
        }
        if array.is_released() {
            return Err(NativeError::Status {
                operation: OPERATION,
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            });
        }
        Ok(ArrowArrayExport { array, schema })
    }
}

#[cfg(test)]
#[path = "../../tests/unit/arrow.rs"]
mod tests;
