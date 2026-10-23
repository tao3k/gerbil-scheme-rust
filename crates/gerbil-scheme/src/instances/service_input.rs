//! Typed service envelope construction; no payload or native session can escape.
use super::{InstanceError, NativeRoot, NativeSession};
use crate::NumericElement;
/// Owner-provided service metadata. Applications may combine native payload and
/// envelope construction; the default request path preserves arbitrary Scheme types.
#[derive(Debug)]
pub struct SchemeServiceInput<'entry> {
    header: [i64; 4],
    length: usize,
    fused: bool,
    submitted: &'entry std::cell::Cell<bool>,
}
impl<'entry> SchemeServiceInput<'entry> {
    pub(super) fn new(
        header: [i64; 4],
        length: usize,
        fused: bool,
        submitted: &'entry std::cell::Cell<bool>,
    ) -> Self {
        Self {
            header,
            length,
            fused,
            submitted,
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

/// Opaque owner-local identity receipt. Only a context submission can create it;
/// an application payload root cannot be returned as a completed admission.
#[derive(Debug)]
pub struct SchemeServiceSubmission<'vm> {
    pub(super) receipt: NativeRoot<'vm>,
}
impl SchemeServiceInput<'_> {
    fn begin_entry(&self) -> Result<(), InstanceError> {
        if self.submitted.replace(true) {
            return Err(InstanceError::InvalidConfig(
                "service entry already attempted",
            ));
        }
        Ok(())
    }
    /// Submit an arbitrary constructed envelope using the original Scheme endpoint.
    /// # Errors
    /// Duplicate attempt, foreign owner, admission or caught Scheme exception.
    pub fn submit<'vm>(
        &self,
        session: &NativeSession<'vm>,
        value: &NativeRoot<'_>,
    ) -> Result<SchemeServiceSubmission<'vm>, InstanceError> {
        session.local(value)?;
        self.begin_entry()?;
        super::service::call(session, value).map(|receipt| SchemeServiceSubmission { receipt })
    }
    /// Construct and admit numeric input in Scheme. With the explicit capability,
    /// no input/header/envelope root is published to Rust; only the identity
    /// receipt crosses back. Rows are freshly allocated and copied, never reused.
    /// # Errors
    /// Bounds/quota failure, duplicate entry, native construction or Scheme admission.
    pub fn enter_numeric<'vm, T: NumericElement>(
        &self,
        session: &NativeSession<'vm>,
        items: &[Vec<T>],
        quantum: usize,
    ) -> Result<SchemeServiceSubmission<'vm>, InstanceError> {
        let Some(entry) = session.entry_id().filter(|_| self.fused) else {
            let value = self.numeric_batch(session, items, quantum)?;
            return self.submit(session, &value);
        };
        super::batch::validate(items, quantum, session.maximum_input)?;
        let bytes = super::batch::input_size(items)
            .and_then(|bytes| bytes.checked_add((self.length + 2) * 8))
            .ok_or(InstanceError::InvalidConfig(
                "native service input overflow",
            ))?;
        session.input(bytes)?;
        let call =
            session
                .image
                .native
                .numeric_service_enter
                .ok_or(InstanceError::InvalidConfig(
                    "native service entry unavailable",
                ))?;
        let pointers: Vec<*const u8> = items.iter().map(|item| item.as_ptr().cast()).collect();
        let lengths: Vec<u64> = items.iter().map(|item| item.len() as u64).collect();
        self.begin_entry()?;
        // SAFETY: same owner's retained capability; validated spans/header live
        // throughout the synchronous Scheme copy/admission. No pointer is retained.
        session
            .call_result(unsafe {
                call(
                    T::KIND,
                    pointers.as_ptr(),
                    lengths.as_ptr(),
                    items.len() as u64,
                    quantum as u64,
                    self.header.as_ptr(),
                    self.length as u64,
                    entry,
                )
            })
            .map(|receipt| SchemeServiceSubmission { receipt })
    }
}
