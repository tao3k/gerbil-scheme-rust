// SPDX-License-Identifier: LGPL-2.1-or-later OR Apache-2.0

//! GC-rooted Scheme value ownership and bytevector access.

use super::types::RootedSchemeOwner;
use super::{
    BytestringDelimiter, ExactIntegerTarget, IntegerDecoding, NativeError, NativeResult,
    RootedSchemeBytevector, RootedSchemeExactInteger, RootedSchemeString, SchemeBytevector,
    SchemeExactInteger,
};
use gerbil_scheme_sys::{
    gerbil_scheme_rust_scheme_object_exact_integer_to_i64,
    gerbil_scheme_rust_scheme_object_exact_integer_to_u64,
};
use std::marker::PhantomData;
use std::num::NonZeroUsize;

impl<'runtime> SchemeBytevector<'runtime> {
    pub(super) fn from_raw(raw: usize) -> Option<Self> {
        NonZeroUsize::new(raw).map(|raw| Self {
            raw,
            _runtime: PhantomData,
        })
    }

    #[must_use]
    pub fn as_raw(&self) -> usize {
        self.raw.get()
    }

    /// Return this bytevector's byte length.
    #[must_use]
    pub fn len(&self) -> NativeResult<usize> {
        let mut out = 0;
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_bytevector_length(
                self.raw.get(),
                &raw mut out,
            )
        };
        if status == gerbil_scheme_sys::GerbilStatus::Ok {
            NativeResult::ok(out)
        } else {
            NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_bytevector_length",
                code: status as i32,
            })
        }
    }

    /// Return whether this bytevector has no bytes.
    #[must_use]
    pub fn is_empty(&self) -> NativeResult<bool> {
        match self.len().into_result() {
            Ok(len) => NativeResult::ok(len == 0),
            Err(error) => NativeResult::err(error),
        }
    }

    /// Return the byte at `index`.
    #[must_use]
    pub fn u8_at(&self, index: usize) -> NativeResult<u8> {
        let mut out = 0;
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_bytevector_u8_ref(
                self.raw.get(),
                index,
                &raw mut out,
            )
        };
        if status == gerbil_scheme_sys::GerbilStatus::Ok {
            NativeResult::ok(out)
        } else {
            NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_bytevector_u8_ref",
                code: status as i32,
            })
        }
    }

    /// Copy this runtime-backed bytevector into owned Rust memory.
    #[must_use]
    pub fn to_vec(&self) -> NativeResult<Vec<u8>> {
        let len = match self.len().into_result() {
            Ok(len) => len,
            Err(error) => return NativeResult::err(error),
        };
        let mut bytes = Vec::with_capacity(len);
        for index in 0..len {
            match self.u8_at(index).into_result() {
                Ok(byte) => bytes.push(byte),
                Err(error) => return NativeResult::err(error),
            }
        }
        NativeResult::ok(bytes)
    }

    /// Decode this Scheme bytevector as an unsigned integer.
    #[must_use]
    pub fn to_uint(&self, decoding: IntegerDecoding) -> NativeResult<u64> {
        let size = match checked_integer_decoding_size(self.len(), decoding) {
            Ok(size) => size,
            Err(error) => return NativeResult::err(error),
        };
        let mut out = 0;
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_bytevector_to_uint(
                self.raw.get(),
                decoding.byte_order().abi_code(),
                size,
                &raw mut out,
            )
        };
        checked_integer_projection(status, out, "gerbil_scheme_rust_bytevector_to_uint")
    }

    /// Decode this Scheme bytevector as a signed two's-complement integer.
    #[must_use]
    pub fn to_sint(&self, decoding: IntegerDecoding) -> NativeResult<i64> {
        let size = match checked_integer_decoding_size(self.len(), decoding) {
            Ok(size) => size,
            Err(error) => return NativeResult::err(error),
        };
        let mut out = 0;
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_bytevector_to_sint(
                self.raw.get(),
                decoding.byte_order().abi_code(),
                size,
                &raw mut out,
            )
        };
        checked_integer_projection(status, out, "gerbil_scheme_rust_bytevector_to_sint")
    }

    /// Convert this bytevector through Gerbil's AOT bytestring implementation.
    ///
    /// The returned Scheme string is held by a native root and releases that
    /// root on drop. Gerbil emits uppercase hexadecimal digits and applies the
    /// requested delimiter between adjacent bytes.
    #[must_use]
    pub fn to_bytestring(
        &self,
        delimiter: BytestringDelimiter,
    ) -> NativeResult<RootedSchemeString<'runtime>> {
        let mut root = gerbil_scheme_sys::GerbilRootId(0);
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_bytevector_to_bytestring_root(
                self.raw.get(),
                delimiter.abi_code(),
                &raw mut root,
            )
        };
        match RootedSchemeOwner::new(
            status,
            root,
            "gerbil_scheme_rust_bytevector_to_bytestring_root",
        ) {
            Ok(owner) => NativeResult::ok(RootedSchemeString { owner }),
            Err(error) => NativeResult::err(error),
        }
    }
}

