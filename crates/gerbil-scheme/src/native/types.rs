// SPDX-License-Identifier: LGPL-2.1-or-later OR Apache-2.0

//! Public value types shared by the safe native Gerbil binding owners.

use super::NativeError;
use std::marker::PhantomData;
use std::num::{NonZeroU8, NonZeroUsize};
use std::rc::Rc;
use std::thread::ThreadId;

/// Exclusive, thread-affine ownership of the in-process Gerbil runtime.
///
/// This handle is deliberately neither [`Send`] nor [`Sync`]. Its existence
/// proves that the process-global runtime was initialized successfully and that
/// the binding module reported the expected ABI version.
#[derive(Debug)]
pub struct GerbilRuntime {
    pub(super) owner: ThreadId,
    pub(super) _not_send_or_sync: PhantomData<Rc<()>>,
}

/// Public receipt describing the initialized native runtime binding surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilRuntimeReceipt {
    /// Stable ABI family identifier, including its terminating NUL byte.
    pub abi_id: &'static [u8],
    /// ABI major version accepted by this Rust binding.
    pub abi_version: u32,
    /// Repository-relative public C header path.
    pub header_path: &'static str,
    /// Gerbil module initialized for the native bridge.
    pub native_module_path: &'static str,
}

impl GerbilRuntimeReceipt {
    /// Runtime module loaded by the native bridge.
    pub const NATIVE_MODULE_PATH: &'static str = "gerbil-scheme-rust/scheme/native";
}

/// Safe borrowed UTF-8 view for native Gerbil calls.
///
/// The Rust string owner keeps the bytes alive for the full borrow. The raw
/// [`gerbil_scheme_sys::GerbilBorrowedUtf8`] value may be passed to native code,
/// but the callee must not retain or free its pointer.
#[derive(Clone, Copy, Debug)]
pub struct GerbilUtf8<'a> {
    text: &'a str,
    abi: gerbil_scheme_sys::GerbilBorrowedUtf8,
}

impl<'a> GerbilUtf8<'a> {
    /// Borrow a Rust string as the stable native UTF-8 ABI shape.
    #[must_use]
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            abi: gerbil_scheme_sys::GerbilBorrowedUtf8::from(text),
        }
    }

    /// Return the Rust owner-side string view.
    #[must_use]
    pub fn as_str(self) -> &'a str {
        self.text
    }

    /// Return the raw C ABI view for one native call.
    #[must_use]
    pub fn as_abi(self) -> gerbil_scheme_sys::GerbilBorrowedUtf8 {
        self.abi
    }

    /// Return the UTF-8 byte length.
    #[must_use]
    pub fn len(self) -> usize {
        self.abi.len
    }

    /// Return whether the string is empty.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.abi.len == 0
    }
}

impl<'a> From<&'a str> for GerbilUtf8<'a> {
    fn from(value: &'a str) -> Self {
        Self::new(value)
    }
}

/// Provenance attached to a borrowed Gerbil value handle.
///
/// This is deliberately narrower than "non-zero word".  Runtime-backed
/// Scheme predicates and traversal must only use handles whose provenance is
/// produced by an initialized runtime/export path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GerbilValueProvenance {
    /// A raw non-zero handle supplied by the caller.
    ///
    /// This keeps tests and FFI boundaries fail-closed: the pointer is not
    /// trusted as a live Gambit/Gerbil object.
    UntrustedRaw,
    /// A Rust-owned sentinel handle produced through an initialized runtime API path.
    ///
    /// This proves the handle was produced while the owning [`GerbilRuntime`]
    /// was alive. It is not a live Gambit/Gerbil object and does not imply type,
    /// GC rooting, or traversal safety.
    RuntimeSentinel,
    /// A borrowed Scheme object produced by the initialized Gerbil native module.
    ///
    /// This proves the handle came from a Gerbil `scheme-object` export while
    /// the owning [`GerbilRuntime`] was alive. It is still borrowed and
    /// unrooted, so traversal and retention remain gated by later APIs.
    SchemeObjectExport,
}

/// Runtime-borrowed opaque Gerbil value handle.
///
/// This wrapper is intentionally non-owning. It proves only that the raw handle
/// is non-zero. Runtime provenance is tracked explicitly by
/// [`GerbilValueProvenance`]; a caller-created raw handle is not enough to
/// claim type, ownership, GC reachability, or validity as a Gambit object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilValue<'runtime> {
    pub(super) raw: NonZeroUsize,
    pub(super) provenance: GerbilValueProvenance,
    pub(super) _runtime: PhantomData<&'runtime GerbilRuntime>,
}

