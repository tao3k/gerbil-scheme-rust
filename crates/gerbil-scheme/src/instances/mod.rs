//! Experimental Darwin instances. Scheme executes; Rust schedules native typed jobs.
//! Explicit shutdown reports native cleanup and joins. Pending owners stay live.
#[cfg(feature = "tokio-dispatch")]
mod async_dispatch;
#[cfg(feature = "tokio-dispatch")]
pub use async_dispatch::AsyncInstanceTicket;
#[cfg(feature = "tokio-dispatch")]
mod capacity;
#[cfg(feature = "tokio-dispatch")]
use capacity::Capacity;
#[cfg(feature = "tokio-dispatch")]
mod capacity_event;
#[cfg(feature = "tokio-dispatch")]
mod capacity_try;
#[cfg(feature = "tokio-dispatch")]
pub use capacity::{BatchAdmission, OwnerCapacitySnapshot};
#[cfg(feature = "tokio-dispatch")]
use capacity_try::EntryCredits;
mod batch;
mod batch_trace;
mod batch_types;
pub use batch_types::{BatchTimeline, ProfiledBatch};
mod dispatch;
#[cfg(feature = "tokio-dispatch")]
use dispatch::Admission;
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
mod service_contract;
#[cfg(feature = "tokio-dispatch")]
pub use service_contract::SchemeServiceExecution;
#[cfg(feature = "tokio-dispatch")]
mod service_cycle;
#[cfg(feature = "tokio-dispatch")]
mod service_loop;
mod service_prepare;
mod service_wake;
#[cfg(feature = "tokio-dispatch")]
pub use service::{NumericServiceBatch, SchemeServiceLane, SchemeServiceRequest};

#[cfg(feature = "tokio-dispatch")]
mod routing;
#[cfg(feature = "tokio-dispatch")]
pub use routing::{
    RoutedSchemeTicket, SchemeRouteClass, SchemeRouteLoad, SchemeRoutingAdmission,
    SchemeRoutingConfig, SchemeServiceRouter,
};

#[cfg(feature = "tokio-dispatch")]
mod service_metrics;
#[cfg(feature = "tokio-dispatch")]
mod service_observation;
#[cfg(feature = "tokio-dispatch")]
pub use service_metrics::{SchemePhaseObservation, SchemeServiceObservation, SchemeServicePhase};

#[cfg(feature = "tokio-dispatch")]
mod service_feed;
#[cfg(feature = "tokio-dispatch")]
mod service_progress;
#[cfg(feature = "tokio-dispatch")]
pub use service_progress::SchemeServiceProgress;

#[cfg(feature = "tokio-dispatch")]
mod service_budget;
#[cfg(feature = "tokio-dispatch")]
pub use service_budget::SchemeCooperativeBudgetObservation;

#[cfg(feature = "tokio-dispatch")]
mod service_entry;
#[cfg(feature = "tokio-dispatch")]
pub use service_entry::{
    SchemeServiceEntryObservation, SchemeServiceEntryPhase, SchemeServiceEntryTrace,
};
#[cfg(feature = "tokio-dispatch")]
pub use service_wake::SchemeWakeDelivery;

#[cfg(feature = "tokio-dispatch")]
mod service_history;
#[cfg(feature = "tokio-dispatch")]
pub use service_history::SchemeServiceOwnerSpan;

#[cfg(feature = "tokio-dispatch")]
mod service_port;
#[cfg(feature = "tokio-dispatch")]
pub use service_port::{SchemeCompletionProgress, SchemeServiceCompletion, SchemeServiceWake};

#[cfg(feature = "tokio-dispatch")]
mod service_notify;
#[cfg(feature = "tokio-dispatch")]
pub use service_notify::SchemeServiceNotification;

#[cfg(feature = "tokio-dispatch")]
mod service_stream;
#[cfg(feature = "tokio-dispatch")]
mod service_stream_acquire;
#[cfg(feature = "tokio-dispatch")]
pub use service_stream_acquire::SchemeServiceSupplyAdmission;
#[cfg(feature = "tokio-dispatch")]
mod service_stream_runner;
#[cfg(feature = "tokio-dispatch")]
pub use service_stream::{SchemeServiceStream, SchemeServiceStreamConfig};

#[cfg(feature = "tokio-dispatch")]
mod service_pool_stream;
#[cfg(feature = "tokio-dispatch")]
mod service_progress_fair;
#[cfg(feature = "tokio-dispatch")]
mod service_stream_source;
#[cfg(feature = "tokio-dispatch")]
pub use service_pool_stream::SchemeServicePoolStreamConfig;
#[cfg(feature = "tokio-dispatch")]
pub(super) use service_progress_fair::Fair;
#[cfg(feature = "tokio-dispatch")]
use service_progress_fair::{RouteTarget, StagedWork};
#[cfg(feature = "tokio-dispatch")]
mod service_offer;
#[cfg(feature = "tokio-dispatch")]
mod service_progress_route;
#[cfg(feature = "tokio-dispatch")]
use service_offer::Offer;
#[cfg(feature = "tokio-dispatch")]
mod service_offer_port;
#[cfg(feature = "tokio-dispatch")]
use service_offer_port::OfferPort;
#[cfg(feature = "tokio-dispatch")]
use service_progress_route::assist as route_progress;
#[cfg(feature = "tokio-dispatch")]
use service_stream_source::Shared as PoolSource;

#[cfg(feature = "tokio-dispatch")]
mod service_settings;
#[cfg(feature = "tokio-dispatch")]
pub use service_settings::SchemeServiceSettings;

#[cfg(feature = "tokio-dispatch")]
mod short_pool;
#[cfg(feature = "tokio-dispatch")]
pub use short_pool::{SchemeShortPool, SchemeShortPoolConfig, SchemeShortWake};
