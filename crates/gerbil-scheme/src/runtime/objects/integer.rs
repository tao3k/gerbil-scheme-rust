//! Canonical sign + little-endian magnitude; never text or machine truncation.
use super::{RootedSchemeObject, SchemeObjectRef, invalid, own};
use crate::runtime::GerbilRuntime;
use crate::runtime::NativeError;
use gerbil_scheme_sys as sys;

/// Arbitrary precision integer transport; zero has empty magnitude and no sign.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BinaryInteger {
    negative: bool,
    magnitude: Vec<u8>,
}
impl BinaryInteger {
    /// Construct canonical little-endian magnitude (no redundant high zero).
    /// # Errors
    /// Rejects negative zero and noncanonical magnitude encodings.
    pub fn new(negative: bool, magnitude: Vec<u8>) -> Result<Self, NativeError> {
        if magnitude.last() == Some(&0) || (negative && magnitude.is_empty()) {
            return Err(invalid("canonical integer"));
        }
        Ok(Self {
            negative,
            magnitude,
        })
    }
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.negative
    }
    #[must_use]
    pub fn magnitude(&self) -> &[u8] {
        &self.magnitude
    }
}
/// Reused staging isolates failed conversion from the published integer.
#[derive(Debug, Default)]
pub struct BigIntegerBuffer {
    value: BinaryInteger,
    staged: Vec<u8>,
}
impl BigIntegerBuffer {
    #[must_use]
    pub fn value(&self) -> &BinaryInteger {
        &self.value
    }
    /// Rust capacity only, excluding Scheme bignum/GC storage.
    #[must_use]
    pub fn reserved_bytes(&self) -> usize {
        self.value.magnitude.capacity() + self.staged.capacity()
    }
}
impl SchemeObjectRef<'_> {
    /// Copy an entire exact integer into reusable owned Rust memory.
    /// # Errors
    /// Rejects non-integers without changing the published value.
    pub fn copy_big_integer(self, output: &mut BigIntegerBuffer) -> Result<(), NativeError> {
        let size = unsafe { sys::gerbil_scheme_rust_bigint_size_indexed(self.root.0, self.index) };
        let size = usize::try_from(size).map_err(|_| invalid("integer size"))?;
        output.staged.resize(size, 0);
        let sign = unsafe {
            sys::gerbil_scheme_rust_bigint_copy_indexed(
                self.root.0,
                self.index,
                output.staged.as_mut_ptr(),
                size as u64,
            )
        };
        if !matches!(sign, 0 | 1) {
            return Err(invalid("integer bulk copy"));
        }
        std::mem::swap(&mut output.value.magnitude, &mut output.staged);
        output.staged.clear();
        output.value.negative = sign == 1;
        Ok(())
    }
}
impl GerbilRuntime {
    /// Construct a strongly rooted Scheme exact integer from binary magnitude.
    /// # Errors
    /// Rejects a wrong owner or failed native creation.
    pub fn big_integer(
        &self,
        value: &BinaryInteger,
    ) -> Result<RootedSchemeObject<'_>, NativeError> {
        self.check_thread()?;
        let root = unsafe {
            sys::gerbil_scheme_rust_bigint_create_raw(
                value.magnitude.as_ptr(),
                value.magnitude.len() as u64,
                i32::from(value.negative),
            )
        };
        own(root).map(|owner| RootedSchemeObject { owner })
    }
}
