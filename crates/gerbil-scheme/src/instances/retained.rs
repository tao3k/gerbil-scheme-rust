//! Checked opaque tokens; every retained Scheme root stays on its native owner.
use super::{InstanceError, NativeRoot, NativeSession};
use std::ffi::CStr;
/// Copyable identity, not a Scheme pointer or authority to enter another VM.
/// An owner session checks image identity and liveness on every access.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetainedValue {
    image: u64,
    id: u64,
    owner: usize,
}
#[derive(Clone, Copy)]
enum Retention {
    Application,
    #[cfg(feature = "tokio-dispatch")]
    Batch,
    #[cfg(feature = "tokio-dispatch")]
    Entry,
    #[cfg(feature = "tokio-dispatch")]
    Exchange,
}
impl RetainedValue {
    #[must_use]
    pub fn owner(self) -> usize {
        self.owner
    }
}
impl<'vm> NativeSession<'vm> {
    /// Transfer a temporary root into this fixed owner's bounded persistent arena.
    /// # Errors
    /// Rejects foreign roots, exhausted identities and the 65,536-root arena limit.
    pub fn retain(&self, value: NativeRoot<'vm>) -> Result<RetainedValue, InstanceError> {
        self.retain_root(value, Retention::Application)
    }
    fn retain_root(
        &self,
        value: NativeRoot<'vm>,
        kind: Retention,
    ) -> Result<RetainedValue, InstanceError> {
        self.local(&value)?;
        let mut roots = self.image.retained.borrow_mut();
        let internal = usize::from(self.image.service_root.get().is_some())
            + usize::from(self.image.entry_root.get().is_some())
            + usize::from(self.image.exchange_root.get().is_some());
        let slot: Option<&std::cell::Cell<Option<u64>>> = match kind {
            Retention::Application => None,
            #[cfg(feature = "tokio-dispatch")]
            Retention::Batch => Some(&self.image.service_root),
            #[cfg(feature = "tokio-dispatch")]
            Retention::Entry => Some(&self.image.entry_root),
            #[cfg(feature = "tokio-dispatch")]
            Retention::Exchange => Some(&self.image.exchange_root),
        };
        if slot.is_some_and(|slot| slot.get().is_some())
            || (slot.is_none() && roots.len() - internal >= 65536)
        {
            return Err(InstanceError::InvalidConfig("retained root bound"));
        }
        let id = self.image.next_retained.get();
        self.image.next_retained.set(
            id.checked_add(1)
                .ok_or(InstanceError::InvalidConfig("retained identity exhausted"))?,
        );
        roots.insert(id, value.root);
        if let Some(slot) = slot {
            slot.set(Some(id));
        }
        std::mem::forget(value);
        Ok(RetainedValue {
            image: self.image.identity,
            id,
            owner: self.image.receipt.owner,
        })
    }
    pub(super) fn retained_id(&self, value: RetainedValue) -> Result<i64, InstanceError> {
        if value.image != self.image.identity {
            return Err(InstanceError::InvalidConfig("foreign retained owner"));
        }
        self.image
            .retained
            .borrow()
            .get(&value.id)
            .copied()
            .ok_or(InstanceError::Closed)
    }
    /// Borrow a retained value through a fresh temporary native strong root.
    /// # Errors
    /// Rejects stale/foreign tokens and native root-clone failures.
    pub fn retained(&self, value: RetainedValue) -> Result<NativeRoot<'vm>, InstanceError> {
        let root = self.retained_id(value)?;
        // SAFETY: arena retains the input; native collection copying stays on owner.
        let vector = self.own(unsafe { (self.image.native.collection)(&raw const root, 1, 0) })?;
        self.vector_element(&vector, 0)
    }
    /// Release one retained value on its owner; repeated release is an error.
    /// # Errors
    /// Rejects stale/foreign tokens or a native release failure.
    pub fn release_retained(&self, value: RetainedValue) -> Result<(), InstanceError> {
        let root = self.retained_id(value)?;
        self.image.retained.borrow_mut().remove(&value.id);
        if self.image.service_root.get() == Some(value.id) {
            self.image.service_root.set(None);
        }
        if self.image.entry_root.get() == Some(value.id) {
            self.image.entry_root.set(None);
        }
        if self.image.exchange_root.get() == Some(value.id) {
            self.image.exchange_root.set(None);
        }
        // SAFETY: unique arena ownership was removed immediately before release.
        if unsafe { (self.image.release)(root) } != 1 {
            return Err(InstanceError::Native {
                operation: "retained release",
                code: -1,
            });
        }
        Ok(())
    }
    /// Call an additional build-owned rooted export resolved before setup.
    /// # Errors
    /// Rejects undeclared exports, foreign roots and caught Scheme exceptions.
    pub fn call_export(
        &self,
        name: &CStr,
        input: &NativeRoot<'_>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.local(input)?;
        let call = self
            .image
            .exports
            .get(name)
            .ok_or_else(|| InstanceError::Loader(format!("undeclared export {name:?}")))?;
        // SAFETY: IsolatedProgram admits this exact unary rooted export ABI.
        self.call_result(unsafe { call(input.root) })
    }
}