/// Borrowed pair parts projected from a runtime-backed pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemePairParts<'runtime> {
    /// Projected car value.
    pub car: GerbilValue<'runtime>,
    /// Projected cdr value.
    pub cdr: GerbilValue<'runtime>,
}

/// Borrowed, runtime-backed Scheme nil / empty-list marker.
///
/// This proves only that a Gerbil-owned Scheme object export satisfied `null?`
/// at projection time. It does not root, retain, or own the underlying object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeNil<'runtime> {
    pub(super) raw: NonZeroUsize,
    pub(super) _runtime: PhantomData<&'runtime GerbilRuntime>,
}

/// Borrowed, runtime-backed Scheme void marker.
///
/// This proves only that a Gerbil-owned Scheme object export satisfied `void?`
/// at projection time. It does not root, retain, or own the underlying object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeVoid<'runtime> {
    raw: NonZeroUsize,
    _runtime: PhantomData<&'runtime GerbilRuntime>,
}

impl SchemeVoid<'_> {
    pub(super) fn from_raw(raw: usize) -> Option<Self> {
        NonZeroUsize::new(raw).map(|raw| Self {
            raw,
            _runtime: PhantomData,
        })
    }

    #[must_use]
    pub fn as_raw(&self) -> usize {
        self.raw.get()
    }
}

/// Borrowed, runtime-backed Scheme bytevector marker.
///
/// This proves only that a Gerbil-owned Scheme object export satisfied
/// `u8vector?` at projection time. It does not root, retain, or own the
/// underlying object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeBytevector<'runtime> {
    pub(super) raw: NonZeroUsize,
    pub(super) _runtime: PhantomData<&'runtime GerbilRuntime>,
}

/// Borrowed, runtime-backed Scheme exact-integer marker.
///
/// This preserves the identity of fixnums and bignums without claiming that an
/// arbitrary-size integer can cross the C ABI by value. Machine projections are
/// checked explicitly through [`ExactIntegerTarget`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemeExactInteger<'runtime> {
    pub(super) raw: NonZeroUsize,
    pub(super) _runtime: PhantomData<&'runtime GerbilRuntime>,
}

/// Delimiter policy for Gerbil hexadecimal bytestring conversions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BytestringDelimiter {
    /// Emit two uppercase hexadecimal digits per byte with no separator.
    Compact,
    /// Place one Unicode scalar between adjacent encoded bytes.
    Character(char),
}

impl BytestringDelimiter {
    /// Gerbil's default bytestring representation: uppercase bytes separated by spaces.
    pub const SPACE: Self = Self::Character(' ');

    pub(super) const fn abi_code(self) -> i32 {
        match self {
            Self::Compact => -1,
            Self::Character(character) => character as i32,
        }
    }
}

impl Default for BytestringDelimiter {
    fn default() -> Self {
        Self::SPACE
    }
}

/// Byte order used by Gerbil integer/bytevector conversions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ByteOrder {
    /// Most-significant byte first, matching Gerbil's default.
    #[default]
    Big,
    /// Least-significant byte first.
    Little,
    /// Native byte order of the compiled Rust/Gerbil runtime.
    Native,
}

impl ByteOrder {
    pub(super) const fn abi_code(self) -> i32 {
        match self {
            Self::Big => gerbil_scheme_sys::GerbilByteOrder::Big.code(),
            Self::Little => gerbil_scheme_sys::GerbilByteOrder::Little.code(),
            Self::Native => gerbil_scheme_sys::GerbilByteOrder::Native.code(),
        }
    }
}

/// Checked byte width for `u64` / `i64` bytevector conversion.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntegerWidth(NonZeroU8);

impl IntegerWidth {
    /// Largest supported width for the machine-integer ABI.
    pub const MAX: u8 = gerbil_scheme_sys::GERBIL_SCHEME_RUST_MAX_INTEGER_BYTES;

    /// Construct a non-zero width no larger than eight bytes.
    #[must_use]
    pub const fn new(width: u8) -> Option<Self> {
        match NonZeroU8::new(width) {
            Some(width) if width.get() <= Self::MAX => Some(Self(width)),
            Some(_) | None => None,
        }
    }

