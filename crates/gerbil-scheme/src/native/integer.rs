// SPDX-License-Identifier: LGPL-2.1-or-later OR Apache-2.0

//! Exact-integer width, byte-order, and Scheme conversion operations.

use super::{
    GerbilValue, GerbilValueProvenance, IntegerEncoding, IntegerWidth, NativeError, NativeResult,
    SchemeBytevector, SchemeExactInteger, SchemeNil, SchemePairParts, SchemeVoid,
};
use gerbil_scheme_sys::{
    gerbil_scheme_rust_scheme_object_as_boolean, gerbil_scheme_rust_scheme_object_as_char,
    gerbil_scheme_rust_scheme_object_as_fixnum, gerbil_scheme_rust_scheme_object_as_flonum,
    gerbil_scheme_rust_scheme_object_is_boolean, gerbil_scheme_rust_scheme_object_is_char,
    gerbil_scheme_rust_scheme_object_is_exact_integer, gerbil_scheme_rust_scheme_object_is_fixnum,
    gerbil_scheme_rust_scheme_object_is_flonum, gerbil_scheme_rust_scheme_object_is_list,
    gerbil_scheme_rust_scheme_object_is_null, gerbil_scheme_rust_scheme_object_is_pair,
};
use std::marker::PhantomData;
use std::num::NonZeroUsize;

pub(super) fn resolved_unsigned_encoding_width(
    value: u64,
    encoding: IntegerEncoding,
) -> Result<usize, NativeError> {
    let width = encoding.width().map_or_else(
        || {
            let bits = u64::BITS - value.leading_zeros();
            u8::try_from(bits.div_ceil(8).max(1)).expect("u64 requires at most eight bytes")
        },
        IntegerWidth::get,
    );
    if !encoding.allows_truncation() && !unsigned_integer_fits(value, width) {
        return Err(NativeError::UnsignedIntegerWidth { value, width });
    }
    Ok(usize::from(width))
}

pub(super) fn resolved_signed_encoding_width(
    value: i64,
    encoding: IntegerEncoding,
) -> Result<usize, NativeError> {
    let width = encoding.width().map_or_else(
        || {
            (1..=IntegerWidth::MAX)
                .find(|width| signed_integer_fits(value, *width))
                .expect("every i64 fits in eight bytes")
        },
        IntegerWidth::get,
    );
    if !encoding.allows_truncation() && !signed_integer_fits(value, width) {
        return Err(NativeError::SignedIntegerWidth { value, width });
    }
    Ok(usize::from(width))
}

const fn unsigned_integer_fits(value: u64, width: u8) -> bool {
    width == IntegerWidth::MAX || value < (1_u64 << ((width as u32) * 8))
}

const fn signed_integer_fits(value: i64, width: u8) -> bool {
    if width == IntegerWidth::MAX {
        return true;
    }
    let magnitude_bits = (width as u32) * 8 - 1;
    let minimum = -(1_i64 << magnitude_bits);
    let maximum = (1_i64 << magnitude_bits) - 1;
    value >= minimum && value <= maximum
}

impl SchemeNil<'_> {
    /// Wrap a non-zero runtime-owned nil handle.
    ///
    /// This constructor does not inspect the handle; callers must prove the
    /// handle came from a runtime-backed `null?` projection before using it.
    #[must_use]
    pub fn from_raw(raw: gerbil_scheme_sys::GerbilValueHandle) -> Option<Self> {
        NonZeroUsize::new(raw).map(|raw| Self {
            raw,
            _runtime: PhantomData,
        })
    }

    /// Return the borrowed raw handle without dereferencing it.
    #[must_use]
    pub const fn as_raw(self) -> gerbil_scheme_sys::GerbilValueHandle {
        self.raw.get()
    }
}

impl<'runtime> GerbilValue<'runtime> {
    /// Return whether this value is known to be a pair.
    ///
    /// Until the sys ABI is backed by runtime classification, this returns a
    /// fail-closed [`NativeError::Status`] instead of guessing.
    #[must_use]
    pub fn is_pair(self) -> NativeResult<bool> {
        match self.provenance {
            GerbilValueProvenance::SchemeObjectExport => checked_native_predicate(
                "gerbil_scheme_rust_scheme_object_is_pair",
                self.raw.get(),
                gerbil_scheme_rust_scheme_object_is_pair,
            ),
            GerbilValueProvenance::UntrustedRaw | GerbilValueProvenance::RuntimeSentinel => {
                checked_native_predicate(
                    "gerbil_scheme_rust_value_is_pair",
                    self.raw.get(),
                    gerbil_scheme_sys::gerbil_scheme_rust_value_is_pair,
                )
            }
        }
    }

