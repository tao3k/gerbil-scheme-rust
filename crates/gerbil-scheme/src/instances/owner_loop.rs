//! Native owner pump. Only complete foreign returns are scheduling boundaries.
use super::{InstanceError, InstanceReply, Request, image::Image, pool::Work};
use std::{
    sync::{atomic::Ordering, mpsc},
    time::Instant,
};

pub(super) fn execute(image: &Image, request: Request, maximum_reply: usize) {
    #[cfg(feature = "tokio-dispatch")]
    let _capacity = request.capacity;
    request.started.store(true, Ordering::Release);
    match request.work {
        Work::Native(job) => job(image),
        Work::Bytes { bytes, response } => {
            let value = if crate::instance_backend::images_ready() {
                image
                    .invoke(&bytes, maximum_reply)
                    .map(|bytes| InstanceReply {
                        owner: image.receipt.owner,
                        bytes,
                    })
            } else {
                Err(InstanceError::Closed)
            };
            let _ = response.send((value, Instant::now()));
        }
        #[cfg(feature = "tokio-dispatch")]
        Work::Service(_) | Work::Producer(_) | Work::Wake => {
            unreachable!("control message routed before execution")
        }
    }
}
#[cfg(not(feature = "tokio-dispatch"))]
pub(super) fn run(image: &Image, requests: &mpsc::Receiver<Request>, maximum_reply: usize) {
    for request in requests {
        execute(image, request, maximum_reply);
    }
}
#[cfg(feature = "tokio-dispatch")]
pub(super) fn run(image: &Image, requests: &mpsc::Receiver<Request>, maximum_reply: usize) {
    use super::batch_stream_runner::{Producer, ProducerStep};
    let mut producer: Option<Box<dyn Producer>> = None;
    let mut received = None;
    loop {
        let message = received.take().map_or_else(|| requests.try_recv(), Ok);
        match message {
            Ok(request) => match request.work {
                Work::Producer(mut next) => {
                    if producer.is_some() {
                        next.reject(InstanceError::QueueFull);
                    } else {
                        producer = Some(next);
                    }
                }
                Work::Service(_) => {
                    unreachable!("service rejected before ordinary owner admission")
                }
                Work::Wake => {}
                work => execute(image, Request { work, ..request }, maximum_reply),
            },
            Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {}
        }
        if let Some(current) = producer.as_mut() {
            match current.step(image) {
                ProducerStep::Progress => continue,
                ProducerStep::Finished => {
                    producer = None;
                    continue;
                }
                ProducerStep::Service(_) => {
                    unreachable!("service producer requires Scheme service")
                }
                ProducerStep::Waiting => {}
            }
            let Some(deadline) = current.deadline() else {
                match requests.recv() {
                    Ok(request) => received = Some(request),
                    Err(_) => break,
                }
                continue;
            };
            let remaining = deadline.saturating_duration_since(Instant::now());
            match requests.recv_timeout(remaining) {
                Ok(request) => received = Some(request),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match requests.recv() {
                Ok(request) => received = Some(request),
                Err(_) => break,
            }
        }
    }
}
#[cfg(feature = "tokio-dispatch")]
pub(super) fn waker(
    sender: mpsc::SyncSender<Request>,
    wake: Option<std::sync::Arc<super::service_wake::Wake>>,
) -> std::task::Waker {
    use std::sync::{Arc, atomic::AtomicBool};
    struct WakeOwner(
        mpsc::SyncSender<Request>,
        Option<Arc<super::service_wake::Wake>>,
    );
    impl std::task::Wake for WakeOwner {
        fn wake(self: Arc<Self>) {
            self.wake_by_ref();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            // A full channel already guarantees that the owner will poll again.
            let _ = self.0.try_send(Request {
                started: Arc::new(AtomicBool::new(false)),
                work: Work::Wake,
                capacity: None,
                work_units: 0,
            });
            if let Some(wake) = &self.1 {
                wake.notify();
            }
        }
    }
    std::task::Waker::from(Arc::new(WakeOwner(sender, wake)))
}
