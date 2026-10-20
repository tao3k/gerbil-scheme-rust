//! Experimental Darwin instances. Scheme executes; Rust schedules native typed jobs.
//! Explicit shutdown reports native cleanup and joins. Pending owners stay live.
#[cfg(feature = "tokio-dispatch")]
mod async_dispatch;
#[cfg(feature = "tokio-dispatch")]
pub use async_dispatch::AsyncInstanceTicket;
mod batch;
mod batch_trace;
mod batch_types;
pub use batch_types::{BatchTimeline, ProfiledBatch};
mod dispatch;
mod image;
mod native;
mod native_api;
mod native_dispatch;
mod object_api;
mod objects;
pub use native::{NativeRoot, NativeSession};
mod pool;
mod preflight;
mod retained;
pub use retained::RetainedValue;
mod retirement;
mod types;
pub use dispatch::InstanceDispatcher;
pub use pool::InstancePool;
use pool::Request;
pub use types::{
    InstanceConfig, InstanceError, InstanceReply, InstanceTicket, IsolatedProgram, OwnerReceipt,
    OwnerShutdown, ShutdownReceipt,
};
