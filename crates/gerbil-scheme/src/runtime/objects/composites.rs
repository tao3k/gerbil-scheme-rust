//! Exact numeric components and shallow strong-table snapshots.
use super::{RootedObjectBatch, RootedSchemeObject, invalid, own};
use crate::runtime::{GerbilRuntime, NativeError};
use gerbil_scheme_sys as sys;

/// Numeric construction/projection semantics delegated to Scheme.
#[derive(Clone, Copy, Debug)]
pub enum NumberKind {
    Rational,
    Complex,
}
impl NumberKind {
    const fn code(self) -> i32 {
        match self {
            Self::Rational => 0,
            Self::Complex => 1,
        }
    }
}
/// Official Scheme key equality, not Rust hashing or string equality.
#[derive(Clone, Copy, Debug)]
pub enum TableEquality {
    Identity,
    Equivalent,
    Structural,
}
impl TableEquality {
    const fn code(self) -> i32 {
        match self {
            Self::Identity => 0,
            Self::Equivalent => 1,
            Self::Structural => 2,
        }
    }
}
impl GerbilRuntime {
    /// Construct a complete vector of Scheme numbers from rooted component pairs.
    /// Rationals require exact integers and nonzero denominators. Scheme owns
    /// canonicalization; a zero imaginary part can normalize to a real number.
    /// # Errors
    /// Rejects invalid components, wrong-thread entry or size overflow.
    pub fn number_batch(
        &self,
        kind: NumberKind,
        components: &[(&RootedSchemeObject<'_>, &RootedSchemeObject<'_>)],
    ) -> Result<RootedSchemeObject<'_>, NativeError> {
        self.check_thread()?;
        let roots = root_pairs(components)?;
        // SAFETY: a complete readable owner-local span; no Rust pointer retained.
        let root = unsafe {
            sys::gerbil_scheme_rust_composite_create(
                roots.as_ptr(),
                u64::try_from(components.len()).map_err(|_| invalid("number count"))?,
                kind.code(),
            )
        };
        own(root).map(|owner| RootedSchemeObject { owner })
    }
    /// Construct a strong Gambit table from a whole key/value span.
    /// Duplicate keys follow the selected official equality and last value wins.
    /// Weak keys/values are intentionally not a Rust ownership protocol.
    /// # Errors
    /// Rejects invalid roots, wrong-thread entry or size overflow.
    pub fn hash_table(
        &self,
        equality: TableEquality,
        entries: &[(&RootedSchemeObject<'_>, &RootedSchemeObject<'_>)],
    ) -> Result<RootedSchemeObject<'_>, NativeError> {
        self.check_thread()?;
        let roots = root_pairs(entries)?;
        // SAFETY: live strong input roots, complete pair span, synchronous copy.
        let root = unsafe {
            sys::gerbil_scheme_rust_table_create(
                roots.as_ptr(),
                u64::try_from(entries.len()).map_err(|_| invalid("table count"))?,
                equality.code(),
            )
        };
        own(root).map(|owner| RootedSchemeObject { owner })
    }
}
fn root_pairs(
    entries: &[(&RootedSchemeObject<'_>, &RootedSchemeObject<'_>)],
) -> Result<Vec<i64>, NativeError> {
    let size = entries
        .len()
        .checked_mul(2)
        .ok_or_else(|| invalid("pair count"))?;
    let mut roots = Vec::with_capacity(size);
    for (left, right) in entries {
        roots.push(left.owner.root_id().0);
        roots.push(right.owner.root_id().0);
    }
    Ok(roots)
}
impl<'r> RootedSchemeObject<'r> {
    /// Snapshot all numerator/denominator or real/imaginary components at once.
    /// Exact integers/rationals stay strongly rooted, never coerced to f64/text.
    /// # Errors
    /// Wrong family leaves the previous snapshot unchanged.
    pub fn copy_number_components(
        &self,
        kind: NumberKind,
        output: &mut RootedObjectBatch<'r>,
    ) -> Result<(), NativeError> {
        self.snapshot_composite(kind.code(), output)
    }
    /// Snapshot alternating key/value references; traversal order is unspecified.
    /// The snapshot is shallow and retains object identity, including nested values.
    /// It remains valid after releasing the table; it is not a deep serialization.
    /// # Errors
    /// A non-table leaves the previous snapshot unchanged.
    pub fn copy_table_entries(
        &self,
        output: &mut RootedObjectBatch<'r>,
    ) -> Result<(), NativeError> {
        self.snapshot_composite(2, output)
    }
    fn snapshot_composite(
        &self,
        kind: i32,
        output: &mut RootedObjectBatch<'r>,
    ) -> Result<(), NativeError> {
        let mut length = 0_u64;
        // SAFETY: live owner root; the complete fresh snapshot has one strong root.
        let root = unsafe {
            sys::gerbil_scheme_rust_composite_snapshot(
                self.owner.root_id().0,
                -1,
                kind,
                &raw mut length,
            )
        };
        let owner = own(root)?;
        let length = usize::try_from(length).map_err(|_| invalid("snapshot length"))?;
        if length % 2 != 0 {
            return Err(invalid("component pair count"));
        }
        output.replace_snapshot(owner, length);
        Ok(())
    }
}
