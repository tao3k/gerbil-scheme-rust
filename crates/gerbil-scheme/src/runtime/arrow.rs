// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Call-scoped Arrow C Data input for an already initialized Scheme owner.

use super::types::RootedSchemeOwner;
use super::{GerbilRuntime, NativeError, NativeResult, RootedSchemeObject};
use arrow_array::ffi::{FFI_ArrowArray, FFI_ArrowSchema};
use arrow_array::{Array, RecordBatch, StructArray};
use arrow_schema::ArrowError;

/// Owns an Arrow C Data export. The descriptors keep the original column
/// allocations alive. Construct once to reuse the metadata on later calls.
#[derive(Debug)]
pub struct ArrowBatchExport {
    array: FFI_ArrowArray,
    schema: FFI_ArrowSchema,
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
        Ok(Self { array, schema })
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
        let result = (|| {
            self.runtime.check_thread()?;
            // SAFETY: binding guarantees the synchronous borrow and unique
            // result root; batch keeps all Arrow allocations alive.
            let root = gerbil_scheme_sys::GerbilRootId(unsafe {
                (self.export)(&raw const batch.array, &raw const batch.schema)
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

#[cfg(test)]
#[path = "../../tests/unit/arrow.rs"]
mod tests;