    /// Return whether this value is known to be a list.
    ///
    /// Until the sys ABI is backed by runtime classification, this returns a
    /// fail-closed [`NativeError::Status`] instead of guessing.
    #[must_use]
    pub fn is_list(self) -> NativeResult<bool> {
        match self.provenance {
            GerbilValueProvenance::SchemeObjectExport => checked_native_predicate(
                "gerbil_scheme_rust_scheme_object_is_list",
                self.raw.get(),
                gerbil_scheme_rust_scheme_object_is_list,
            ),
            GerbilValueProvenance::UntrustedRaw | GerbilValueProvenance::RuntimeSentinel => {
                checked_native_predicate(
                    "gerbil_scheme_rust_value_is_list",
                    self.raw.get(),
                    gerbil_scheme_sys::gerbil_scheme_rust_value_is_list,
                )
            }
        }
    }

    /// Return whether this value is known to be Scheme null.
    ///
    /// Until the sys ABI is backed by runtime classification, this returns a
    /// fail-closed [`NativeError::Status`] instead of guessing.
    #[must_use]
    pub fn is_null(self) -> NativeResult<bool> {
        match self.provenance {
            GerbilValueProvenance::SchemeObjectExport => checked_native_predicate(
                "gerbil_scheme_rust_scheme_object_is_null",
                self.raw.get(),
                gerbil_scheme_rust_scheme_object_is_null,
            ),
            GerbilValueProvenance::UntrustedRaw | GerbilValueProvenance::RuntimeSentinel => {
                checked_native_predicate(
                    "gerbil_scheme_rust_value_is_null",
                    self.raw.get(),
                    gerbil_scheme_sys::gerbil_scheme_rust_value_is_null,
                )
            }
        }
    }

    /// Project this value's car if it is backed by a pair.
    ///
    /// This delegates to the sys ABI and only succeeds for Scheme-object
    /// exports.
    /// Projects this value as Scheme nil / the empty list.
    ///
    /// This succeeds only for runtime-produced Scheme-object exports that
    /// satisfy `null?`. It returns a borrowed marker around the same opaque
    /// handle and does not claim ownership or GC rooting.
    #[must_use]
    pub fn as_nil(self) -> NativeResult<SchemeNil<'runtime>> {
        if self.provenance != GerbilValueProvenance::SchemeObjectExport {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_nil",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            });
        }

