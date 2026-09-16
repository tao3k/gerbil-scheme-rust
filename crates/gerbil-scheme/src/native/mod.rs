// SPDX-License-Identifier: LGPL-2.1-or-later OR Apache-2.0

//! Safe native Gerbil/Gambit bindings organized by ownership boundary.
//!
//! `types` defines ABI-neutral value shapes, `rooted` owns GC-rooted Scheme
//! values, `integer` implements exact-integer conversion, `runtime` owns
//! Gerbil runtime entry points, and `value` owns safe value interpretation.

mod integer;
mod program;
mod rooted;
mod runtime;
mod types;
mod value;
pub use program::{LinkedGerbilProgram, LinkedStringExport};
pub use runtime::{GerbilI64Callback, GerbilI64CallbackAbi};
pub use types::{
    ByteOrder, BytestringDelimiter, ExactIntegerTarget, GerbilRuntime, GerbilRuntimeReceipt,
    GerbilUtf8, GerbilValue, GerbilValueProvenance, IntegerDecoding, IntegerEncoding, IntegerWidth,
    RootedSchemeBytevector, RootedSchemeExactInteger, RootedSchemeString, RootedSchemeValue,
    RootedSchemeValueKind, SchemeBytevector, SchemeExactInteger, SchemeNil, SchemePairParts,
    SchemeVoid,
};
pub use value::{
    NativeError, NativeResult, SchemeBorrowedBytevector, SchemeBorrowedVector, SchemeKeyword,
    SchemeList, SchemePair, SchemeScalar, SchemeSymbol,
};
