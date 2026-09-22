// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Safe Rust APIs for invoking Gerbil Scheme.
//!
//! The `native` feature exposes a one-shot in-process runtime and GC-rooted
//! Scheme values. The separate `Gerbil` toolchain API invokes a configured
//! `gxi` explicitly; it is never a fallback for native initialization or calls.
//! Both surfaces remain independent from downstream application semantics.

mod toolchain;
pub use toolchain::{
    GERBIL_GSC_ENV, GERBIL_GXC_ENV, GERBIL_GXI_ENV, Gerbil, GerbilError, GerbilToolchain,
    default_gambit_gsc_program_for_gxi, is_gambit_gsc_program, resolve_gerbil_executable,
};

#[cfg(feature = "native")]
pub use gerbil_scheme_sys::{
    GERBIL_SCHEME_RUST_ABI_ID, GERBIL_SCHEME_RUST_ABI_VERSION, GerbilStatus,
};

#[cfg(feature = "native")]
mod native;

#[cfg(feature = "native")]
pub use native::{
    ByteOrder, BytestringDelimiter, ExactIntegerTarget, IntegerDecoding, IntegerEncoding,
    IntegerWidth, RootedSchemeBytevector, RootedSchemeExactInteger, RootedSchemeString,
    RootedSchemeValue, RootedSchemeValueKind, SchemeBorrowedBytevector, SchemeBorrowedVector,
    SchemeBytevector, SchemeExactInteger, SchemeKeyword, SchemeList, SchemeNil, SchemePair,
    SchemePairParts, SchemeScalar, SchemeSymbol, SchemeVoid,
};

#[cfg(feature = "native")]
pub use native::{
    GerbilI64Callback, GerbilI64CallbackAbi, GerbilRuntime, GerbilRuntimeReceipt, GerbilUtf8,
    GerbilValue, GerbilValueProvenance, LinkedGerbilProgram, LinkedStringExport, NativeError,
    NativeResult,
};

pub mod native_environment;