// Owner-local RAII custody; the persistent arena participates in balance checks.
#[cfg(feature = "tokio-dispatch")]
pub(super) struct OwnerRetained<'a> {
    session: NativeSession<'a>,
    value: RetainedValue,
    _owner: std::marker::PhantomData<std::rc::Rc<()>>,
}
#[cfg(feature = "tokio-dispatch")]
impl<'a> OwnerRetained<'a> {
    pub(super) fn new(image: &'a super::image::Image, value: RetainedValue) -> Self {
        Self {
            session: NativeSession {
                image,
                maximum_input: 0,
                maximum_output: 0,
                transferred: std::cell::Cell::new(0),
            },
            value,
            _owner: std::marker::PhantomData,
        }
    }
    pub(super) fn value(&self) -> RetainedValue {
        self.value
    }
}
#[cfg(feature = "tokio-dispatch")]
impl Drop for OwnerRetained<'_> {
    fn drop(&mut self) {
        if self.session.release_retained(self.value).is_err() {
            crate::instance_backend::poison_images();
        }
    }
}
#[cfg(feature = "tokio-dispatch")]
impl<'vm> NativeSession<'vm> {
    pub(super) fn retain_service(
        &self,
        value: NativeRoot<'vm>,
    ) -> Result<RetainedValue, InstanceError> {
        self.retain_root(value, Retention::Batch)
    }
    pub(super) fn retain_entry(&self, value: NativeRoot<'vm>) -> Result<(), InstanceError> {
        self.retain_root(value, Retention::Entry).map(|_| ())
    }
    pub(super) fn retain_exchange(&self, value: NativeRoot<'vm>) -> Result<(), InstanceError> {
        self.retain_root(value, Retention::Exchange).map(|_| ())
    }
    pub(super) fn exchange_id(&self) -> Option<i64> {
        self.image
            .exchange_root
            .get()
            .and_then(|id| self.image.retained.borrow().get(&id).copied())
    }
    pub(super) fn clear_exchange(&self) -> Result<(), InstanceError> {
        if let Some(id) = self.image.exchange_root.get() {
            self.release_retained(RetainedValue {
                image: self.image.identity,
                id,
                owner: self.owner_index(),
            })?;
        }
        Ok(())
    }
    pub(super) fn entry_id(&self) -> Option<i64> {
        self.image
            .entry_root
            .get()
            .and_then(|id| self.image.retained.borrow().get(&id).copied())
    }
    pub(super) fn clear_entry(&self) -> Result<(), InstanceError> {
        if let Some(id) = self.image.entry_root.get() {
            self.release_retained(RetainedValue {
                image: self.image.identity,
                id,
                owner: self.owner_index(),
            })?;
        }
        Ok(())
    }
    pub(super) fn take_result_slot(
        &self,
        value: RetainedValue,
        index: usize,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        let root = self.retained_id(value)?;
        let take = self
            .image
            .native
            .result_slot_take
            .ok_or(InstanceError::InvalidConfig(
                "native result slot ABI unavailable",
            ))?;
        // SAFETY: verified owner arena identity; native code validates slot occupancy,
        // roots the payload first, then clears it. No Scheme value leaves this owner.
        self.own(unsafe { take(root, index as u64) })
    }
}
