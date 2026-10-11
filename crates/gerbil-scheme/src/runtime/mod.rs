// SPDX-License-Identifier: LGPL-2.1-or-later OR Apache-2.0

//! Safe native Gerbil/Gambit bindings organized by ownership boundary.
//!
//! `types` defines ABI-neutral value shapes, `rooted` owns GC-rooted Scheme
//! values, `integer` implements exact-integer conversion, `lifecycle` owns
//! Gerbil runtime entry points, and `value` owns safe value interpretation.

mod capabilities;
mod integer;
mod lifecycle;
mod program;
mod rooted;
mod types;
mod utf8;
mod value;
mod value_buffers;
pub use capabilities::VmCapabilities;
pub use lifecycle::{GerbilI64Callback, GerbilI64CallbackAbi};
pub use program::{LinkedBytesExport, LinkedGerbilProgram, LinkedStringExport};
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
