// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! VM-independent owned state, shared by generated functions and host adapters.
use std::{
    error::Error,
    fmt,
    sync::{Arc, RwLock},
};

/// A closed state cell and a poisoned lock are distinct lifecycle failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateError {
    Closed,
    Poisoned,
}
impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Closed => "owned state closed",
            Self::Poisoned => "owned state lock poisoned",
        })
    }
}
impl Error for StateError {}

/// One independently allocated state cell. Clones share the cell, not a VM root.
#[derive(Debug)]
pub struct OwnedState<T>(Arc<RwLock<Option<T>>>);
impl<T> Clone for OwnedState<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> OwnedState<T> {
    #[must_use]
    pub fn new(value: T) -> Self {
        Self(Arc::new(RwLock::new(Some(value))))
    }
    /// Replace atomically. A closed or poisoned cell is never revived.
    /// # Errors
    /// Rejects closed/poisoned state without changing the stored value.
    pub fn replace(&self, value: T) -> Result<T, StateError> {
        let mut state = self.0.write().map_err(|_| StateError::Poisoned)?;
        let current = state.as_mut().ok_or(StateError::Closed)?;
        Ok(std::mem::replace(current, value))
    }
    /// # Errors
    /// Reports poisoned locks instead of treating them as a closed cell.
    pub fn close(&self) -> Result<(), StateError> {
        self.0.write().map_err(|_| StateError::Poisoned)?.take();
        Ok(())
    }
    /// # Errors
    /// Reports poisoned locks distinctly from closed state.
    pub fn is_closed(&self) -> Result<bool, StateError> {
        Ok(self.0.read().map_err(|_| StateError::Poisoned)?.is_none())
    }
}
impl<T: Clone> OwnedState<T> {
    /// Copy one complete snapshot while retaining owner-controlled state.
    /// # Errors
    /// Rejects closed or poisoned cells; no borrowed pointer escapes.
    pub fn snapshot(&self) -> Result<T, StateError> {
        self.0
            .read()
            .map_err(|_| StateError::Poisoned)?
            .clone()
            .ok_or(StateError::Closed)
    }
}
