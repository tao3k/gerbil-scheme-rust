//! Typed service envelope construction; no payload or native session can escape.
use super::{InstanceError, NativeRoot, NativeSession};
use crate::NumericElement;
/// Owner-provided service metadata. Applications may combine native payload and
/// envelope construction; the default request path preserves arbitrary Scheme types.
#[derive(Debug)]
pub struct SchemeServiceInput {
    header: [i64; 4],
    length: usize,
    fused: bool,
}
impl SchemeServiceInput {
    pub(super) fn new(header: [i64; 4], length: usize, fused: bool) -> Self {
        Self {
            header,
            length,
            fused,
        }
    }
    /// Wrap an arbitrary existing native payload using this request's metadata.
    /// # Errors
    /// Native input quota, ownership or construction failure.
    pub fn wrap<'vm>(
        &self,
        session: &NativeSession<'vm>,
        value: &NativeRoot<'_>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        super::service::envelope(session, &self.header[..self.length], Some(value))
    }
    /// Construct a native numeric Actor batch and its service envelope together
    /// when the selected policy supports it. No Rust pointer is retained by Scheme.
    /// # Errors
    /// Item/quantum bounds, cumulative input quota or native construction failure.
    pub fn numeric_batch<'vm, T: NumericElement>(
        &self,
        session: &NativeSession<'vm>,
        items: &[Vec<T>],
        quantum: usize,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        if !self.fused {
            let value = session.numeric_batch_envelope(items, quantum)?;
            return self.wrap(session, &value);
        }
        super::batch::validate(items, quantum, session.maximum_input)?;
        let bytes = super::batch::input_size(items)
            .and_then(|bytes| bytes.checked_add((self.length + 2) * 8))
            .ok_or(InstanceError::InvalidConfig(
                "native service input overflow",
            ))?;
        session.input(bytes)?;
        let create =
            session
                .image
                .native
                .numeric_service_create
                .ok_or(InstanceError::InvalidConfig(
                    "native service constructor unavailable",
                ))?;
        let pointers: Vec<*const u8> = items.iter().map(|item| item.as_ptr().cast()).collect();
        let lengths: Vec<u64> = items.iter().map(|item| item.len() as u64).collect();
        // SAFETY: sealed numeric layouts; validated spans and header stay live
        // through the synchronous native copy. Only the resulting parent is rooted.
        session.own(unsafe {
            create(
                T::KIND,
                pointers.as_ptr(),
                lengths.as_ptr(),
                items.len() as u64,
                quantum as u64,
                self.header.as_ptr(),
                self.length as u64,
            )
        })
    }
}
