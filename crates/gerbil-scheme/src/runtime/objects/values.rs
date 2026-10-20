//! Strong Scheme roots: no GC-managed words escape collection ownership.
use crate::runtime::RootedSchemeOwner;
use crate::runtime::{
    GerbilRuntime, GerbilValue, GerbilValueProvenance, NativeError, SchemeScalar,
};
use gerbil_scheme_sys::{self as sys, GerbilRootId, GerbilStatus};
use std::marker::PhantomData;

/// Scheme object shape; ownership and application meaning remain separate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectKind {
    Void,
    Boolean,
    ExactInteger,
    String,
    Nil,
    Pair,
    Vector,
    GambitTable,
    GerbilHashTable,
    Character,
    Flonum,
    Bytevector,
    Symbol,
    Keyword,
    Procedure,
    Other,
}
impl SchemeObjectRef<'_> {
    /// Inspect a live strongly rooted value, including false/zero and snapshot slots.
    /// # Errors
    /// Rejects invalid native roots and snapshot indices.
    pub fn kind(self) -> Result<ObjectKind, NativeError> {
        // SAFETY: this reference borrows its owner root and cannot cross threads.
        let tag = unsafe { sys::gerbil_scheme_rust_object_kind(self.root.0, self.index) };
        decode_kind(tag)
    }
}
pub(crate) fn decode_kind(tag: i32) -> Result<ObjectKind, NativeError> {
    Ok(match tag {
        0 => ObjectKind::Void,
        1 => ObjectKind::Boolean,
        2 => ObjectKind::ExactInteger,
        3 => ObjectKind::String,
        4 => ObjectKind::Nil,
        5 => ObjectKind::Pair,
        6 => ObjectKind::Vector,
        7 => ObjectKind::GambitTable,
        8 => ObjectKind::GerbilHashTable,
        9 => ObjectKind::Character,
        10 => ObjectKind::Flonum,
        11 => ObjectKind::Bytevector,
        12 => ObjectKind::Symbol,
        13 => ObjectKind::Keyword,
        14 => ObjectKind::Procedure,
        15 => ObjectKind::Other,
        _ => return Err(invalid("object kind")),
    })
}
pub(super) fn invalid(operation: &'static str) -> NativeError {
    NativeError::Status {
        operation,
        code: GerbilStatus::InvalidValue as i32,
    }
}

/// Single owner of any strongly rooted Scheme object, including false and zero.
#[derive(Debug)]
pub struct RootedSchemeObject<'runtime> {
    pub(super) owner: RootedSchemeOwner<'runtime>,
}
impl RootedSchemeObject<'_> {
    pub(in crate::runtime) fn root_id(&self) -> GerbilRootId {
        self.owner.root_id()
    }
    /// Borrow an object without extending its root lifetime.
    #[must_use]
    pub fn as_ref(&self) -> SchemeObjectRef<'_> {
        SchemeObjectRef {
            root: self.owner.root_id(),
            index: -1,
            _runtime: PhantomData,
        }
    }
}
impl<'runtime> RootedSchemeObject<'runtime> {
    pub(in crate::runtime) fn from_owner(owner: RootedSchemeOwner<'runtime>) -> Self {
        Self { owner }
    }
    /// Check a string root and transfer its unique ownership without copying.
    /// # Errors
    /// Rejects a non-string and releases its consumed root.
    pub fn into_string(self) -> Result<crate::RootedSchemeString<'runtime>, NativeError> {
        if self.as_ref().kind()? != ObjectKind::String {
            return Err(invalid("string object"));
        }
        Ok(crate::RootedSchemeString { owner: self.owner })
    }
    /// Transfer a checked bytevector root without copying or borrowing GC words.
    /// # Errors
    /// Rejects a different object kind, releasing the consumed root.
    pub fn into_bytevector(self) -> Result<crate::RootedSchemeBytevector<'runtime>, NativeError> {
        if self.as_ref().kind()? != ObjectKind::Bytevector {
            return Err(invalid("bytevector object"));
        }
        Ok(crate::RootedSchemeBytevector { owner: self.owner })
    }
    /// Check a collection shape and transfer its unique ownership without copying.
    /// # Errors
    /// Rejects the wrong shape, improper lists or native length overflow.
    pub fn into_collection(
        self,
        kind: CollectionKind,
    ) -> Result<RootedSchemeCollection<'runtime>, NativeError> {
        // SAFETY: live strong owner; native shape and proper-list checks precede length.
        let length = unsafe {
            sys::gerbil_scheme_rust_collection_length_raw(self.owner.root_id().0, kind.code())
        };
        let length = usize::try_from(length).map_err(|_| invalid("collection object"))?;
        Ok(RootedSchemeCollection {
            owner: self.owner,
            kind,
            length,
        })
    }
}
impl<'runtime> crate::RootedSchemeString<'runtime> {
    /// Transfer the string's unique strong owner to the generic object API.
    #[must_use]
    pub fn into_object(self) -> RootedSchemeObject<'runtime> {
        RootedSchemeObject { owner: self.owner }
    }
}
impl<'runtime> crate::RootedSchemeBytevector<'runtime> {
    /// Transfer the bytevector's unique strong owner to the generic object API.
    #[must_use]
    pub fn into_object(self) -> RootedSchemeObject<'runtime> {
        RootedSchemeObject { owner: self.owner }
    }
}