    /// Return the width in bytes.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0.get()
    }
}

/// Width, byte-order, and overflow policy for integer-to-bytevector encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntegerEncoding {
    byte_order: ByteOrder,
    width: Option<IntegerWidth>,
    truncating: bool,
}

impl IntegerEncoding {
    /// Use Gerbil's minimal-width encoding for the selected byte order.
    #[must_use]
    pub const fn minimal(byte_order: ByteOrder) -> Self {
        Self {
            byte_order,
            width: None,
            truncating: false,
        }
    }

    /// Use an explicit fixed width and reject values that do not fit.
    #[must_use]
    pub const fn fixed(byte_order: ByteOrder, width: IntegerWidth) -> Self {
        Self {
            byte_order,
            width: Some(width),
            truncating: false,
        }
    }

    /// Explicitly opt into Gerbil's fixed-width high-bit truncation semantics.
    #[must_use]
    pub const fn truncating(mut self) -> Self {
        self.truncating = true;
        self
    }

    /// Selected byte order.
    #[must_use]
    pub const fn byte_order(self) -> ByteOrder {
        self.byte_order
    }

    /// Explicit width, or `None` for Gerbil's minimal representation.
    #[must_use]
    pub const fn width(self) -> Option<IntegerWidth> {
        self.width
    }

    /// Whether a too-small fixed width may truncate high bits.
    #[must_use]
    pub const fn allows_truncation(self) -> bool {
        self.truncating
    }
}

impl Default for IntegerEncoding {
    fn default() -> Self {
        Self::minimal(ByteOrder::Big)
    }
}

/// Byte-order and optional prefix width for bytevector-to-integer decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntegerDecoding {
    byte_order: ByteOrder,
    width: Option<IntegerWidth>,
}

impl IntegerDecoding {
    /// Decode the complete bytevector, matching Gerbil's default size.
    #[must_use]
    pub const fn entire(byte_order: ByteOrder) -> Self {
        Self {
            byte_order,
            width: None,
        }
    }

    /// Decode exactly the first `width` bytes of the bytevector.
    #[must_use]
    pub const fn prefix(byte_order: ByteOrder, width: IntegerWidth) -> Self {
        Self {
            byte_order,
            width: Some(width),
        }
    }

    /// Selected byte order.
    #[must_use]
    pub const fn byte_order(self) -> ByteOrder {
        self.byte_order
    }

    /// Prefix width, or `None` to decode the complete bytevector.
    #[must_use]
    pub const fn width(self) -> Option<IntegerWidth> {
        self.width
    }
}

impl Default for IntegerDecoding {
    fn default() -> Self {
        Self::entire(ByteOrder::Big)
    }
}

/// Owned root for a Scheme string created by a native conversion.
///
/// The root is released automatically on the Gerbil runtime owner thread. It
/// is deliberately neither [`Clone`] nor [`Copy`] because release has exactly
/// one owner.
#[derive(Debug)]
pub struct RootedSchemeString<'runtime> {
    pub(super) owner: RootedSchemeOwner<'runtime>,
}

/// Owned root for a Scheme bytevector created by a native conversion.
///
/// The root is released automatically on the Gerbil runtime owner thread. It
/// is deliberately neither [`Clone`] nor [`Copy`] because release has exactly
/// one owner.
#[derive(Debug)]
pub struct RootedSchemeBytevector<'runtime> {
    pub(super) owner: RootedSchemeOwner<'runtime>,
}

/// Single private owner for one live Scheme root.
///
/// Public typed wrappers compose this owner so root validation and release
/// cannot drift across Scheme value families.
#[derive(Debug)]
pub(super) struct RootedSchemeOwner<'runtime> {
    root: gerbil_scheme_sys::GerbilRootId,
    _runtime: PhantomData<&'runtime GerbilRuntime>,
}

impl RootedSchemeOwner<'_> {
    pub(super) fn new(
        status: gerbil_scheme_sys::GerbilStatus,
        root: gerbil_scheme_sys::GerbilRootId,
        operation: &'static str,
    ) -> Result<Self, NativeError> {
        if status == gerbil_scheme_sys::GerbilStatus::Ok && root.is_valid() {
            Ok(Self {
                root,
                _runtime: PhantomData,
            })
        } else {
            Err(NativeError::Status {
                operation,
                code: if status == gerbil_scheme_sys::GerbilStatus::Ok {
                    gerbil_scheme_sys::GerbilStatus::InvalidValue as i32
                } else {
                    status as i32
                },
            })
        }
    }

    pub(super) const fn root_id(&self) -> gerbil_scheme_sys::GerbilRootId {
        self.root
    }
}