impl SchemeExactInteger<'_> {
    pub(super) fn from_raw(raw: usize) -> Option<Self> {
        NonZeroUsize::new(raw).map(|raw| Self {
            raw,
            _runtime: PhantomData,
        })
    }

    /// Return the borrowed Scheme-object handle.
    #[must_use]
    pub const fn as_raw(self) -> usize {
        self.raw.get()
    }

    /// Project this exact integer to `i64`, rejecting values outside the range.
    #[must_use]
    pub fn to_i64(self) -> NativeResult<i64> {
        let mut out = 0;
        let status = unsafe {
            gerbil_scheme_rust_scheme_object_exact_integer_to_i64(self.raw.get(), &raw mut out)
        };
        checked_exact_integer_projection(
            status,
            out,
            "gerbil_scheme_rust_scheme_object_exact_integer_to_i64",
            ExactIntegerTarget::I64,
        )
    }

    /// Project this exact integer to `u64`, rejecting negative or oversized values.
    #[must_use]
    pub fn to_u64(self) -> NativeResult<u64> {
        self.to_unsigned_target(ExactIntegerTarget::U64)
    }

    /// Project this exact integer to `usize` for the current Rust target.
    #[must_use]
    pub fn to_usize(self) -> NativeResult<usize> {
        checked_exact_integer_usize(self.to_unsigned_target(ExactIntegerTarget::Usize))
    }

    fn to_unsigned_target(self, target: ExactIntegerTarget) -> NativeResult<u64> {
        let mut out = 0;
        let status = unsafe {
            gerbil_scheme_rust_scheme_object_exact_integer_to_u64(self.raw.get(), &raw mut out)
        };
        checked_exact_integer_projection(
            status,
            out,
            "gerbil_scheme_rust_scheme_object_exact_integer_to_u64",
            target,
        )
    }
}

impl RootedSchemeExactInteger<'_> {
    /// Return the owned native root token without transferring ownership.
    #[must_use]
    pub const fn root_id(&self) -> gerbil_scheme_sys::GerbilRootId {
        self.owner.root_id()
    }

    /// Project this rooted exact integer to `i64` with range checking.
    #[must_use]
    pub fn to_i64(&self) -> NativeResult<i64> {
        let mut out = 0;
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_exact_integer_to_i64(
                self.owner.root_id(),
                &raw mut out,
            )
        };
        checked_exact_integer_projection(
            status,
            out,
            "gerbil_scheme_rust_root_exact_integer_to_i64",
            ExactIntegerTarget::I64,
        )
    }

    /// Project this rooted exact integer to `u64` with range checking.
    #[must_use]
    pub fn to_u64(&self) -> NativeResult<u64> {
        self.to_unsigned_target(ExactIntegerTarget::U64)
    }

    /// Project this rooted exact integer to `usize` for the current Rust target.
    #[must_use]
    pub fn to_usize(&self) -> NativeResult<usize> {
        checked_exact_integer_usize(self.to_unsigned_target(ExactIntegerTarget::Usize))
    }

    fn to_unsigned_target(&self, target: ExactIntegerTarget) -> NativeResult<u64> {
        let mut out = 0;
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_exact_integer_to_u64(
                self.owner.root_id(),
                &raw mut out,
            )
        };
        checked_exact_integer_projection(
            status,
            out,
            "gerbil_scheme_rust_root_exact_integer_to_u64",
            target,
        )
    }
}