/// Root-bound object reference; cannot outlive its owner or cross threads.
#[derive(Clone, Copy, Debug)]
pub struct SchemeObjectRef<'owner> {
    pub(super) root: GerbilRootId,
    pub(super) index: i64,
    _runtime: PhantomData<&'owner GerbilRuntime>,
}
impl SchemeObjectRef<'_> {
    /// Owned scalar projection; no borrowed Scheme word survives this operation.
    /// # Errors
    /// Rejects non-scalar values or invalid native payloads.
    pub fn scalar(self) -> Result<SchemeScalar, NativeError> {
        let mut raw = sys::GerbilScalarProjection::default();
        // SAFETY: root-bound lifetime; the word stays inside Gambit's GC boundary.
        let status = unsafe {
            sys::gerbil_scheme_rust_object_scalar_indexed(self.root.0, self.index, &raw mut raw)
        };
        if status != 0 {
            return Err(invalid("object scalar"));
        }
        match raw.tag {
            1 => Ok(SchemeScalar::Boolean(raw.bits != 0)),
            2 => isize::try_from(i64::from_ne_bytes(raw.bits.to_ne_bytes()))
                .map(SchemeScalar::Fixnum)
                .map_err(|_| invalid("fixnum")),
            3 => u32::try_from(raw.bits)
                .ok()
                .and_then(char::from_u32)
                .map(SchemeScalar::Char)
                .ok_or_else(|| invalid("character")),
            4 => Ok(SchemeScalar::Flonum(f64::from_bits(raw.bits))),
            _ => Err(invalid("scalar tag")),
        }
    }
}