impl Drop for RootedSchemeOwner<'_> {
    fn drop(&mut self) {
        let status = unsafe { gerbil_scheme_sys::gerbil_scheme_rust_root_release(self.root) };
        debug_assert_eq!(status, gerbil_scheme_sys::GerbilStatus::Ok);
    }
}

/// Runtime-backed type carried by a [`RootedSchemeValue`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootedSchemeValueKind {
    /// A Scheme exact integer, including both fixnums and bignums.
    ExactInteger,
    /// A Scheme string.
    String,
    /// A Scheme bytevector.
    Bytevector,
}

/// One owned Scheme root with a type-preserving safe Rust projection.
///
/// Each variant retains the existing typed wrapper as the sole owner of the
/// native root. Dropping this value therefore releases exactly one Scheme
/// root without introducing a parallel C ABI or a second ownership path.
#[derive(Debug)]
pub enum RootedSchemeValue<'runtime> {
    /// A rooted Scheme exact integer.
    ExactInteger(RootedSchemeExactInteger<'runtime>),
    /// A rooted Scheme string.
    String(RootedSchemeString<'runtime>),
    /// A rooted Scheme bytevector.
    Bytevector(RootedSchemeBytevector<'runtime>),
}

impl<'runtime> RootedSchemeValue<'runtime> {
    /// Return the runtime-backed type carried by this root.
    #[must_use]
    pub const fn kind(&self) -> RootedSchemeValueKind {
        match self {
            Self::ExactInteger(_) => RootedSchemeValueKind::ExactInteger,
            Self::String(_) => RootedSchemeValueKind::String,
            Self::Bytevector(_) => RootedSchemeValueKind::Bytevector,
        }
    }

    /// Borrow the typed exact-integer projection when this root carries one.
    #[must_use]
    pub const fn as_exact_integer(&self) -> Option<&RootedSchemeExactInteger<'runtime>> {
        match self {
            Self::ExactInteger(value) => Some(value),
            Self::String(_) | Self::Bytevector(_) => None,
        }
    }

    /// Borrow the typed string projection when this root carries one.
    #[must_use]
    pub const fn as_string(&self) -> Option<&RootedSchemeString<'runtime>> {
        match self {
            Self::String(value) => Some(value),
            Self::ExactInteger(_) | Self::Bytevector(_) => None,
        }
    }

    /// Borrow the typed bytevector projection when this root carries one.
    #[must_use]
    pub const fn as_bytevector(&self) -> Option<&RootedSchemeBytevector<'runtime>> {
        match self {
            Self::Bytevector(value) => Some(value),
            Self::ExactInteger(_) | Self::String(_) => None,
        }
    }
}

impl<'runtime> From<RootedSchemeExactInteger<'runtime>> for RootedSchemeValue<'runtime> {
    fn from(value: RootedSchemeExactInteger<'runtime>) -> Self {
        Self::ExactInteger(value)
    }
}

impl<'runtime> From<RootedSchemeString<'runtime>> for RootedSchemeValue<'runtime> {
    fn from(value: RootedSchemeString<'runtime>) -> Self {
        Self::String(value)
    }
}

impl<'runtime> From<RootedSchemeBytevector<'runtime>> for RootedSchemeValue<'runtime> {
    fn from(value: RootedSchemeBytevector<'runtime>) -> Self {
        Self::Bytevector(value)
    }
}

/// Owned root for a Scheme exact integer created from a Rust machine integer.
///
/// The Scheme object may be a fixnum or bignum depending on its magnitude. The
/// root has one Rust owner and releases automatically on drop.
#[derive(Debug)]
pub struct RootedSchemeExactInteger<'runtime> {
    pub(super) owner: RootedSchemeOwner<'runtime>,
}

/// Rust machine target requested for a checked Scheme exact-integer projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactIntegerTarget {
    /// Signed 64-bit integer.
    I64,
    /// Unsigned 64-bit integer.
    U64,
    /// Target-platform pointer-sized unsigned integer.
    Usize,
}
