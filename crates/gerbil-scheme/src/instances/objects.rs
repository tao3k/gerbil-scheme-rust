//! Whole native object transfer reuses the existing Scheme type/root ABI.
use super::{InstanceError, NativeRoot, NativeSession, object_api::ObjectApi};
use crate::{BinaryInteger, ObjectKind};
impl<'vm> NativeSession<'vm> {
    fn objects(&self) -> Result<&ObjectApi, InstanceError> {
        self.image
            .native
            .objects
            .as_ref()
            .ok_or_else(|| InstanceError::Loader("full native object ABI unavailable".into()))
    }
    /// Construct an actual Scheme void value.
    /// # Errors
    /// Reports native construction rejection.
    pub fn void_object(&self) -> Result<NativeRoot<'vm>, InstanceError> {
        // SAFETY: native scalar ABI explicitly defines tag zero as void.
        self.own(unsafe { (self.image.native.scalar_create)(0, 0, 0.0) })
    }
    /// Copy bytes into a native bytevector, never a JSON representation.
    /// # Errors
    /// Rejects transfer bounds and failed native creation.
    pub fn bytevector(&self, bytes: &[u8]) -> Result<NativeRoot<'vm>, InstanceError> {
        self.input(bytes.len())?;
        // SAFETY: the borrowed input remains live through the synchronous copy.
        self.own(unsafe { (self.image.input)(bytes.as_ptr(), bytes.len() as u64) })
    }
    /// Construct a canonical arbitrary precision Scheme integer.
    /// # Errors
    /// Rejects missing ABI, input bounds or native creation.
    pub fn big_integer(&self, value: &BinaryInteger) -> Result<NativeRoot<'vm>, InstanceError> {
        self.input(value.magnitude().len())?;
        // SAFETY: BinaryInteger validates sign/magnitude; owner-local synchronous copy.
        self.own(unsafe {
            (self.objects()?.integer)(
                value.magnitude().as_ptr(),
                value.magnitude().len() as u64,
                i32::from(value.is_negative()),
            )
        })
    }
    /// Inspect a rooted object's native Scheme kind.
    /// # Errors
    /// Rejects foreign roots, missing ABI or invalid native kind.
    pub fn kind(&self, value: &NativeRoot<'_>) -> Result<ObjectKind, InstanceError> {
        self.local(value)?;
        // SAFETY: live local root; -1 selects the object itself.
        let tag = unsafe { (self.objects()?.kind)(value.root, -1) };
        crate::runtime::decode_kind(tag).map_err(|_| InstanceError::Native {
            operation: "object kind",
            code: i64::from(tag),
        })
    }
    /// Copy a whole exact integer without narrowing or text serialization.
    /// # Errors
    /// Rejects type, output bounds, missing ABI or noncanonical output.
    pub fn copy_big_integer(&self, value: &NativeRoot<'_>) -> Result<BinaryInteger, InstanceError> {
        self.local(value)?;
        let api = self.objects()?;
        // SAFETY: local strong root remains live through size and copy.
        let size = unsafe { (api.integer_size)(value.root) };
        let size =
            usize::try_from(size).map_err(|_| InstanceError::InvalidConfig("integer size"))?;
        if size > self.maximum_output {
            return Err(InstanceError::InvalidConfig("integer output bound"));
        }
        let mut bytes = vec![0; size];
        // SAFETY: disjoint, writable complete magnitude span.
        let sign = unsafe { (api.integer_copy)(value.root, bytes.as_mut_ptr(), size as u64) };
        if !matches!(sign, 0 | 1) {
            return Err(InstanceError::Native {
                operation: "integer copy",
                code: i64::from(sign),
            });
        }
        BinaryInteger::new(sign == 1, bytes)
            .map_err(|_| InstanceError::InvalidConfig("integer encoding"))
    }
    /// Native list construction, preserving elements and Scheme identity.
    /// # Errors
    /// Rejects foreign roots, bounds and construction failures.
    pub fn list(&self, values: &[&NativeRoot<'_>]) -> Result<NativeRoot<'vm>, InstanceError> {
        self.input(std::mem::size_of_val(values))?;
        let mut roots = Vec::with_capacity(values.len());
        for value in values {
            self.local(value)?;
            roots.push(value.root);
        }
        // SAFETY: complete owner-local span; kind 1 is a native proper list.
        self.own(unsafe { (self.image.native.collection)(roots.as_ptr(), roots.len() as u64, 1) })
    }
    /// Construct an equal?-keyed Gambit table from native pairs.
    /// # Errors
    /// Rejects foreign roots, transfer bounds or construction failure.
    pub fn table(
        &self,
        entries: &[(&NativeRoot<'_>, &NativeRoot<'_>)],
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.input(std::mem::size_of_val(entries))?;
        let mut roots = Vec::with_capacity(entries.len() * 2);
        for (key, value) in entries {
            self.local(key)?;
            self.local(value)?;
            roots.extend([key.root, value.root]);
        }
        // SAFETY: complete key/value root pairs, equality 2 means equal?.
        self.own(unsafe { (self.objects()?.table)(roots.as_ptr(), entries.len() as u64, 2) })
    }
    /// Compare native object identity within one owner.
    /// # Errors
    /// Rejects foreign roots and native errors.
    pub fn identical(
        &self,
        left: &NativeRoot<'_>,
        right: &NativeRoot<'_>,
    ) -> Result<bool, InstanceError> {
        self.local(left)?;
        self.local(right)?;
        // SAFETY: both roots stay live on this owner; no raw Scheme word escapes.
        match unsafe { (self.objects()?.identical)(left.root, -1, right.root, -1) } {
            0 => Ok(false),
            1 => Ok(true),
            code => Err(InstanceError::Native {
                operation: "identity",
                code: i64::from(code),
            }),
        }
    }
    /// Snapshot native collection children; cyclic/shared elements remain handles.
    /// # Errors
    /// Rejects unsupported kinds, output bounds and native snapshot failures.
    pub fn elements(&self, value: &NativeRoot<'_>) -> Result<Vec<NativeRoot<'vm>>, InstanceError> {
        let api = self.objects()?;
        let (snapshot, len) = match self.kind(value)? {
            ObjectKind::GambitTable | ObjectKind::GerbilHashTable => {
                let mut len = 0;
                // SAFETY: table remains rooted; kind 2 returns native alternating entries.
                let root = self.own(unsafe { (api.composite)(value.root, -1, 2, &raw mut len) })?;
                (root, len)
            }
            ObjectKind::Nil | ObjectKind::Pair | ObjectKind::Vector => {
                let kind = i32::from(self.kind(value)? != ObjectKind::Vector);
                // SAFETY: local collection, validated kind; checked native length.
                let len = unsafe { (api.length)(value.root, kind) };
                let len = u64::try_from(len)
                    .map_err(|_| InstanceError::InvalidConfig("collection length"))?;
                if len > (self.maximum_output / 8) as u64 {
                    return Err(InstanceError::InvalidConfig("snapshot bound"));
                }
                // SAFETY: length belongs to this rooted collection, snapshot transfers a root.
                (
                    self.own(unsafe { (api.snapshot)(value.root, kind, len) })?,
                    len,
                )
            }
            _ => return Err(InstanceError::InvalidConfig("not a native collection")),
        };
        let len =
            usize::try_from(len).map_err(|_| InstanceError::InvalidConfig("snapshot length"))?;
        if len > self.maximum_output / 8 {
            return Err(InstanceError::InvalidConfig("snapshot bound"));
        }
        (0..len)
            .map(|index| self.vector_element(&snapshot, index))
            .collect()
    }
    /// Copy a native bytevector to independently owned Rust storage.
    /// # Errors
    /// Rejects non-bytevectors, foreign roots and output bounds.
    pub fn copy_bytes(&self, value: &NativeRoot<'_>) -> Result<Vec<u8>, InstanceError> {
        self.local(value)?;
        self.image.copy_reply(value.root, self.maximum_output)
    }
}
