//! Strong-root collections and binary arbitrary-precision integer transport.
mod integer;
mod numeric;
pub use numeric::{NumericBuffer, NumericElement};
mod values;
pub use integer::{BigIntegerBuffer, BinaryInteger};
pub use values::{
    CollectionKind, RootedObjectBatch, RootedSchemeCollection, RootedSchemeObject, SchemeObjectRef,
};
use values::{invalid, own};
