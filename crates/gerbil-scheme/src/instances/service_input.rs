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
    /// Construct a service envelope whose payload is `[s64vector tag, numeric data]`
    /// in one owner-local Scheme construction frame. Child values never acquire
    /// temporary external roots. Submission still uses the ordinary Actor endpoint.
    /// All numeric families retain their existing sealed type/copy contract.
    /// # Errors
    /// Missing image capability, cumulative input quota/overflow or native construction.
    pub fn tagged_numeric<'vm, T: NumericElement>(
        &self,
        session: &NativeSession<'vm>,
        tag: &[i64],
        values: &[T],
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        let create = session.image.native.tagged_numeric_service_create.ok_or(
            InstanceError::InvalidConfig("tagged numeric service constructor unavailable"),
        )?;
        // Same charge as separate tag/data vectors, two pair vectors, and header.
        let bytes = std::mem::size_of_val(tag)
            .checked_add(std::mem::size_of_val(values))
            .and_then(|bytes| bytes.checked_add(self.length * std::mem::size_of::<i64>()))
            .and_then(|bytes| bytes.checked_add(4 * std::mem::size_of::<&NativeRoot<'_>>()))
            .ok_or(InstanceError::InvalidConfig(
                "tagged numeric service input overflow",
            ))?;
        session.input(bytes)?;
        // SAFETY: sealed numeric layouts; borrowed slices and header live through
        // synchronous copying on this owner. Only a new parent root is returned.
        session.own(unsafe {
            create(
                T::KIND,
                values.as_ptr().cast(),
                values.len() as u64,
                tag.as_ptr(),
                tag.len() as u64,
                self.header.as_ptr(),
                self.length as u64,
            )
        })
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
    receipt: Receipt<'vm>,
}
#[derive(Debug)]
enum Receipt<'vm> {
    Root(NativeRoot<'vm>),
    Identity(i64),
}
impl SchemeServiceSubmission<'_> {
    pub(super) fn id(
        &self,
        session: &NativeSession<'_>,
    ) -> Result<super::service::JobId, InstanceError> {
        let id = match &self.receipt {
            Receipt::Identity(id) => *id,
            Receipt::Root(root) => {
                let ids = session.copy_numeric::<i64>(root)?;
                if ids.len() != 1 {
                    return Err(InstanceError::InvalidConfig("Scheme service identity"));
                }
                ids[0]
            }
        };
        if id <= 0 {
            return Err(InstanceError::InvalidConfig("Scheme service identity"));
        }
        Ok(super::service::JobId(id))
    }
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
        super::service::call(session, value).map(|receipt| SchemeServiceSubmission {
            receipt: Receipt::Root(receipt),
        })
    }
    /// Copy a tagged numeric payload and submit it in one native call. Only a
    /// scalar job identity returns; no input, envelope or receipt root escapes.
    /// # Errors
    /// Missing capability, input quota, duplicate attempt or Scheme exception.
    pub fn enter_tagged_numeric<'vm, T: NumericElement>(
        &self,
        session: &NativeSession<'vm>,
        tag: &[i64],
        values: &[T],
    ) -> Result<SchemeServiceSubmission<'vm>, InstanceError> {
        let capability = session
            .exchange_id()
            .ok_or(InstanceError::InvalidConfig("native exchange disabled"))?;
        let api = session
            .image
            .native
            .exchange
            .ok_or(InstanceError::InvalidConfig("native exchange unavailable"))?;
        let bytes = std::mem::size_of_val(tag)
            .checked_add(std::mem::size_of_val(values))
            .and_then(|n| n.checked_add(self.length * std::mem::size_of::<i64>()))
            .and_then(|n| n.checked_add(4 * std::mem::size_of::<&NativeRoot<'_>>()))
            .ok_or(InstanceError::InvalidConfig(
                "tagged numeric service input overflow",
            ))?;
        session.input(bytes)?;
        self.begin_entry()?;
        // SAFETY: exclusive owner, sealed numeric layout; all spans are copied
        // synchronously before return. Scheme retains no Rust pointer.
        let id = unsafe {
            (api.enter)(
                capability,
                T::KIND,
                values.as_ptr().cast(),
                values.len() as u64,
                tag.as_ptr(),
                tag.len() as u64,
                self.header.as_ptr(),
                self.length as u64,
            )
        };
        if id <= 0 {
            return Err(session.exchange_failure());
        }
        Ok(SchemeServiceSubmission {
            receipt: Receipt::Identity(id),
        })
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
            .map(|receipt| SchemeServiceSubmission {
                receipt: Receipt::Root(receipt),
            })
    }
}
