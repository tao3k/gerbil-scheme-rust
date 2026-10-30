//! Kernel arrival clock, separate from issue and native admission.
mod clock;
#[cfg(target_os = "macos")]
mod darwin;
pub use clock::{InstanceArrivalClock, InstanceArrivalWake};
