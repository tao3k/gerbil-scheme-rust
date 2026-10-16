//! Strong-root collections and binary arbitrary-precision integer transport.
mod composites;
mod identifiers;
pub use composites::{NumberKind, TableEquality};
mod integer;
pub use identifiers::{IdentifierBuffer, IdentifierKind};
mod numeric;
pub use numeric::{NumericBuffer, NumericElement};
mod values;
pub use integer::{BigIntegerBuffer, BinaryInteger};
pub use values::{
    CollectionKind, ObjectKind, RootedObjectBatch, RootedSchemeCollection, RootedSchemeObject,
    SchemeObjectRef,
};
use values::{invalid, own};