impl RootedSchemeString<'_> {
    /// Return the number of Scheme characters in this rooted string.
    #[must_use]
    pub fn len(&self) -> NativeResult<usize> {
        let mut out = 0;
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_string_length(
                self.owner.root_id(),
                &raw mut out,
            )
        };
        if status == gerbil_scheme_sys::GerbilStatus::Ok {
            NativeResult::ok(out)
        } else {
            NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_root_string_length",
                code: status as i32,
            })
        }
    }

    /// Return whether this rooted string is empty.
    #[must_use]
    pub fn is_empty(&self) -> NativeResult<bool> {
        match self.len().into_result() {
            Ok(length) => NativeResult::ok(length == 0),
            Err(error) => NativeResult::err(error),
        }
    }

    /// Return the Scheme character at `index`.
    #[must_use]
    pub fn char_at(&self, index: usize) -> NativeResult<char> {
        let mut out = gerbil_scheme_sys::GerbilChar::default();
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_string_char_ref(
                self.owner.root_id(),
                index,
                &raw mut out,
            )
        };
        if status != gerbil_scheme_sys::GerbilStatus::Ok {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_root_string_char_ref",
                code: status as i32,
            });
        }
        match char::try_from(out) {
            Ok(character) => NativeResult::ok(character),
            Err(()) => NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_root_string_char_ref",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            }),
        }
    }

    /// Copy this rooted Scheme string into owned Rust UTF-8 storage.
    #[must_use]
    pub fn to_string(&self) -> NativeResult<String> {
        let length = match self.len().into_result() {
            Ok(length) => length,
            Err(error) => return NativeResult::err(error),
        };
        let mut text = String::with_capacity(length);
        for index in 0..length {
            match self.char_at(index).into_result() {
                Ok(character) => text.push(character),
                Err(error) => return NativeResult::err(error),
            }
        }
        NativeResult::ok(text)
    }
}

impl RootedSchemeBytevector<'_> {
    /// Return this rooted bytevector's byte length.
    #[must_use]
    pub fn len(&self) -> NativeResult<usize> {
        let mut out = 0;
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_bytevector_length(
                self.owner.root_id(),
                &raw mut out,
            )
        };
        if status == gerbil_scheme_sys::GerbilStatus::Ok {
            NativeResult::ok(out)
        } else {
            NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_root_bytevector_length",
                code: status as i32,
            })
        }
    }

    /// Return whether this rooted bytevector is empty.
    #[must_use]
    pub fn is_empty(&self) -> NativeResult<bool> {
        match self.len().into_result() {
            Ok(length) => NativeResult::ok(length == 0),
            Err(error) => NativeResult::err(error),
        }
    }

    /// Return the byte at `index`.
    #[must_use]
    pub fn u8_at(&self, index: usize) -> NativeResult<u8> {
        let mut out = 0;
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_bytevector_u8_ref(
                self.owner.root_id(),
                index,
                &raw mut out,
            )
        };
        if status == gerbil_scheme_sys::GerbilStatus::Ok {
            NativeResult::ok(out)
        } else {
            NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_root_bytevector_u8_ref",
                code: status as i32,
            })
        }
    }

    /// Copy this rooted Scheme bytevector into owned Rust memory.
    #[must_use]
    pub fn to_vec(&self) -> NativeResult<Vec<u8>> {
        let length = match self.len().into_result() {
            Ok(length) => length,
            Err(error) => return NativeResult::err(error),
        };
        let mut bytes = Vec::with_capacity(length);
        for index in 0..length {
            match self.u8_at(index).into_result() {
                Ok(byte) => bytes.push(byte),
                Err(error) => return NativeResult::err(error),
            }
        }
        NativeResult::ok(bytes)
    }

    /// Decode this rooted Scheme bytevector as an unsigned integer.
    #[must_use]
    pub fn to_uint(&self, decoding: IntegerDecoding) -> NativeResult<u64> {
        let size = match checked_integer_decoding_size(self.len(), decoding) {
            Ok(size) => size,
            Err(error) => return NativeResult::err(error),
        };
        let mut out = 0;
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_bytevector_to_uint(
                self.owner.root_id(),
                decoding.byte_order().abi_code(),
                size,
                &raw mut out,
            )
        };
        checked_integer_projection(status, out, "gerbil_scheme_rust_root_bytevector_to_uint")
    }

    /// Decode this rooted Scheme bytevector as a signed two's-complement integer.
    #[must_use]
    pub fn to_sint(&self, decoding: IntegerDecoding) -> NativeResult<i64> {
        let size = match checked_integer_decoding_size(self.len(), decoding) {
            Ok(size) => size,
            Err(error) => return NativeResult::err(error),
        };
        let mut out = 0;
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_bytevector_to_sint(
                self.owner.root_id(),
                decoding.byte_order().abi_code(),
                size,
                &raw mut out,
            )
        };
        checked_integer_projection(status, out, "gerbil_scheme_rust_root_bytevector_to_sint")
    }
}

