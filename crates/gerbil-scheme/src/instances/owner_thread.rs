//! One native owner's initialization, service loop and verified finalization.
use super::{
    InstanceConfig, InstanceError, IsolatedProgram, OwnerReceipt, OwnerShutdown,
    image::Image,
    mapping::Mapping,
    pool::{LIFECYCLE, Request},
    service_wake::{Reader, Wake},
};
use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
};
pub(super) struct Task {
    pub path: PathBuf,
    pub program: IsolatedProgram,
    pub owner: usize,
    pub mapping: Arc<Mapping>,
    pub config: InstanceConfig,
    pub ready: mpsc::Sender<Result<OwnerReceipt, InstanceError>>,
    pub finished: mpsc::Sender<OwnerShutdown>,
    pub wake_reader: Option<Reader>,
    pub wake: Option<Arc<Wake>>,
    pub requests: mpsc::Receiver<Request>,
}
pub(super) fn run(task: Task) {
    let loaded = {
        let _guard = LIFECYCLE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Image::load(&task.path, &task.program, task.owner, task.mapping)
    };
    let image = match loaded {
        Ok(image) => image,
        Err(error) => {
            let _ = task.ready.send(Err(error));
            return;
        }
    };
    #[cfg(feature = "tokio-dispatch")]
    if let (Some(reader), Some(wake)) = (task.wake_reader.as_ref(), task.wake.as_ref()) {
        if let Err(error) =
            super::service_loop::open(&image, reader, wake, task.config.queue_capacity)
        {
            let _ = task.ready.send(Err(error));
            super::service_loop::close(&image);
            let _guard = LIFECYCLE
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let _ = task.finished.send(image.finish());
            return;
        }
    }
    #[cfg(not(feature = "tokio-dispatch"))]
    let _ = (&task.wake_reader, &task.wake);
    let _ = task.ready.send(Ok(image.receipt.clone()));
    #[cfg(feature = "tokio-dispatch")]
    if let Some(wake) = task.wake.as_ref() {
        super::service_loop::run(
            &image,
            &task.requests,
            task.config.maximum_request_bytes,
            task.config.maximum_reply_bytes,
            wake,
        );
    } else {
        super::owner_loop::run(&image, &task.requests, task.config.maximum_reply_bytes);
    }
    #[cfg(not(feature = "tokio-dispatch"))]
    super::owner_loop::run(&image, &task.requests, task.config.maximum_reply_bytes);
    let receipt = {
        let _guard = LIFECYCLE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        image.finish()
    };
    let _ = task.finished.send(receipt);
}