        match self.is_null().into_result() {
            Ok(true) => SchemeNil::from_raw(self.raw.get()).map_or_else(
                || {
                    NativeResult::err(NativeError::Status {
                        operation: "gerbil_scheme_rust_scheme_object_as_nil",
                        code: gerbil_scheme_sys::GerbilStatus::NullPointer as i32,
                    })
                },
                NativeResult::ok,
            ),
            Ok(false) => NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_nil",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            }),
            Err(error) => NativeResult::err(error),
        }
    }

    /// Checks whether this value is Scheme void.
    ///
    /// This only succeeds for values exported by the initialized Gerbil runtime.
    #[must_use]
    pub fn is_void(self) -> NativeResult<bool> {
        match self.provenance {
            GerbilValueProvenance::SchemeObjectExport => checked_native_predicate(
                "gerbil_scheme_rust_scheme_object_is_void",
                self.raw.get(),
                gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_is_void,
            ),
            GerbilValueProvenance::UntrustedRaw | GerbilValueProvenance::RuntimeSentinel => {
                NativeResult::err(NativeError::Status {
                    operation: "gerbil_scheme_rust_scheme_object_is_void",
                    code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
                })
            }
        }
    }

    /// Projects this value as Scheme void.
    ///
    /// This succeeds only for runtime-produced Scheme-object exports that
    /// satisfy `void?`. It returns a borrowed marker around the same opaque
    /// handle and does not claim ownership or GC rooting.
    #[must_use]
    pub fn as_void(self) -> NativeResult<SchemeVoid<'runtime>> {
        if self.provenance != GerbilValueProvenance::SchemeObjectExport {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_void",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            });
        }

        match self.is_void().into_result() {
            Ok(true) => SchemeVoid::from_raw(self.raw.get()).map_or_else(
                || {
                    NativeResult::err(NativeError::Status {
                        operation: "gerbil_scheme_rust_scheme_object_as_void",
                        code: gerbil_scheme_sys::GerbilStatus::NullPointer as i32,
                    })
                },
                NativeResult::ok,
            ),
            Ok(false) => NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_void",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            }),
            Err(error) => NativeResult::err(error),
        }
    }

    /// Checks whether this value is a Scheme bytevector.
    ///
    /// This only succeeds for values exported by the initialized Gerbil runtime.
    #[must_use]
    pub fn is_bytevector(self) -> NativeResult<bool> {
        match self.provenance {
            GerbilValueProvenance::SchemeObjectExport => checked_native_predicate(
                "gerbil_scheme_rust_scheme_object_is_bytevector",
                self.raw.get(),
                gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_is_bytevector,
            ),
            GerbilValueProvenance::UntrustedRaw | GerbilValueProvenance::RuntimeSentinel => {
                NativeResult::err(NativeError::Status {
                    operation: "gerbil_scheme_rust_scheme_object_is_bytevector",
                    code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
                })
            }
        }
    }

    /// Projects this value as a Scheme bytevector.
    ///
    /// This succeeds only for runtime-produced Scheme-object exports that
    /// satisfy `u8vector?`. It returns a borrowed marker around the same opaque
    /// handle and does not claim ownership or GC rooting.
    #[must_use]
    pub fn as_bytevector(self) -> NativeResult<SchemeBytevector<'runtime>> {
        if self.provenance != GerbilValueProvenance::SchemeObjectExport {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_bytevector",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            });
        }

        match self.is_bytevector().into_result() {
            Ok(true) => SchemeBytevector::from_raw(self.raw.get()).map_or_else(
                || {
                    NativeResult::err(NativeError::Status {
                        operation: "gerbil_scheme_rust_scheme_object_as_bytevector",
                        code: gerbil_scheme_sys::GerbilStatus::NullPointer as i32,
                    })
                },
                NativeResult::ok,
            ),
            Ok(false) => NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_bytevector",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            }),
            Err(error) => NativeResult::err(error),
        }
    }

    /// Checks whether this value is a Scheme boolean.
    ///
    /// This only succeeds for values exported by the initialized Gerbil runtime.
    #[must_use]
    pub fn is_boolean(self) -> NativeResult<bool> {
        match self.provenance {
            GerbilValueProvenance::SchemeObjectExport => checked_native_predicate(
                "gerbil_scheme_rust_scheme_object_is_boolean",
                self.raw.get(),
                gerbil_scheme_rust_scheme_object_is_boolean,
            ),
            GerbilValueProvenance::UntrustedRaw | GerbilValueProvenance::RuntimeSentinel => {
                NativeResult::err(NativeError::Status {
                    operation: "gerbil_scheme_rust_scheme_object_is_boolean",
                    code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
                })
            }
        }
    }

    /// Projects this value as a Scheme boolean.
    ///
    /// This only succeeds for Scheme-object exports that satisfy `boolean?`.
    #[must_use]
    pub fn as_boolean(self) -> NativeResult<bool> {
        if self.provenance != GerbilValueProvenance::SchemeObjectExport {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_boolean",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            });
        }

        let mut out = gerbil_scheme_sys::GerbilBoolean::from_bool(false);
        // SAFETY: `out` is a valid output slot for one GerbilBoolean.
        let status =
            unsafe { gerbil_scheme_rust_scheme_object_as_boolean(self.raw.get(), &raw mut out) };
        if status != gerbil_scheme_sys::GerbilStatus::Ok {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_boolean",
                code: status as i32,
            });
        }
        NativeResult::ok(out.as_bool())
    }

    /// Returns whether this value is a Scheme fixnum.
    ///
    /// This only succeeds for Scheme-object exports; untrusted raw handles and
    /// runtime sentinels fail closed with `InvalidValue`.
    #[must_use]
    pub fn is_fixnum(self) -> NativeResult<bool> {
        match self.provenance {
            GerbilValueProvenance::SchemeObjectExport => checked_native_predicate(
                "gerbil_scheme_rust_scheme_object_is_fixnum",
                self.raw.get(),
                gerbil_scheme_rust_scheme_object_is_fixnum,
            ),
            GerbilValueProvenance::UntrustedRaw | GerbilValueProvenance::RuntimeSentinel => {
                NativeResult::err(NativeError::Status {
                    operation: "gerbil_scheme_rust_scheme_object_is_fixnum",
                    code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
                })
            }
        }
    }

    /// Projects this value as a Scheme fixnum.
    ///
    /// This intentionally covers only Gerbil fixnums. Bignums and other exact
    /// integer objects must use a later, explicitly versioned projection path.
    #[must_use]
    pub fn as_fixnum(self) -> NativeResult<isize> {
        if self.provenance != GerbilValueProvenance::SchemeObjectExport {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_fixnum",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            });
        }

        let mut out = gerbil_scheme_sys::GerbilFixnum::default();
        // SAFETY: `out` is a valid output slot for one GerbilFixnum.
        let status =
            unsafe { gerbil_scheme_rust_scheme_object_as_fixnum(self.raw.get(), &raw mut out) };
        if status != gerbil_scheme_sys::GerbilStatus::Ok {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_fixnum",
                code: status as i32,
            });
        }
        NativeResult::ok(out.0)
    }

    /// Projects this value as a Scheme fixnum widened to `i64`.
    #[must_use]
    pub fn as_fixnum_i64(self) -> NativeResult<i64> {
        match self.as_fixnum().as_result() {
            Ok(value) => NativeResult::ok(*value as i64),
            Err(error) => NativeResult::err(*error),
        }
    }

    /// Return whether this runtime-backed Scheme object is an exact integer.
    ///
    /// Both fixnums and bignums satisfy this predicate. Untrusted raw handles and
    /// runtime sentinels remain fail-closed.
    #[must_use]
    pub fn is_exact_integer(self) -> NativeResult<bool> {
        match self.provenance {
            GerbilValueProvenance::SchemeObjectExport => checked_native_predicate(
                "gerbil_scheme_rust_scheme_object_is_exact_integer",
                self.raw.get(),
                gerbil_scheme_rust_scheme_object_is_exact_integer,
            ),
            GerbilValueProvenance::UntrustedRaw | GerbilValueProvenance::RuntimeSentinel => {
                NativeResult::err(NativeError::Status {
                    operation: "gerbil_scheme_rust_scheme_object_is_exact_integer",
                    code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
                })
            }
        }
    }

    /// Project this runtime-backed Scheme object as an exact integer handle.
    ///
    /// The returned marker is borrowed and unrooted. Use its checked machine
    /// projections without retaining it beyond the runtime borrow.
    #[must_use]
    pub fn as_exact_integer(self) -> NativeResult<SchemeExactInteger<'runtime>> {
        if self.provenance != GerbilValueProvenance::SchemeObjectExport {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_exact_integer",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            });
        }

        match self.is_exact_integer().into_result() {
            Ok(true) => SchemeExactInteger::from_raw(self.raw.get()).map_or_else(
                || {
                    NativeResult::err(NativeError::Status {
                        operation: "gerbil_scheme_rust_scheme_object_as_exact_integer",
                        code: gerbil_scheme_sys::GerbilStatus::NullPointer as i32,
                    })
                },
                NativeResult::ok,
            ),
            Ok(false) => NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_exact_integer",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            }),
            Err(error) => NativeResult::err(error),
        }
    }

    /// Returns whether this value is a Scheme character.
    ///
    /// This only succeeds for Scheme-object exports; untrusted raw handles and
    /// runtime sentinels fail closed with `InvalidValue`.
    #[must_use]
    pub fn is_char(self) -> NativeResult<bool> {
        match self.provenance {
            GerbilValueProvenance::SchemeObjectExport => checked_native_predicate(
                "gerbil_scheme_rust_scheme_object_is_char",
                self.raw.get(),
                gerbil_scheme_rust_scheme_object_is_char,
            ),
            GerbilValueProvenance::UntrustedRaw | GerbilValueProvenance::RuntimeSentinel => {
                NativeResult::err(NativeError::Status {
                    operation: "gerbil_scheme_rust_scheme_object_is_char",
                    code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
                })
            }
        }
    }

    /// Projects this value as a Scheme character.
    ///
    /// The sys layer returns a Unicode scalar value carrier and this method
    /// performs Rust scalar validation before exposing `char`.
    #[must_use]
    pub fn as_char(self) -> NativeResult<char> {
        if self.provenance != GerbilValueProvenance::SchemeObjectExport {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_char",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            });
        }

        let mut out = gerbil_scheme_sys::GerbilChar::default();
        // SAFETY: `out` is a valid output slot for one GerbilChar.
        let status =
            unsafe { gerbil_scheme_rust_scheme_object_as_char(self.raw.get(), &raw mut out) };
        if status != gerbil_scheme_sys::GerbilStatus::Ok {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_char",
                code: status as i32,
            });
        }

        match char::try_from(out) {
            Ok(value) => NativeResult::ok(value),
            Err(()) => NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_char",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            }),
        }
    }

    /// Returns whether this value is a Scheme flonum.
    ///
    /// This only succeeds for Scheme-object exports; untrusted raw handles and
    /// runtime sentinels fail closed with `InvalidValue`.
    #[must_use]
    pub fn is_flonum(self) -> NativeResult<bool> {
        match self.provenance {
            GerbilValueProvenance::SchemeObjectExport => checked_native_predicate(
                "gerbil_scheme_rust_scheme_object_is_flonum",
                self.raw.get(),
                gerbil_scheme_rust_scheme_object_is_flonum,
            ),
            GerbilValueProvenance::UntrustedRaw | GerbilValueProvenance::RuntimeSentinel => {
                NativeResult::err(NativeError::Status {
                    operation: "gerbil_scheme_rust_scheme_object_is_flonum",
                    code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
                })
            }
        }
    }

    /// Projects this value as a Scheme flonum.
    ///
    /// The Rust side preserves IEEE-754 `f64` semantics, including NaN,
    /// infinities, and signed zero.
    #[must_use]
    pub fn as_flonum(self) -> NativeResult<f64> {
        if self.provenance != GerbilValueProvenance::SchemeObjectExport {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_flonum",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            });
        }

        let mut out = gerbil_scheme_sys::GerbilFlonum::default();
        // SAFETY: `out` is a valid output slot for one GerbilFlonum.
        let status =
            unsafe { gerbil_scheme_rust_scheme_object_as_flonum(self.raw.get(), &raw mut out) };
        if status != gerbil_scheme_sys::GerbilStatus::Ok {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_flonum",
                code: status as i32,
            });
        }

        NativeResult::ok(out.0)
    }

    /// Project this value's car if it is backed by a pair.
    ///
    /// This delegates to the sys ABI and only succeeds for Scheme-object
    /// exports.
    #[must_use]
    pub fn pair_car(self) -> NativeResult<Self> {
        checked_scheme_object_projection(
            "gerbil_scheme_rust_pair_car",
            self,
            gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_pair_car,
        )
    }

    /// Project this value's cdr if it is backed by a pair.
    ///
    /// This delegates to the sys ABI and only succeeds for Scheme-object
    /// exports.
    #[must_use]
    pub fn pair_cdr(self) -> NativeResult<Self> {
        checked_scheme_object_projection(
            "gerbil_scheme_rust_pair_cdr",
            self,
            gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_pair_cdr,
        )
    }

    /// Project this value's pair parts if it is backed by a pair.
    ///
    /// This delegates to the sys ABI and only succeeds for Scheme-object
    /// exports.
    #[must_use]
    pub fn pair_parts(self) -> NativeResult<SchemePairParts<'runtime>> {
        if self.provenance != GerbilValueProvenance::SchemeObjectExport {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_pair_parts",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            });
        }

        let mut pair = gerbil_scheme_sys::GerbilPair { car: 0, cdr: 0 };
        // SAFETY: `pair` is a valid output slot for one GerbilPair.
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_pair_parts(
                self.raw.get(),
                &raw mut pair,
            )
        };
        if status != gerbil_scheme_sys::GerbilStatus::Ok {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_pair_parts",
                code: status as i32,
            });
        }
        let Some(car) = value_from_native_handle_with_provenance(
            pair.car,
            GerbilValueProvenance::SchemeObjectExport,
        ) else {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_pair_parts.car",
                code: gerbil_scheme_sys::GerbilStatus::NullPointer as i32,
            });
        };
        let Some(cdr) = value_from_native_handle_with_provenance(
            pair.cdr,
            GerbilValueProvenance::SchemeObjectExport,
        ) else {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_pair_parts.cdr",
                code: gerbil_scheme_sys::GerbilStatus::NullPointer as i32,
            });
        };
        NativeResult::ok(SchemePairParts { car, cdr })
    }
}

