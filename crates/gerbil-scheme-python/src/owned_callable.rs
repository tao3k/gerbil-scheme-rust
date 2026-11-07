// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! A lease over an already compiled callable; no source parsing or evaluation.
use crate::{OwnedState, StateError};
use std::{fmt, sync::Arc};

type Function<A, R> = dyn Fn(A) -> Result<R, StateError> + Send + Sync;
/// Shared host lease over a compiled Rust callable; not a foreign closure evaluator.
pub struct OwnedCallable<A, R>(OwnedState<Arc<Function<A, R>>>);
impl<A, R> Clone for OwnedCallable<A, R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<A, R> fmt::Debug for OwnedCallable<A, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnedCallable").finish_non_exhaustive()
    }
}
impl<A, R> OwnedCallable<A, R> {
    #[must_use]
    pub fn new(function: impl Fn(A) -> Result<R, StateError> + Send + Sync + 'static) -> Self {
        Self(OwnedState::new(Arc::new(function)))
    }
    /// Invoke one admitted function snapshot. Close prevents subsequent admission;
    /// an already admitted invocation retains its function until it returns.
    /// # Errors
    /// Propagates closed/poisoned leases and the compiled function's typed error.
    pub fn call(&self, argument: A) -> Result<R, StateError> {
        let function = self.0.snapshot()?;
        function(argument)
    }
    /// # Errors
    /// Reports poisoned state instead of hiding a teardown failure.
    pub fn close(&self) -> Result<(), StateError> {
        self.0.close()
    }
    /// # Errors
    /// Reports poisoned state distinctly from release.
    pub fn is_closed(&self) -> Result<bool, StateError> {
        self.0.is_closed()
    }
}
