//! Owned name spans; Scheme roots, not spelling, retain object identity.
use super::{RootedObjectBatch, RootedSchemeObject, SchemeObjectRef, invalid, own};
use crate::runtime::{GerbilRuntime, NativeError, utf8::validated_string};
use gerbil_scheme_sys as sys;

/// Identifier family; a spelling is descriptive data, not a unique identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentifierKind {
    Symbol,
    Keyword,
}
impl IdentifierKind {
    const fn code(self) -> i32 {
        match self {
            Self::Symbol => 0,
            Self::Keyword => 1,
        }
    }
}
/// Reused transactional UTF-8 names, count+1 offsets and family tags.
/// No Scheme handle or heap pointer remains after successful copy.
#[derive(Debug, Default)]
pub struct IdentifierBuffer {
    names: String,
    offsets: Vec<u64>,
    kinds: Vec<u8>,
    staged: Vec<u8>,
    staged_offsets: Vec<u64>,
    staged_kinds: Vec<u8>,
}
impl IdentifierBuffer {
    #[must_use]
    pub fn len(&self) -> usize {
        self.kinds.len()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }
    /// Borrow a validated name without rescanning its UTF-8 bytes.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<(IdentifierKind, &str)> {
        let kind = match *self.kinds.get(index)? {
            0 => IdentifierKind::Symbol,
            1 => IdentifierKind::Keyword,
            _ => return None,
        };
        let start = usize::try_from(*self.offsets.get(index)?).ok()?;
        let end = usize::try_from(*self.offsets.get(index + 1)?).ok()?;
        Some((kind, self.names.get(start..end)?))
    }
    /// Retained Rust payload and staging capacity; excludes six Vec/String headers.
    #[must_use]
    pub fn reserved_bytes(&self) -> usize {
        self.names.capacity()
            + self.staged.capacity()
            + (self.offsets.capacity() + self.staged_offsets.capacity()) * 8
            + self.kinds.capacity()
            + self.staged_kinds.capacity()
    }
    fn clear(&mut self) {
        self.names.clear();
        self.offsets.clear();
        self.kinds.clear();
    }
    fn copy(&mut self, root: i64, index: i64) -> Result<(), NativeError> {
        let mut metadata = [0_u64; 2];
        // SAFETY: live owner-local source and fixed disjoint metadata span.
        let packet = own(unsafe {
            sys::gerbil_scheme_rust_identifiers_prepare(root, index, metadata.as_mut_ptr())
        })?;
        let count = usize::try_from(metadata[0]).map_err(|_| invalid("identifier count"))?;
        let capacity = usize::try_from(metadata[1]).map_err(|_| invalid("identifier capacity"))?;
        self.staged.resize(capacity, 0);
        self.staged_offsets.resize(
            count
                .checked_add(1)
                .ok_or_else(|| invalid("identifier offsets"))?,
            0,
        );
        self.staged_kinds.resize(count, 0);
        // SAFETY: the prepared packet root stays live. Initialized disjoint spans
        // match its admitted shape; no Rust pointer is retained by Scheme.
        let written = unsafe {
            sys::gerbil_scheme_rust_identifiers_copy(
                packet.root_id().0,
                self.staged.as_mut_ptr(),
                metadata[1],
                self.staged_offsets.as_mut_ptr(),
                self.staged_kinds.as_mut_ptr(),
                metadata[0],
            )
        };
        let written = usize::try_from(written).map_err(|_| invalid("identifier copy"))?;
        if written > capacity {
            return Err(invalid("identifier length"));
        }
        self.staged.truncate(written);
        let names = validated_string(std::mem::take(&mut self.staged))?;
        if self.staged_offsets.first() != Some(&0)
            || self.staged_offsets.last() != Some(&(written as u64))
        {
            return Err(invalid("identifier endpoints"));
        }
        for (offsets, kind) in self.staged_offsets.windows(2).zip(&self.staged_kinds) {
            let start = usize::try_from(offsets[0]).map_err(|_| invalid("identifier offset"))?;
            let end = usize::try_from(offsets[1]).map_err(|_| invalid("identifier offset"))?;
            if *kind > 1 || names.get(start..end).is_none() {
                return Err(invalid("identifier span"));
            }
        }
        let previous = std::mem::replace(&mut self.names, names);
        self.staged = previous.into_bytes();
        self.staged.clear();
        std::mem::swap(&mut self.offsets, &mut self.staged_offsets);
        std::mem::swap(&mut self.kinds, &mut self.staged_kinds);
        self.staged_offsets.clear();
        self.staged_kinds.clear();
        Ok(())
    }
}
impl GerbilRuntime {
    /// Intern using the official Scheme constructor, retaining one strong root.
    /// Name equality does not authorize merging arbitrary existing symbols.
    /// # Errors
    /// Rejects wrong-thread entry or failed Scheme construction.
    pub fn intern_identifier(
        &self,
        kind: IdentifierKind,
        name: &str,
    ) -> Result<RootedSchemeObject<'_>, NativeError> {
        self.identifier_from_name(kind.code(), name)
    }
    /// Construct a fresh symbol; identical spelling does not imply identity.
    /// # Errors
    /// Rejects wrong-thread entry or failed official Scheme construction.
    pub fn uninterned_symbol(&self, name: &str) -> Result<RootedSchemeObject<'_>, NativeError> {
        self.identifier_from_name(2, name)
    }
    fn identifier_from_name(
        &self,
        kind: i32,
        name: &str,
    ) -> Result<RootedSchemeObject<'_>, NativeError> {
        self.check_thread()?;
        let name = self.string_from_utf8(name)?;
        // SAFETY: the input string root lives throughout the official conversion.
        let root =
            unsafe { sys::gerbil_scheme_rust_identifier_create(name.owner.root_id().0, kind) };
        own(root).map(|owner| RootedSchemeObject { owner })
    }
}
impl SchemeObjectRef<'_> {
    /// Official Scheme eq? on owner-bound values, including indexed snapshots.
    /// Neither root-token equality nor copied name equality substitutes for this.
    #[must_use]
    pub fn is_identical(self, other: Self) -> bool {
        // SAFETY: live owner-bound references cannot escape or cross threads.
        unsafe {
            sys::gerbil_scheme_rust_object_identical(
                self.root.0,
                self.index,
                other.root.0,
                other.index,
            ) == 1
        }
    }
    /// Copy a single identifier or complete identifier vector into owned names.
    /// # Errors
    /// Mixed unsupported types or failed encoding leave published output unchanged.
    pub fn copy_identifier_names(self, output: &mut IdentifierBuffer) -> Result<(), NativeError> {
        output.copy(self.root.0, self.index)
    }
}
impl RootedObjectBatch<'_> {
    /// Project every identifier in one rooted snapshot, with one packet transfer.
    /// # Errors
    /// Rejects unsupported elements without replacing the previous result.
    pub fn copy_identifier_names(&self, output: &mut IdentifierBuffer) -> Result<(), NativeError> {
        if let Some(first) = self.get(0) {
            output.copy(first.root.0, -1)
        } else {
            output.clear();
            Ok(())
        }
    }
}
