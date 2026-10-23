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
        self.retain_root(value, false)
    }
    fn retain_root(
        &self,
        value: NativeRoot<'vm>,
        service: bool,
    ) -> Result<RetainedValue, InstanceError> {
        self.local(&value)?;
        let mut roots = self.image.retained.borrow_mut();
        let occupied = self.image.service_root.get().is_some();
        if (service && occupied) || (!service && roots.len() - usize::from(occupied) >= 65536) {
            return Err(InstanceError::InvalidConfig("retained root bound"));
        }
        let id = self.image.next_retained.get();
        self.image.next_retained.set(
            id.checked_add(1)
                .ok_or(InstanceError::InvalidConfig("retained identity exhausted"))?,
        );
        roots.insert(id, value.root);
        if service {
            self.image.service_root.set(Some(id));
        }
        std::mem::forget(value);
        Ok(RetainedValue {
            image: self.image.identity,
            id,
            owner: self.image.receipt.owner,
        })
    }
    fn retained_id(&self, value: RetainedValue) -> Result<i64, InstanceError> {
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
        self.retain_root(value, true)
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
