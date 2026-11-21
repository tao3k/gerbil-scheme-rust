//! Whole-value scalar projection with explicit, reusable Rust scratch.
use super::{GerbilRuntime, GerbilValue, GerbilValueProvenance, NativeError, SchemeScalar};
use gerbil_scheme_sys::{GerbilScalarProjection, GerbilStatus};

impl GerbilRuntime {
    /// Construct a Scheme immediate fixnum without allocating a Scheme object.
    /// # Errors
    /// Returns an error for a wrong owner, inactive runtime or out-of-range value.
    pub fn fixnum_value(&self, number: isize) -> Result<GerbilValue<'_>, NativeError> {
        self.check_thread()?;
        let mut raw = 0;
        // SAFETY: active owner, immediate value, writable output word.
        let status =
            unsafe { gerbil_scheme_sys::gerbil_scheme_rust_fixnum_from_word(number, &raw mut raw) };
        if status != GerbilStatus::Ok {
            return Err(NativeError::Status {
                operation: "gerbil_scheme_rust_fixnum_from_word",
                code: status as i32,
            });
        }
        Ok(GerbilValue {
            raw: super::types::SchemeObjectWord(raw),
            provenance: GerbilValueProvenance::SchemeObjectExport,
            _runtime: std::marker::PhantomData,
        })
    }
}

/// Reusable scalar projection storage, independent of Scheme root ownership.
#[derive(Debug, Default)]
pub struct ScalarBatch {
    handles: Vec<usize>,
    projected: Vec<GerbilScalarProjection>,
}
impl ScalarBatch {
    /// Rust scratch reserve, excluding caller output and allocator metadata.
    #[must_use]
    pub fn reserved_bytes(&self) -> usize {
        self.handles.capacity() * std::mem::size_of::<usize>()
            + self.projected.capacity() * std::mem::size_of::<GerbilScalarProjection>()
    }
    /// Project bool, fixnum, char and flonum values in one foreign call.
    ///
    /// No Scheme handles or pointers are retained after return. Unsupported
    /// types and untrusted handles fail without modifying the caller's output.
    ///
    /// # Errors
    /// Returns an error for foreign-owner or untrusted handles, unsupported
    /// Scheme types, an inactive runtime, or invalid ABI payloads.
    ///
    /// # Panics
    /// Panics if already-validated immutable payloads fail a second identical
    /// decode, which would indicate an internal implementation defect.
    pub fn project(
        &mut self,
        runtime: &GerbilRuntime,
        values: &[GerbilValue<'_>],
        output: &mut Vec<SchemeScalar>,
    ) -> Result<(), NativeError> {
        let error = || NativeError::Status {
            operation: "gerbil_scheme_rust_scalar_batch",
            code: GerbilStatus::InvalidValue as i32,
        };
        if runtime.owner != std::thread::current().id()
            || values
                .iter()
                .any(|v| v.provenance != GerbilValueProvenance::SchemeObjectExport)
        {
            return Err(error());
        }
        self.handles.clear();
        self.handles.extend(values.iter().map(|v| v.raw.get()));
        self.projected
            .resize(values.len(), GerbilScalarProjection::default());
        // SAFETY: trusted runtime exports on their owner, nonoverlapping Rust
        // storage, no allocation in the foreign span, no body pointer escapes.
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_scalar_batch(
                self.handles.as_ptr(),
                self.handles.len(),
                self.projected.as_mut_ptr(),
            )
        };
        self.handles.clear();
        if status != GerbilStatus::Ok {
            return Err(NativeError::Status {
                operation: "gerbil_scheme_rust_scalar_batch",
                code: status as i32,
            });
        }
        // Validate before publishing, including ABI drift and invalid chars.
        self.projected
            .iter()
            .try_for_each(|v| decode_scalar_projection(v).map(|_| ()))?;
        output.clear();
        output.extend(
            self.projected
                .iter()
                .map(|v| decode_scalar_projection(v).expect("validated scalar")),
        );
        Ok(())
    }
}

pub(crate) fn decode_scalar_projection(
    value: &GerbilScalarProjection,
) -> Result<SchemeScalar, NativeError> {
    let error = || NativeError::Status {
        operation: "gerbil_scheme_rust_scalar_batch",
        code: GerbilStatus::InvalidValue as i32,
    };
    Ok(match value.tag {
        1 => SchemeScalar::Boolean(value.bits != 0),
        2 => SchemeScalar::Fixnum(
            isize::try_from(i64::from_ne_bytes(value.bits.to_ne_bytes())).map_err(|_| error())?,
        ),
        3 => SchemeScalar::Char(
            char::from_u32(u32::try_from(value.bits).map_err(|_| error())?).ok_or_else(error)?,
        ),
        4 => SchemeScalar::Flonum(f64::from_bits(value.bits)),
        _ => return Err(error()),
    })
}
