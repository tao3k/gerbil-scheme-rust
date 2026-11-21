//! Owner-local, exactly-once projection from a completed native slot.
use super::{InstanceError, NativeRoot, NativeSession, service::JobId, service_cycle::ReadyResult};
use crate::NumericElement;
use std::cell::Cell;
/// Borrowed completion custody. It cannot outlive the owner turn or cross threads.
/// Generic Scheme values retain ordinary rooted projection and continuation support.
pub struct SchemeServiceOutput<'turn> {
    id: JobId,
    ready: Option<ReadyResult>,
    taken: &'turn Cell<bool>,
    attempted: Cell<bool>,
}
impl<'turn> SchemeServiceOutput<'turn> {
    pub(super) fn new(id: JobId, ready: Option<ReadyResult>, taken: &'turn Cell<bool>) -> Self {
        Self {
            id,
            ready,
            taken,
            attempted: Cell::new(false),
        }
    }
    fn begin(&self) -> Result<(), InstanceError> {
        if self.attempted.replace(true) {
            return Err(InstanceError::InvalidConfig(
                "service result already projected",
            ));
        }
        Ok(())
    }
    fn take<'vm>(&self, session: &NativeSession<'vm>) -> Result<NativeRoot<'vm>, InstanceError> {
        match self.ready {
            Some(ready) => ready.project(session, self.taken),
            None => super::service::command(session, &[2, self.id.0], None),
        }
    }
    /// Transfer a result into an independent temporary root for arbitrary types.
    /// # Errors
    /// Duplicate consumption, native custody failure or caught Scheme exception.
    pub fn rooted<'vm>(
        &self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.begin()?;
        self.take(session)
    }
    /// Copy a bounded numeric result and retire its slot in one native call when
    /// exchange is enabled. Bounds/type errors discard the slot independently;
    /// failed native take retains capacity for the existing drain protocol.
    /// # Errors
    /// Duplicate consumption, numeric family/size, quota or Scheme exception.
    pub fn copy_numeric<T: NumericElement>(
        &self,
        session: &NativeSession<'_>,
        maximum_elements: usize,
    ) -> Result<Vec<T>, InstanceError> {
        self.begin()?;
        let Some(ready) = self
            .ready
            .filter(|r| !r.failed)
            .filter(|_| session.scheme_service_exchange_enabled())
        else {
            let value = self.take(session)?;
            return session.copy_numeric_bounded(
                &value,
                Self::maximum_bytes::<T>(session, maximum_elements)?,
            );
        };
        let mut values = self.allocate::<T>(session, maximum_elements)?;
        let size = self.copy_slot(session, ready, &mut values)?;
        values.truncate(size);
        Ok(values)
    }
    fn allocate<T: NumericElement>(
        &self,
        session: &NativeSession<'_>,
        count: usize,
    ) -> Result<Vec<T>, InstanceError> {
        let prepared = (|| {
            Self::maximum_bytes::<T>(session, count)?;
            let mut values = Vec::new();
            values
                .try_reserve_exact(count)
                .map_err(|error| InstanceError::Loader(error.to_string()))?;
            values.resize(count, T::default());
            Ok(values)
        })();
        if prepared.is_err() {
            drop(self.take(session)?);
        }
        prepared
    }
    fn copy_slot<T: NumericElement>(
        &self,
        session: &NativeSession<'_>,
        ready: ReadyResult,
        values: &mut [T],
    ) -> Result<usize, InstanceError> {
        let count = values.len();
        let api = session
            .image
            .native
            .exchange
            .ok_or(InstanceError::InvalidConfig("native exchange unavailable"))?;
        let root = session.retained_id(ready.batch)?;
        // SAFETY: retained batch belongs to this owner; initialized disjoint Rust
        // storage has the sealed numeric layout. Native checks precede slot clear.
        let size = unsafe {
            (api.copy)(
                root,
                ready.index as u64,
                T::KIND,
                values.as_mut_ptr().cast(),
                count as u64,
            )
        };
        if let Ok(size) = usize::try_from(size) {
            self.taken.set(true);
            if size > count {
                return Err(InstanceError::InvalidConfig(
                    "native numeric exchange length",
                ));
            }
            return Ok(size);
        }
        // Validation did not clear the slot. Take/discard it before returning its
        // permit; an unsuccessful take leaves `taken` false and triggers drain.
        drop(self.take(session)?);
        Err(if size == -2 {
            InstanceError::InvalidConfig("native output bound")
        } else {
            InstanceError::Native {
                operation: "native numeric type",
                code: size,
            }
        })
    }
    fn maximum_bytes<T>(session: &NativeSession<'_>, count: usize) -> Result<usize, InstanceError> {
        count
            .checked_mul(std::mem::size_of::<T>())
            .filter(|n| *n <= session.maximum_output)
            .ok_or(InstanceError::InvalidConfig("native output bound"))
    }
}
impl std::fmt::Debug for SchemeServiceOutput<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemeServiceOutput")
            .field("job", &self.id.0)
            .field("attempted", &self.attempted.get())
            .field("taken", &self.taken.get())
            .finish_non_exhaustive()
    }
}
