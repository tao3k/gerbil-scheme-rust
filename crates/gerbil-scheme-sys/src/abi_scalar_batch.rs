//! Whole-span scalar projection; values remain owned by the Scheme caller.
use crate::{GerbilScalarProjection, GerbilStatus, GerbilValueHandle};

unsafe extern "C" {
    /// Construct an immediate fixnum; zero is a valid Scheme word.
    /// # Safety
    /// Must run on the active owner with a writable output word.
    pub fn gerbil_scheme_rust_fixnum_from_word(value: isize, out: *mut usize) -> GerbilStatus;
    /// Project an entire span without allocation or publishing partial results.
    ///
    /// # Safety
    /// The runtime must be entered on its owner. Every handle must reference
    /// a live Scheme object. Nonempty spans must be valid, aligned, disjoint,
    /// and length elements long. No pointer is retained across this call.
    /// A zero Scheme word denotes fixnum zero, not a nullable input pointer.
    pub fn gerbil_scheme_rust_scalar_batch(
        values: *const GerbilValueHandle,
        length: usize,
        output: *mut GerbilScalarProjection,
    ) -> GerbilStatus;
}
