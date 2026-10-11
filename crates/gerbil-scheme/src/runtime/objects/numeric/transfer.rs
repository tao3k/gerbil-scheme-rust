//! Native homogeneous vector transport, without per-element foreign calls.
use super::sealed;
use crate::runtime::objects::{RootedSchemeObject, SchemeObjectRef, invalid, own};
use crate::runtime::{GerbilRuntime, NativeError};
use gerbil_scheme_sys as sys;

/// Supported native numeric elements. Sealed: arbitrary Rust layouts cannot
/// be reinterpreted as Scheme vector storage. Every bit pattern is valid.
///
/// ```compile_fail
/// use gerbil_scheme::{GerbilRuntime, NumericBuffer};
/// fn unsupported(runtime: &GerbilRuntime) {
///     let _ = runtime.numeric_vector(&[true, false]);
///     let _ = NumericBuffer::<bool>::default();
/// }
/// ```
pub trait NumericElement: sealed::Sealed + Copy + Default {
    #[doc(hidden)]
    const KIND: i32;
}
macro_rules! elements {
    ($($ty:ty => $kind:expr),* $(,)?) => {$(
        impl sealed::Sealed for $ty {}
        impl NumericElement for $ty { const KIND: i32 = $kind; }
    )*};
}
elements!(u16 => 0, u32 => 1, u64 => 2, f32 => 3, f64 => 4);

/// Reusable owned projection; staging preserves published output on failure.
#[derive(Debug, Default)]
pub struct NumericBuffer<T: NumericElement> {
    values: Vec<T>,
    staged: Vec<T>,
}
impl<T: NumericElement> NumericBuffer<T> {
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        &self.values
    }
    /// Retained Rust capacity, including transactional staging, not Scheme heap.
    #[must_use]
    pub fn reserved_bytes(&self) -> usize {
        (self.values.capacity() + self.staged.capacity()) * std::mem::size_of::<T>()
    }
}
impl GerbilRuntime {
    /// Create a strongly rooted homogeneous vector with one native span transfer.
    /// # Errors
    /// Rejects wrong-thread entry, size overflow or failed Scheme construction.
    pub fn numeric_vector<T: NumericElement>(
        &self,
        values: &[T],
    ) -> Result<RootedSchemeObject<'_>, NativeError> {
        self.check_thread()?;
        let len = u64::try_from(values.len()).map_err(|_| invalid("numeric length"))?;
        // SAFETY: sealed types have the official native scalar layout; this
        // owner-local call copies synchronously and retains no Rust pointer.
        let root =
            unsafe { sys::gerbil_scheme_rust_numeric_create(T::KIND, values.as_ptr().cast(), len) };
        own(root).map(|owner| RootedSchemeObject { owner })
    }
}
impl SchemeObjectRef<'_> {
    /// Copy the complete vector, preserving NaN payloads and signed zero.
    /// # Errors
    /// Wrong family or failed transfer leaves published output unchanged.
    pub fn copy_numeric<T: NumericElement>(
        self,
        output: &mut NumericBuffer<T>,
    ) -> Result<(), NativeError> {
        // SAFETY: the reference retains its owner-local root, including snapshots.
        let len = unsafe { sys::gerbil_scheme_rust_numeric_size(self.root.0, self.index, T::KIND) };
        let len = usize::try_from(len).map_err(|_| invalid("numeric type"))?;
        output.staged.resize(len, T::default());
        // SAFETY: initialized writable contiguous native elements; the sealed
        // family matches the checked Scheme type. No body pointer escapes.
        let status = unsafe {
            sys::gerbil_scheme_rust_numeric_copy(
                self.root.0,
                self.index,
                T::KIND,
                output.staged.as_mut_ptr().cast(),
                0,
                len as u64,
            )
        };
        if status != 0 {
            return Err(invalid("numeric copy"));
        }
        std::mem::swap(&mut output.values, &mut output.staged);
        output.staged.clear();
        Ok(())
    }
}
