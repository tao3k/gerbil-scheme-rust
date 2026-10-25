//! Available native registration is distinct from Tokio cooperation deferral.
use super::InstanceError;
use std::{sync::Arc, time::Instant};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};
pub(super) async fn acquire(
    semaphore: &Arc<Semaphore>,
    deadline: Instant,
) -> Result<OwnedSemaphorePermit, InstanceError> {
    if Instant::now() >= deadline {
        return Err(InstanceError::Timeout);
    }
    let permit = match Arc::clone(semaphore).try_acquire_owned() {
        Ok(permit) => permit,
        Err(TryAcquireError::Closed) => return Err(InstanceError::Closed),
        Err(TryAcquireError::NoPermits) => tokio::time::timeout_at(
            tokio::time::Instant::from_std(deadline),
            Arc::clone(semaphore).acquire_owned(),
        )
        .await
        .map_err(|_| InstanceError::Timeout)?
        .map_err(|_| InstanceError::Closed)?,
    };
    if Instant::now() >= deadline {
        return Err(InstanceError::Timeout);
    }
    Ok(permit)
}
#[cfg(test)]
#[path = "../../tests/unit/producer_registration.rs"]
mod tests;
