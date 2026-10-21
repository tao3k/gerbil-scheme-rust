//! Experimental Darwin instances. Scheme executes; Rust schedules native typed jobs.
//! Explicit shutdown reports native cleanup and joins. Pending owners stay live.
#[cfg(feature = "tokio-dispatch")]
mod async_dispatch;
#[cfg(feature = "tokio-dispatch")]
pub use async_dispatch::AsyncInstanceTicket;
#[cfg(feature = "tokio-dispatch")]
mod capacity;
#[cfg(feature = "tokio-dispatch")]
pub use capacity::{BatchAdmission, OwnerCapacitySnapshot};
mod batch;
mod batch_trace;
mod batch_types;
pub use batch_types::{BatchTimeline, ProfiledBatch};
mod dispatch;
mod image;
mod mapping;
mod mapping_modules;
pub use mapping::{ImageCodeLease, ImageMappingReceipt, ImageMappingState};
mod native;
mod native_api;
mod native_dispatch;
mod object_api;
mod objects;
pub use native::{NativeRoot, NativeSession};
#[cfg(feature = "tokio-dispatch")]
mod batch_stream;
#[cfg(feature = "tokio-dispatch")]
mod batch_stream_runner;
mod owner_loop;
mod owner_thread;
mod pool;
#[cfg(feature = "tokio-dispatch")]
pub use batch_stream::{BatchStreamRow, NumericBatchStreamTicket, ProfiledBatchStreamTicket};
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

#[cfg(feature = "tokio-dispatch")]
mod service;
#[cfg(feature = "tokio-dispatch")]
mod service_loop;
mod service_wake;
#[cfg(feature = "tokio-dispatch")]
pub use service::{NumericServiceBatch, SchemeServiceLane, SchemeServiceRequest};

#[cfg(feature = "tokio-dispatch")]
mod routing;
#[cfg(feature = "tokio-dispatch")]
pub use routing::{RoutedSchemeTicket, SchemeRouteLoad, SchemeRoutingConfig, SchemeServiceRouter};
