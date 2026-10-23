//! A private native row snapshot pins identity across sizing, GC and bulk copy.
use super::{InstanceError, NativeRoot, NativeSession, preflight::symbol};
use crate::NumericElement;
use std::{ffi::c_void, marker::PhantomData};
#[derive(Debug)]
pub(super) struct Transfer {
    snapshot: unsafe extern "C" fn(i64, i32, *mut u64, u64) -> i64,
    copy: unsafe extern "C" fn(i64, i32, *const *mut u8, *const u64, u64) -> i32,
}
impl Transfer {
    pub(super) unsafe fn load(handle: *mut c_void) -> Option<Self> {
        // SAFETY: optional image-local typed snapshot/copy ABI, resolved before setup.
        unsafe {
            Some(Self {
                snapshot: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(i64, i32, *mut u64, u64) -> i64,
                >(
                    symbol(handle, c"gerbil_scheme_rust_numeric_batch_snapshot").ok()?
                ),
                copy: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(i64, i32, *const *mut u8, *const u64, u64) -> i32,
                >(
                    symbol(handle, c"gerbil_scheme_rust_numeric_batch_copy").ok()?
                ),
            })
        }
    }
}
/// A private snapshot of numeric row identities. It is owner-local and cannot
/// escape through a Send ticket. Child vectors remain live across GC; no Scheme
/// body pointer is exposed. Copying does not freeze application-owned contents.
#[derive(Debug)]
pub struct NativeNumericSnapshot<'vm, T: NumericElement> {
    root: NativeRoot<'vm>,
    sizes: Vec<u64>,
    _element: PhantomData<T>,
}
fn output_bytes<T>(sizes: &[u64], maximum: usize) -> Result<(), InstanceError> {
    let bytes = sizes.iter().try_fold(0_usize, |sum, &size| {
        usize::try_from(size)
            .ok()
            .and_then(|size| size.checked_mul(std::mem::size_of::<T>()))
            .and_then(|bytes| sum.checked_add(bytes))
    });
    if bytes.is_none_or(|bytes| bytes > maximum) {
        return Err(InstanceError::InvalidConfig("native batch output bound"));
    }
    Ok(())
}
impl<'vm> NativeSession<'vm> {
    /// Pin one private vector of numeric children and read all row sizes in one
    /// native entry. Rust allocation/copy may follow after other owner-local GC.
    /// # Errors
    /// Foreign root, count outside 1..=256, shape/type, output quota or missing ABI.
    pub fn numeric_batch_snapshot<T: NumericElement>(
        &self,
        value: &NativeRoot<'_>,
        count: usize,
    ) -> Result<NativeNumericSnapshot<'vm, T>, InstanceError> {
        self.local(value)?;
        if !(1..=256).contains(&count) {
            return Err(InstanceError::InvalidConfig(
                "native numeric snapshot count",
            ));
        }
        let api = self.image.native.numeric_batch_transfer.as_ref().ok_or(
            InstanceError::InvalidConfig("native batch transfer unavailable"),
        )?;
        let mut sizes = vec![0; count];
        // SAFETY: live owner root and complete writable metadata. Native code
        // copies child identities before querying sizes and transfers one root.
        let root = self.own(unsafe {
            (api.snapshot)(value.root, T::KIND, sizes.as_mut_ptr(), count as u64)
        })?;
        output_bytes::<T>(&sizes, self.maximum_output)?;
        Ok(NativeNumericSnapshot {
            root,
            sizes,
            _element: PhantomData,
        })
    }
}
impl<T: NumericElement> NativeNumericSnapshot<'_, T> {
    /// Copy the pinned rows through one native entry. The original mutable parent
    /// is no longer consulted. Shape/kind/sizes are revalidated before any copy.
    /// # Errors
    /// Wrong owner, quota/allocation failure or native shape/copy error.
    pub fn copy(self, session: &NativeSession<'_>) -> Result<Vec<Vec<T>>, InstanceError> {
        session.local(&self.root)?;
        output_bytes::<T>(&self.sizes, session.maximum_output)?;
        let api = session.image.native.numeric_batch_transfer.as_ref().ok_or(
            InstanceError::InvalidConfig("native batch transfer unavailable"),
        )?;
        let mut rows = self
            .sizes
            .iter()
            .map(|&size| {
                let size = usize::try_from(size)
                    .map_err(|_| InstanceError::InvalidConfig("native batch output bound"))?;
                let mut row = Vec::new();
                row.try_reserve_exact(size)
                    .map_err(|e| InstanceError::Loader(e.to_string()))?;
                row.resize(size, T::default());
                Ok(row)
            })
            .collect::<Result<Vec<_>, InstanceError>>()?;
        let pointers: Vec<*mut u8> = rows.iter_mut().map(|row| row.as_mut_ptr().cast()).collect();
        // SAFETY: private strong snapshot pins every row; disjoint initialized
        // Rust buffers match the checked sizes/kind, valid through synchronous copy.
        let code = unsafe {
            (api.copy)(
                self.root.root,
                T::KIND,
                pointers.as_ptr(),
                self.sizes.as_ptr(),
                self.sizes.len() as u64,
            )
        };
        if code != 0 {
            return Err(InstanceError::Native {
                operation: "native numeric batch copy",
                code: i64::from(code),
            });
        }
        Ok(rows)
    }
}