type NativePredicate = unsafe extern "C" fn(
    gerbil_scheme_sys::GerbilValueHandle,
    *mut gerbil_scheme_sys::GerbilBoolean,
) -> gerbil_scheme_sys::GerbilStatus;

type NativeValueProjection = unsafe extern "C" fn(
    gerbil_scheme_sys::GerbilValueHandle,
    *mut gerbil_scheme_sys::GerbilValueHandle,
) -> gerbil_scheme_sys::GerbilStatus;

fn checked_native_predicate(
    operation: &'static str,
    value: gerbil_scheme_sys::GerbilValueHandle,
    predicate: NativePredicate,
) -> NativeResult<bool> {
    let mut out = gerbil_scheme_sys::GerbilBoolean::FALSE;
    // SAFETY: `out` is a valid output slot for one GerbilBoolean.
    let status = unsafe { predicate(value, &raw mut out) };
    if status == gerbil_scheme_sys::GerbilStatus::Ok {
        NativeResult::ok(out.as_bool())
    } else {
        NativeResult::err(NativeError::Status {
            operation,
            code: status as i32,
        })
    }
}

fn checked_scheme_object_projection<'runtime>(
    operation: &'static str,
    value: GerbilValue<'runtime>,
    projection: NativeValueProjection,
) -> NativeResult<GerbilValue<'runtime>> {
    if value.provenance != GerbilValueProvenance::SchemeObjectExport {
        return NativeResult::err(NativeError::Status {
            operation,
            code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
        });
    }

    let mut out = 0;
    // SAFETY: `out` is a valid output slot for one Gerbil value handle.
    let status = unsafe { projection(value.raw.get(), &raw mut out) };
    if status != gerbil_scheme_sys::GerbilStatus::Ok {
        return NativeResult::err(NativeError::Status {
            operation,
            code: status as i32,
        });
    }

    value_from_native_handle_with_provenance(out, GerbilValueProvenance::SchemeObjectExport)
        .map_or_else(
            || {
                NativeResult::err(NativeError::Status {
                    operation,
                    code: gerbil_scheme_sys::GerbilStatus::NullPointer as i32,
                })
            },
            NativeResult::ok,
        )
}

