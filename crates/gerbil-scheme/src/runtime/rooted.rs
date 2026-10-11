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

use super::value_buffers::copy_reused_bytevector;

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
        copy_owned_bytevector(
            self.len(),
            "gerbil_scheme_rust_scheme_object_bytevector_copy",
            |output, length| {
                // SAFETY: this view retains the runtime lifetime and the helper
                // provides disjoint writable capacity for this synchronous call.
                unsafe {
                    gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_bytevector_copy(
                        self.raw.get(),
                        output,
                        length,
                    )
                }
            },
        )
    }

    /// Replace an owned Rust buffer, reusing capacity across varying lengths.
    ///
    /// Length-query failure preserves the buffer. Once copying begins, failure
    /// leaves it empty; partially initialized bytes are never exposed.
    #[must_use]
    pub fn copy_to_vec(&self, output: &mut Vec<u8>) -> NativeResult<()> {
        copy_reused_bytevector(
            self.len(),
            output,
            "gerbil_scheme_rust_scheme_object_bytevector_copy",
            |destination, length| unsafe {
                // SAFETY: this owner-affine view remains live, and the helper
                // supplies exclusively borrowed capacity for the entire call.
                gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_bytevector_copy(
                    self.raw.get(),
                    destination,
                    length,
                )
            },
        )
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

impl<'runtime> RootedSchemeString<'runtime> {
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

    /// Encode an independent, owner-affine UTF-8 snapshot using Scheme's codec.
    ///
    /// The snapshot outlives this string root, but not its runtime. Callers may
    /// reuse its bulk-copy APIs instead of re-encoding unchanged text. This is
    /// explicit snapshot ownership, not a cache of a mutable Scheme string.
    #[must_use]
    pub fn to_utf8_bytes(&self) -> NativeResult<RootedSchemeBytevector<'runtime>> {
        let mut root = gerbil_scheme_sys::GerbilRootId(0);
        // SAFETY: this owner-affine root stays live throughout encoding. The
        // Scheme encoder transfers an independent rooted bytevector.
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_string_to_utf8(
                self.owner.root_id(),
                &raw mut root,
            )
        };
        RootedSchemeOwner::new(status, root, "gerbil_scheme_rust_root_string_to_utf8")
            .map(|owner| RootedSchemeBytevector { owner })
            .into()
    }

    /// Encode this rooted Scheme string directly into owned Rust UTF-8 storage.
    ///
    /// Capacity reserves at most four bytes per current Scheme character.
    /// This avoids temporary Scheme bytes and a second copy; capacity is not
    /// shrunk by a second allocation. No cached snapshot or heap view escapes.
    #[must_use]
    pub fn to_string(&self) -> NativeResult<String> {
        let result = (|| {
            let operation = "gerbil_scheme_rust_root_string_encode_into";
            let failure = || NativeError::Status {
                operation,
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            };
            let capacity = self
                .len()
                .into_result()?
                .checked_mul(4)
                .ok_or_else(failure)?;
            let mut bytes = Vec::<u8>::new();
            bytes.try_reserve_exact(capacity).map_err(|_| failure())?;
            let mut written = 0;
            // SAFETY: this string root stays owner-local and live. Exclusive
            // Rust capacity remains stable across Scheme polls during the call.
            let status = unsafe {
                gerbil_scheme_sys::gerbil_scheme_rust_root_string_encode_into(
                    self.owner.root_id(),
                    bytes.as_mut_ptr(),
                    capacity,
                    &raw mut written,
                )
            };
            if status != gerbil_scheme_sys::GerbilStatus::Ok {
                return Err(NativeError::Status {
                    operation,
                    code: status as i32,
                });
            }
            if written > capacity {
                return Err(failure());
            }
            // SAFETY: only complete success initializes and admits this prefix.
            unsafe { bytes.set_len(written) };
            super::utf8::validated_string(bytes)
        })();
        result.into()
    }
}

impl RootedSchemeBytevector<'_> {
    /// Return the owned root token without transferring ownership.
    #[must_use]
    pub const fn root_id(&self) -> gerbil_scheme_sys::GerbilRootId {
        self.owner.root_id()
    }

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
        copy_owned_bytevector(
            self.len(),
            "gerbil_scheme_rust_root_bytevector_copy",
            |output, length| {
                // SAFETY: the owner retains this root and the helper provides
                // disjoint writable capacity for this synchronous call.
                unsafe {
                    gerbil_scheme_sys::gerbil_scheme_rust_root_bytevector_copy(
                        self.owner.root_id(),
                        output,
                        length,
                    )
                }
            },
        )
    }

    /// Replace an owned Rust buffer, reusing capacity across varying lengths.
    ///
    /// Length-query failure preserves the buffer. Once copying begins, failure
    /// leaves it empty; partially initialized bytes are never exposed.
    #[must_use]
    pub fn copy_to_vec(&self, output: &mut Vec<u8>) -> NativeResult<()> {
        copy_reused_bytevector(
            self.len(),
            output,
            "gerbil_scheme_rust_root_bytevector_copy",
            |destination, length| unsafe {
                // SAFETY: the root retains owner affinity, and the helper
                // supplies exclusively borrowed capacity for the entire call.
                gerbil_scheme_sys::gerbil_scheme_rust_root_bytevector_copy(
                    self.owner.root_id(),
                    destination,
                    length,
                )
            },
        )
    }

    /// Copy into an exactly sized reusable Rust buffer without allocating.
    ///
    /// A length mismatch or invalid root leaves the buffer unchanged. This
    /// performs one copy; it does not borrow a pointer into the Scheme heap.
    #[must_use]
    pub fn copy_into(&self, output: &mut [u8]) -> NativeResult<()> {
        // SAFETY: the root retains owner affinity and output is exclusively
        // borrowed for this synchronous call; it cannot overlap Scheme storage.
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_bytevector_copy(
                self.owner.root_id(),
                output.as_mut_ptr(),
                output.len(),
            )
        };
        if status == gerbil_scheme_sys::GerbilStatus::Ok {
            NativeResult::ok(())
        } else {
            NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_root_bytevector_copy",
                code: status as i32,
            })
        }
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

// Both borrowed and rooted bytevectors use the same owned-buffer initialization
// rule; only their checked Scheme source identity differs.
fn copy_owned_bytevector(
    length: NativeResult<usize>,
    operation: &'static str,
    copy: impl FnOnce(*mut u8, usize) -> gerbil_scheme_sys::GerbilStatus,
) -> NativeResult<Vec<u8>> {
    let mut bytes = Vec::new();
    match copy_reused_bytevector(length, &mut bytes, operation, copy).into_result() {
        Ok(()) => NativeResult::ok(bytes),
        Err(error) => NativeResult::err(error),
    }
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
#[path = "../../tests/unit/owner/rooted_scheme_owner.rs"]
mod rooted_scheme_owner_tests;