/// Scheme collection shape, never a version or execution-policy flag.
#[derive(Clone, Copy, Debug)]
pub enum CollectionKind {
    Vector,
    List,
    Pair,
}
impl CollectionKind {
    const fn code(self) -> i32 {
        match self {
            Self::Vector => 0,
            Self::List => 1,
            Self::Pair => 2,
        }
    }
}
/// Strong collection root; its elements survive release of input roots.
#[derive(Debug)]
pub struct RootedSchemeCollection<'runtime> {
    owner: RootedSchemeOwner<'runtime>,
    kind: CollectionKind,
    length: usize,
}
impl<'runtime> RootedSchemeCollection<'runtime> {
    /// Element count (pairs expose car and cdr as two elements).
    #[must_use]
    pub fn len(&self) -> usize {
        self.length
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
    /// Root one element. Prefer `copy_into` for a complete collection.
    /// # Errors
    /// Rejects invalid indices, roots or collection shapes.
    pub fn element(&self, index: usize) -> Result<RootedSchemeObject<'_>, NativeError> {
        let root = unsafe {
            sys::gerbil_scheme_rust_collection_element_raw(
                self.owner.root_id().0,
                self.kind.code(),
                index as u64,
            )
        };
        own(root).map(|owner| RootedSchemeObject { owner })
    }
    /// Copy all element references into one strongly rooted Scheme snapshot.
    /// Old output remains owned until the complete new batch is admitted.
    /// # Errors
    /// Rejects shape/capacity errors without replacing the active root batch.
    pub fn copy_into(&self, output: &mut RootedObjectBatch<'runtime>) -> Result<(), NativeError> {
        let len = self.length;
        let root = unsafe {
            sys::gerbil_scheme_rust_collection_snapshot_raw(
                self.owner.root_id().0,
                self.kind.code(),
                len as u64,
            )
        };
        let owner = own(root)?;
        output.owner = Some(owner);
        output.length = len;
        Ok(())
    }
    /// Transfer the collection's sole root owner to the generic object surface.
    #[must_use]
    pub fn into_object(self) -> RootedSchemeObject<'runtime> {
        RootedSchemeObject { owner: self.owner }
    }
}
pub(super) fn own<'r>(root: i64) -> Result<RootedSchemeOwner<'r>, NativeError> {
    RootedSchemeOwner::new(
        if root > 0 {
            GerbilStatus::Ok
        } else {
            GerbilStatus::InvalidValue
        },
        GerbilRootId(root),
        "strong object root",
    )
}
/// One strong snapshot root retains all elements without per-element roots.
///
/// Owned roots cannot escape the runtime lifetime.
/// ```compile_fail
/// use gerbil_scheme::{CollectionKind, GerbilRuntime, RootedObjectBatch};
/// let mut batch = RootedObjectBatch::default();
/// {
///     let runtime = GerbilRuntime::initialize().unwrap();
///     let collection = runtime.collection_from_objects(CollectionKind::Vector, &[]).unwrap();
///     collection.copy_into(&mut batch).unwrap();
/// }
/// drop(batch);
/// ```
/// Nor can a rooted batch be sent to a different owner thread.
/// ```compile_fail
/// use gerbil_scheme::RootedObjectBatch;
/// fn send<T: Send>() {}
/// send::<RootedObjectBatch<'static>>();
/// ```
#[derive(Debug, Default)]
pub struct RootedObjectBatch<'runtime> {
    owner: Option<RootedSchemeOwner<'runtime>>,
    length: usize,
}
impl<'r> RootedObjectBatch<'r> {
    pub(super) fn replace_snapshot(&mut self, owner: RootedSchemeOwner<'r>, length: usize) {
        self.owner = Some(owner);
        self.length = length;
    }
    /// Release the single snapshot root and all its retained element references.
    pub fn clear(&mut self) {
        self.owner = None;
        self.length = 0;
    }
    /// Borrow one element. Its root cannot outlive this batch borrow.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<SchemeObjectRef<'_>> {
        if index >= self.length {
            return None;
        }
        Some(SchemeObjectRef {
            root: self.owner.as_ref()?.root_id(),
            index: i64::try_from(index).ok()?,
            _runtime: PhantomData,
        })
    }
    /// Root one snapshot element independently. Prefer borrowing the complete
    /// snapshot when all elements are needed: this entry allocates another root.
    /// # Errors
    /// Rejects an out-of-bounds element or failed strong-root registration.
    pub fn root_element(&self, index: usize) -> Result<RootedSchemeObject<'r>, NativeError> {
        let reference = self.get(index).ok_or_else(|| invalid("snapshot index"))?;
        // SAFETY: live snapshot vector root and checked element index.
        let root = unsafe {
            sys::gerbil_scheme_rust_collection_element_raw(reference.root.0, 0, index as u64)
        };
        own(root).map(|owner| RootedSchemeObject { owner })
    }
    #[must_use]
    pub fn len(&self) -> usize {
        self.length
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
    /// Inline Rust snapshot descriptor, excluding Scheme vector/registry storage.
    #[must_use]
    pub fn reserved_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

impl GerbilRuntime {
    /// Allocate a strongly rooted scalar through the official Scheme constructors.
    /// # Errors
    /// Rejects a wrong owner, out-of-range fixnum or native allocation failure.
    pub fn scalar_object(
        &self,
        value: SchemeScalar,
    ) -> Result<RootedSchemeObject<'_>, NativeError> {
        let (tag, integer, real) = match value {
            SchemeScalar::Boolean(value) => (1, i64::from(value), 0.0),
            SchemeScalar::Fixnum(value) => (2, value as i64, 0.0),
            SchemeScalar::Char(value) => (3, i64::from(u32::from(value)), 0.0),
            SchemeScalar::Flonum(value) => (4, 0, value),
        };
        self.marker_object(tag, integer, real)
    }
    /// Create a fresh strong owner of Scheme void.
    /// # Errors
    /// Rejects wrong-thread entry or native allocation failure.
    pub fn void_object(&self) -> Result<RootedSchemeObject<'_>, NativeError> {
        self.marker_object(0, 0, 0.0)
    }
    /// Create a fresh strong owner of the empty Scheme list.
    /// # Errors
    /// Rejects wrong-thread entry or native allocation failure.
    pub fn nil_object(&self) -> Result<RootedSchemeObject<'_>, NativeError> {
        self.marker_object(5, 0, 0.0)
    }
    fn marker_object(
        &self,
        tag: i32,
        integer: i64,
        real: f64,
    ) -> Result<RootedSchemeObject<'_>, NativeError> {
        self.check_thread()?;
        // SAFETY: owner checked; the native constructor returns a fresh strong root.
        let root = unsafe { sys::gerbil_scheme_rust_scalar_object(tag, integer, real) };
        own(root).map(|owner| RootedSchemeObject { owner })
    }
    /// Strongly root a trusted borrowed object, including Scheme false/zero.
    /// # Errors
    /// Rejects untrusted provenance, foreign owners or failed root creation.
    pub fn root_value(
        &self,
        value: GerbilValue<'_>,
    ) -> Result<RootedSchemeObject<'_>, NativeError> {
        self.check_thread()?;
        if value.provenance != GerbilValueProvenance::SchemeObjectExport {
            return Err(invalid("object provenance"));
        }
        let root = unsafe { sys::gerbil_scheme_rust_object_root_raw(value.as_raw()) };
        own(root).map(|owner| RootedSchemeObject { owner })
    }
    /// Construct a collection with strong references to input objects.
    /// # Errors
    /// Rejects a wrong owner, invalid roots or a pair with other than two fields.
    pub fn collection_from_objects(
        &self,
        kind: CollectionKind,
        objects: &[&RootedSchemeObject<'_>],
    ) -> Result<RootedSchemeCollection<'_>, NativeError> {
        self.check_thread()?;
        let roots: Vec<_> = objects.iter().map(|v| v.owner.root_id()).collect();
        let root = unsafe {
            sys::gerbil_scheme_rust_collection_create_raw(
                roots.as_ptr(),
                roots.len() as u64,
                kind.code(),
            )
        };
        own(root).map(|owner| RootedSchemeCollection {
            owner,
            kind,
            length: roots.len(),
        })
    }
}
