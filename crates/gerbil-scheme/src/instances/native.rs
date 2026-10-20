//! Native typed values stay on the Scheme owner; only owned Rust projections return.
use super::{InstanceError, image::Image};
use crate::{NumericElement, SchemeScalar};
use std::{cell::Cell, marker::PhantomData, rc::Rc};

/// Borrowed fixed-owner native call context. Existing native conversion functions
/// construct and project actual Scheme values; no JSON or secondary codec runs.
#[derive(Debug)]
pub struct NativeSession<'owner> {
    pub(super) image: &'owner Image,
    pub(super) maximum_input: usize,
    pub(super) maximum_output: usize,
    pub(super) transferred: Cell<usize>,
}
/// Owner-local strong root. It is neither `Send` nor `Sync` and cannot be a job result.
/// ```compile_fail
/// use gerbil_scheme::instances::InstanceDispatcher;
/// fn cannot_export_root(dispatch: &InstanceDispatcher) {
///     let _ = dispatch.submit_native_on(0, |session| session.numeric_vector(&[1_i64]));
/// }
/// ```
#[derive(Debug)]
pub struct NativeRoot<'owner> {
    pub(super) image: &'owner Image,
    pub(super) root: i64,
    _owner: PhantomData<Rc<()>>,
}
impl Drop for NativeRoot<'_> {
    fn drop(&mut self) {
        // SAFETY: owner affinity and the image borrow outlive this unique root.
        if unsafe { (self.image.release)(self.root) } != 1 {
            crate::instance_backend::poison_images();
        }
    }
}
fn checked(code: i64, operation: &'static str) -> Result<(), InstanceError> {
    if code == 0 {
        Ok(())
    } else {
        Err(InstanceError::Native { operation, code })
    }
}
impl<'owner> NativeSession<'owner> {
    pub(super) fn own(&self, root: i64) -> Result<NativeRoot<'owner>, InstanceError> {
        if root <= 0 {
            return Err(InstanceError::Native {
                operation: "native root",
                code: root,
            });
        }
        Ok(NativeRoot {
            image: self.image,
            root,
            _owner: PhantomData,
        })
    }
    pub(super) fn call_result(&self, root: i64) -> Result<NativeRoot<'owner>, InstanceError> {
        if root == 0
            && let Some(take) = self.image.native.call_error
        {
            // SAFETY: same owner, immediately after the rooted call. The
            // optional export consumes the diagnostic and transfers a root.
            let token = unsafe { take() };
            if token > 0 {
                let message = self.own(token)?;
                let bytes = self.image.copy_reply(message.root, 16384)?;
                return Err(InstanceError::Scheme(
                    String::from_utf8_lossy(&bytes).into_owned(),
                ));
            }
        }
        self.own(root)
    }
    pub(super) fn input(&self, bytes: usize) -> Result<(), InstanceError> {
        let total = self
            .transferred
            .get()
            .checked_add(bytes)
            .ok_or(InstanceError::InvalidConfig("native input overflow"))?;
        if total > self.maximum_input {
            return Err(InstanceError::InvalidConfig("native input bound"));
        }
        self.transferred.set(total);
        Ok(())
    }
    pub(super) fn local(&self, root: &NativeRoot<'_>) -> Result<(), InstanceError> {
        if std::ptr::eq(self.image, root.image) {
            Ok(())
        } else {
            Err(InstanceError::InvalidConfig("foreign runtime root"))
        }
    }
    /// Construct an existing audited native numeric-vector family.
    /// # Errors
    /// Rejects transfer bounds or failed native construction.
    pub fn numeric_vector<T: NumericElement>(
        &self,
        values: &[T],
    ) -> Result<NativeRoot<'owner>, InstanceError> {
        self.input(std::mem::size_of_val(values))?;
        // SAFETY: sealed NumericElement is the same layout contract used by GerbilRuntime.
        self.own(unsafe {
            (self.image.native.numeric_create)(T::KIND, values.as_ptr().cast(), values.len() as u64)
        })
    }
    /// Construct bool, fixnum, char or flonum using the existing native scalar ABI.
    /// # Errors
    /// Rejects failed construction or transfer bounds.
    pub fn scalar(&self, value: SchemeScalar) -> Result<NativeRoot<'owner>, InstanceError> {
        self.input(16)?;
        let (tag, integer, real) = match value {
            SchemeScalar::Boolean(value) => (1, i64::from(value), 0.0),
            SchemeScalar::Fixnum(value) => (2, value as i64, 0.0),
            SchemeScalar::Char(value) => (3, i64::from(u32::from(value)), 0.0),
            SchemeScalar::Flonum(value) => (4, 0, value),
        };
        // SAFETY: matching audited scalar payload; native code validates ranges.
        self.own(unsafe { (self.image.native.scalar_create)(tag, integer, real) })
    }
    /// Construct a native Scheme string through the existing UTF-8 conversion ABI.
    /// # Errors
    /// Rejects transfer bounds or native construction.
    pub fn string(&self, value: &str) -> Result<NativeRoot<'owner>, InstanceError> {
        self.input(value.len())?;
        // SAFETY: the input span stays valid through the synchronous native copy.
        let bytes = self.own(unsafe { (self.image.input)(value.as_ptr(), value.len() as u64) })?;
        // SAFETY: the rooted bytevector remains live through Scheme decoding.
        self.own(unsafe { (self.image.native.utf8_string)(bytes.root) })
    }
    /// Construct a native Scheme vector with roots from this owner.
    /// # Errors
    /// Rejects foreign-owner roots, transfer bounds or failed construction.
    pub fn vector(&self, values: &[&NativeRoot<'_>]) -> Result<NativeRoot<'owner>, InstanceError> {
        self.input(std::mem::size_of_val(values))?;
        for value in values {
            self.local(value)?;
        }
        let roots: Vec<_> = values.iter().map(|value| value.root).collect();
        // SAFETY: complete span of live owner-local roots; Scheme copies synchronously.
        self.own(unsafe { (self.image.native.collection)(roots.as_ptr(), roots.len() as u64, 0) })
    }
    /// Invoke the build-owned Scheme object export with one borrowed root.
    /// # Errors
    /// Rejects foreign roots or a caught Scheme exception (zero result root).
    pub fn call(&self, input: &NativeRoot<'_>) -> Result<NativeRoot<'owner>, InstanceError> {
        self.local(input)?;
        // SAFETY: the descriptor admits this export; input remains rooted and
        // exceptions are contained in Scheme. Output transfers a fresh root.
        self.call_result(unsafe { (self.image.call)(input.root) })
    }
    /// Force the selected SDK's collector on this instance's owner.
    /// # Errors
    /// Returns the native collector status without substituting a Rust collector.
    pub fn collect(&self) -> Result<(), InstanceError> {
        // SAFETY: active fixed Scheme owner; all external values are strong roots.
        checked(
            i64::from(unsafe { (self.image.native.force_gc)() }),
            "native GC",
        )
    }
    /// Read GC count, heap capacity bytes, SDK post-GC live metric and root count.
    /// # Errors
    /// Rejects an unavailable native snapshot.
    pub fn usage(&self) -> Result<[f64; 4], InstanceError> {
        let mut values = [0.0; 4];
        // SAFETY: writable four-double output on a live fixed owner.
        checked(
            i64::from(unsafe { (self.image.native.usage)(values.as_mut_ptr()) }),
            "native usage",
        )?;
        Ok(values)
    }
    /// Optional cumulative owner counters: GC user/system/real seconds, owner
    /// thread CPU seconds, strong roots and GC count. Older ABI-v3 images may
    /// omit this export. A failed snapshot returns no counters, never zeroes.
    #[must_use]
    pub fn diagnostics(&self) -> Option<[f64; 6]> {
        let read = self.image.native.diagnostics?;
        let mut values = [0.0; 6];
        // SAFETY: optional audited export writes exactly six doubles on this owner.
        (unsafe { read(values.as_mut_ptr()) } == 0).then_some(values)
    }
    /// Project an existing native vector without per-element foreign calls.
    /// # Errors
    /// Rejects type mismatch, output bounds or native copy failure.
    pub fn copy_numeric<T: NumericElement>(
        &self,
        value: &NativeRoot<'_>,
    ) -> Result<Vec<T>, InstanceError> {
        self.copy_numeric_bounded(value, self.maximum_output)
    }
    pub(super) fn copy_numeric_bounded<T: NumericElement>(
        &self,
        value: &NativeRoot<'_>,
        maximum: usize,
    ) -> Result<Vec<T>, InstanceError> {
        self.local(value)?;
        // SAFETY: the root remains live on this owner, same existing numeric ABI.
        let size = unsafe { (self.image.native.numeric_size)(value.root, -1, T::KIND) };
        let size = usize::try_from(size).map_err(|_| InstanceError::Native {
            operation: "native numeric type",
            code: size,
        })?;
        if size
            .checked_mul(std::mem::size_of::<T>())
            .is_none_or(|bytes| bytes > maximum)
        {
            return Err(InstanceError::InvalidConfig("native output bound"));
        }
        let mut output = Vec::new();
        output
            .try_reserve_exact(size)
            .map_err(|error| InstanceError::Loader(error.to_string()))?;
        output.resize(size, T::default());
        // SAFETY: sealed native storage, complete disjoint output, matching family.
        checked(
            unsafe {
                (self.image.native.numeric_copy)(
                    value.root,
                    -1,
                    T::KIND,
                    output.as_mut_ptr().cast(),
                    0,
                    size as u64,
                )
            },
            "native numeric copy",
        )?;
        Ok(output)
    }
    /// Project a native Scheme string using the existing UTF-8 bridge.
    /// # Errors
    /// Rejects type/encoding errors, byte bounds or failed conversion.
    pub fn copy_string(&self, value: &NativeRoot<'_>) -> Result<String, InstanceError> {
        self.local(value)?;
        // SAFETY: live strong string root; encoding transfers another strong root.
        let bytes = self.own(unsafe { (self.image.native.string_utf8)(value.root) })?;
        String::from_utf8(self.image.copy_reply(bytes.root, self.maximum_output)?)
            .map_err(|error| InstanceError::Loader(error.to_string()))
    }
    /// Borrow one vector slot through an independently owned strong root.
    /// # Errors
    /// Rejects a foreign root or invalid native vector index.
    pub fn vector_element(
        &self,
        value: &NativeRoot<'_>,
        index: usize,
    ) -> Result<NativeRoot<'owner>, InstanceError> {
        self.local(value)?;
        // SAFETY: a live owner-local root; native code checks shape and bounds.
        self.own(unsafe { (self.image.native.element)(value.root, 0, index as u64) })
    }
    /// Project one native scalar, including signed zero and Unicode characters.
    /// # Errors
    /// Rejects a non-scalar or invalid projection.
    pub fn copy_scalar(&self, value: &NativeRoot<'_>) -> Result<SchemeScalar, InstanceError> {
        self.local(value)?;
        let mut projection = gerbil_scheme_sys::GerbilScalarProjection::default();
        // SAFETY: live owner-local root, writable disjoint scalar result.
        checked(
            i64::from(unsafe {
                (self.image.native.scalar_read)(value.root, -1, &raw mut projection)
            }),
            "native scalar projection",
        )?;
        crate::runtime::decode_scalar_projection(&projection).map_err(|_| InstanceError::Native {
            operation: "native scalar decode",
            code: -1,
        })
    }
}