pub(super) fn rooted_exact_integer<'runtime>(
    status: gerbil_scheme_sys::GerbilStatus,
    root: gerbil_scheme_sys::GerbilRootId,
    operation: &'static str,
) -> Result<RootedSchemeExactInteger<'runtime>, NativeError> {
    RootedSchemeOwner::new(status, root, operation).map(|owner| RootedSchemeExactInteger { owner })
}

fn checked_exact_integer_projection<T>(
    status: gerbil_scheme_sys::GerbilStatus,
    value: T,
    operation: &'static str,
    target: ExactIntegerTarget,
) -> NativeResult<T> {
    match status {
        gerbil_scheme_sys::GerbilStatus::Ok => NativeResult::ok(value),
        gerbil_scheme_sys::GerbilStatus::InvalidValue => {
            NativeResult::err(NativeError::ExactIntegerOutOfRange { target })
        }
        status => NativeResult::err(NativeError::Status {
            operation,
            code: status as i32,
        }),
    }
}

fn checked_exact_integer_usize(value: NativeResult<u64>) -> NativeResult<usize> {
    match value.into_result() {
        Ok(value) => usize::try_from(value).map_or_else(
            |_| {
                NativeResult::err(NativeError::ExactIntegerOutOfRange {
                    target: ExactIntegerTarget::Usize,
                })
            },
            NativeResult::ok,
        ),
        Err(error) => NativeResult::err(error),
    }
}

fn checked_integer_decoding_size(
    length: NativeResult<usize>,
    decoding: IntegerDecoding,
) -> Result<usize, NativeError> {
    let length = length.into_result()?;
    let size = decoding
        .width()
        .map_or(length, |width| usize::from(width.get()));
    if size > length || size > usize::from(gerbil_scheme_sys::GERBIL_SCHEME_RUST_MAX_INTEGER_BYTES)
    {
        return Err(NativeError::Status {
            operation: "integer bytevector decoding width",
            code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
        });
    }
    Ok(size)
}

fn checked_integer_projection<T>(
    status: gerbil_scheme_sys::GerbilStatus,
    value: T,
    operation: &'static str,
) -> NativeResult<T> {
    if status == gerbil_scheme_sys::GerbilStatus::Ok {
        NativeResult::ok(value)
    } else {
        NativeResult::err(NativeError::Status {
            operation,
            code: status as i32,
        })
    }
}

pub(super) fn rooted_integer_bytevector<'runtime>(
    status: gerbil_scheme_sys::GerbilStatus,
    root: gerbil_scheme_sys::GerbilRootId,
    operation: &'static str,
) -> Result<RootedSchemeBytevector<'runtime>, NativeError> {
    RootedSchemeOwner::new(status, root, operation).map(|owner| RootedSchemeBytevector { owner })
}
#[cfg(test)]
#[path = "../../tests/unit/native/rooted_scheme_owner.rs"]
mod rooted_scheme_owner_tests;