pub(super) fn value_from_native_handle_with_provenance<'runtime>(
    raw: gerbil_scheme_sys::GerbilValueHandle,
    provenance: GerbilValueProvenance,
) -> Option<GerbilValue<'runtime>> {
    NonZeroUsize::new(raw).map(|raw| GerbilValue {
        raw,
        provenance,
        _runtime: PhantomData,
    })
}

impl GerbilValue<'_> {
    /// Wrap a raw runtime-borrowed value handle, rejecting zero handles.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::Status`] with `NullPointer` when `raw` is zero.
    pub fn from_raw(raw: gerbil_scheme_sys::GerbilValueHandle) -> Result<Self, NativeError> {
        let raw = NonZeroUsize::new(raw).ok_or(NativeError::Status {
            operation: "GerbilValue::from_raw",
            code: gerbil_scheme_sys::GerbilStatus::NullPointer as i32,
        })?;

        Ok(Self {
            raw,
            provenance: GerbilValueProvenance::UntrustedRaw,
            _runtime: PhantomData,
        })
    }

    /// Return the raw borrowed value handle.
    #[must_use]
    pub fn as_raw(self) -> gerbil_scheme_sys::GerbilValueHandle {
        self.raw.get()
    }

    /// Return the provenance attached to this borrowed value handle.
    #[must_use]
    pub const fn provenance(self) -> GerbilValueProvenance {
        self.provenance
    }
}
