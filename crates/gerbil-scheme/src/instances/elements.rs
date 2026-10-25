//! Borrowed projections over one owner-local snapshot; no Scheme words escape.
use super::{InstanceError, NativeRoot, NativeSession};
use crate::{ObjectKind, SchemeScalar};

/// A strong snapshot of collection slots, scoped to the fixed VM owner.
/// Scalar reads borrow those slots. Complex children acquire an independent root
/// only when requested. Child identity survives parent mutation and native GC.
/// ```compile_fail
/// use gerbil_scheme::instances::InstanceDispatcher;
/// fn cannot_export_snapshot(dispatch: &InstanceDispatcher) {
///     let _ = dispatch.submit_native_on(0, |session| {
///         let value = session.vector(&[])?;
///         session.elements_snapshot(&value)
///     });
/// }
/// ```
#[derive(Debug)]
pub struct ElementSnapshot<'session, 'vm> {
    pub(super) session: &'session NativeSession<'vm>,
    pub(super) snapshot: NativeRoot<'vm>,
    pub(super) len: usize,
}
impl<'vm> ElementSnapshot<'_, 'vm> {
    /// Number of captured slots (alternating keys and values for tables).
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }
    /// Whether the captured collection has no slots.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    fn index(&self, index: usize) -> Result<i64, InstanceError> {
        if index >= self.len {
            return Err(InstanceError::InvalidConfig("snapshot index"));
        }
        i64::try_from(index).map_err(|_| InstanceError::InvalidConfig("snapshot index"))
    }
    /// Inspect a captured child without allocating another strong root.
    /// # Errors
    /// Rejects out-of-bounds indices, missing ABI or native kind failure.
    pub fn kind(&self, index: usize) -> Result<ObjectKind, InstanceError> {
        let index = self.index(index)?;
        // SAFETY: bounds checked; the snapshot remains strongly rooted on this owner.
        let tag = unsafe { (self.session.objects()?.kind)(self.snapshot.root, index) };
        crate::runtime::decode_kind(tag).map_err(|_| InstanceError::Native {
            operation: "snapshot kind",
            code: i64::from(tag),
        })
    }
    /// Copy an immediate scalar, or report that this child requires another
    /// representation. Arbitrary precision integers may require an owned root.
    /// # Errors
    /// Rejects out-of-bounds indices, native failures and malformed projections.
    pub fn scalar(&self, index: usize) -> Result<Option<SchemeScalar>, InstanceError> {
        let index = self.index(index)?;
        let mut projection = gerbil_scheme_sys::GerbilScalarProjection::default();
        // SAFETY: bounds checked on an owner-local strong snapshot; writable output.
        let status = unsafe {
            (self.session.image.native.scalar_read)(self.snapshot.root, index, &raw mut projection)
        };
        // The existing scalar ABI uses status 3 when no scalar projection is available.
        // All other rejections must remain errors, never a conversion fallback.
        match status {
            3 => Ok(None),
            0 => crate::runtime::decode_scalar_projection(&projection)
                .map(Some)
                .map_err(|_| InstanceError::Native {
                    operation: "snapshot scalar decode",
                    code: -1,
                }),
            code => Err(InstanceError::Native {
                operation: "snapshot scalar projection",
                code: i64::from(code),
            }),
        }
    }
    /// Promote one child to a root with a lifetime independent of this snapshot.
    /// # Errors
    /// Rejects out-of-bounds indices and failed native root creation.
    pub fn rooted(&self, index: usize) -> Result<NativeRoot<'vm>, InstanceError> {
        self.index(index)?;
        self.session.vector_element(&self.snapshot, index)
    }
}
