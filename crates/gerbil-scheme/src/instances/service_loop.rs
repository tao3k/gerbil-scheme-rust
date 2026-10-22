//! The owner enters Scheme's blocking event wait, never a Rust polling timer.
use super::{
    InstanceError, Request,
    image::Image,
    pool::Work,
    service::{self, Job, JobId},
    service_wake::{Reader, Wake},
};
use std::{
    collections::BTreeMap,
    os::fd::AsRawFd,
    sync::{atomic::Ordering, mpsc},
    time::{Duration, Instant},
};
fn dequeue(requests: &mpsc::Receiver<Request>, wake: &Wake) -> Result<Request, mpsc::TryRecvError> {
    let request = requests.try_recv()?;
    if let Work::Service(job) = &request.work {
        if let Some(trace) = job.entry_trace() {
            trace.mark(super::SchemeServiceEntryPhase::Dequeued);
            if let Some(monitor) = wake.observation.get() {
                trace.owner(monitor);
            }
        }
    }
    Ok(request)
}
struct Pending {
    job: Box<dyn Job>,
    _permit: Option<super::capacity::CapacityPermit>,
}
fn control(image: &Image, header: &[i64]) -> Result<Vec<i64>, InstanceError> {
    let mut result = None;
    // Protocol metadata is bounded independently of application payload bytes.
    super::native_dispatch::run_native(
        image,
        1024,
        65536 * 8,
        |session| {
            let value = service::command(session, header, None)?;
            session.copy_numeric::<i64>(&value)
        },
        |value| {
            result = Some(value.0);
            true
        },
    );
    result.unwrap()
}
fn observed<T>(
    image: &Image,
    wake: &Wake,
    phase: super::SchemeServicePhase,
    action: impl FnOnce() -> T,
) -> T {
    let Some(monitor) = wake.observation.get() else {
        return action();
    };
    monitor.measure(
        phase,
        || {
            let read = image.native.diagnostics?;
            let mut values = [0.0; 6];
            // SAFETY: audited native counters on this active fixed owner, no Scheme entry.
            (unsafe { read(values.as_mut_ptr()) } == 0).then_some(values)
        },
        action,
    )
}
pub(super) fn open(
    image: &Image,
    reader: &Reader,
    wake: &Wake,
    capacity: usize,
) -> Result<(), InstanceError> {
    let count = i64::try_from(capacity)
        .map_err(|_| InstanceError::InvalidConfig("service capacity overflow"))?;
    if count > 65536 {
        return Err(InstanceError::InvalidConfig("service queue bound"));
    }
    let initialized = control(
        image,
        &[
            0,
            i64::from(reader.as_raw_fd()),
            i64::from(wake.write_fd),
            count,
        ],
    )?;
    if initialized.len() != 4
        || initialized[0] != 0
        || !matches!(initialized[1], 1..=5)
        || !(1..=64).contains(&initialized[2])
        || initialized[3] != count
    {
        return Err(InstanceError::InvalidConfig(
            "Scheme service ABI/version receipt",
        ));
    }
    wake.service_version
        .store(usize::try_from(initialized[1]).unwrap(), Ordering::Release);
    wake.lanes
        .store(usize::try_from(initialized[2]).unwrap(), Ordering::Release);
    Ok(())
}
pub(super) fn close(image: &Image) {
    let prepared = control(image, &[7]);
    image
        .service_status
        .set(Some(if prepared.is_ok() && control(image, &[5]).is_ok() {
            0
        } else {
            -73
        }));
}
struct Pump<'a> {
    image: &'a Image,
    input: usize,
    output: usize,
    jobs: BTreeMap<JobId, Pending>,
    producer: Option<Box<dyn super::batch_stream_runner::Producer>>,
    wake: &'a Wake,
}
impl Pump<'_> {
    fn receive(&mut self, request: Request) -> Result<(), InstanceError> {
        if let Work::Service(job) = &request.work {
            if let Some(trace) = job.entry_trace() {
                trace.mark(super::SchemeServiceEntryPhase::OwnerTurn);
            }
        }
        request.started.store(true, Ordering::Release);
        match request.work {
            Work::Service(mut job) => match observed(
                self.image,
                self.wake,
                super::SchemeServicePhase::Submit,
                || job.start(self.image, self.input, self.output),
            ) {
                Ok(id) => {
                    if self.jobs.contains_key(&id) {
                        job.fail(InstanceError::InvalidConfig(
                            "duplicate Scheme service identity",
                        ));
                        return Err(InstanceError::InvalidConfig(
                            "duplicate Scheme service identity",
                        ));
                    }
                    self.jobs.insert(
                        id,
                        Pending {
                            job,
                            _permit: request.capacity,
                        },
                    );
                }
                Err(error) => {
                    let entered = job.may_have_entered();
                    job.fail(error.clone());
                    if entered {
                        return Err(error);
                    }
                }
            },
            Work::Producer(mut next) => {
                if self.producer.is_some() {
                    next.reject(InstanceError::QueueFull);
                } else {
                    self.producer = Some(next);
                }
            }
            Work::Wake => {}
            work => {
                super::owner_loop::execute(self.image, Request { work, ..request }, self.output);
            }
        }
        Ok(())
    }
    fn collect(&mut self) -> Result<(), InstanceError> {
        if self.wake.direct.load(Ordering::Acquire) && self.jobs.is_empty() {
            return Ok(());
        }

        for raw in observed(
            self.image,
            self.wake,
            super::SchemeServicePhase::Advance,
            || control(self.image, &[3]),
        )? {
            let id = JobId(raw);
            let pending = self.jobs.remove(&id).ok_or(InstanceError::InvalidConfig(
                "unknown Scheme service completion",
            ))?;
            observed(
                self.image,
                self.wake,
                super::SchemeServicePhase::Retire,
                || {
                    pending
                        .job
                        .complete(self.image, id, self.input, self.output);
                },
            );
            // The retained work/slot permit drops only after projection/root balance.
        }
        Ok(())
    }
    fn feed(
        &mut self,
        first: Request,
        requests: &mpsc::Receiver<Request>,
        maximum: usize,
    ) -> Result<(), InstanceError> {
        let mut request = first;
        for index in 0..maximum {
            self.receive(request)?;
            if index + 1 == maximum {
                return self.collect();
            }
            match dequeue(requests, self.wake) {
                Ok(next) if matches!(&next.work, Work::Service(_)) => request = next,
                Ok(next) => {
                    self.collect()?;
                    return self.receive(next);
                }
                Err(_) => return self.collect(),
            }
        }
        unreachable!("positive validated feed bound")
    }
    fn drive(
        &mut self,
        requests: &mpsc::Receiver<Request>,
        wake: &Wake,
    ) -> Result<(), InstanceError> {
        use super::batch_stream_runner::ProducerStep;
        let mut draining = false;
        loop {
            let incoming = dequeue(requests, self.wake).ok();
            let feed_limit = wake.feed_limit.load(Ordering::Acquire);
            if feed_limit > 1
                && incoming
                    .as_ref()
                    .is_some_and(|request| matches!(&request.work, Work::Service(_)))
            {
                self.feed(incoming.unwrap(), requests, feed_limit)?;
                continue;
            }
            // Publish an already-admitted short payload before entering Scheme's
            // next green-thread turn. Do not leave it in Rust while yielding.
            // The FIFO inbox order is unchanged; Scheme still chooses execution.
            if incoming.as_ref().is_some_and(
                |request| matches!(&request.work, Work::Service(job) if job.short_service()),
            ) {
                self.receive(incoming.unwrap())?;
                self.collect()?;
                continue;
            }
            self.collect()?;
            if let Some(request) = incoming.or_else(|| dequeue(requests, self.wake).ok()) {
                self.receive(request)?;
                continue;
            }
            let mut waiting = false;
            if let Some(producer) = self.producer.as_mut() {
                match observed(
                    self.image,
                    self.wake,
                    super::SchemeServicePhase::Supply,
                    || producer.step(self.image),
                ) {
                    ProducerStep::Service(request) => {
                        self.receive(request)?;
                        continue;
                    }
                    ProducerStep::Progress => continue,
                    ProducerStep::Finished => {
                        self.producer = None;
                        continue;
                    }
                    ProducerStep::Waiting => waiting = true,
                }
            }
            // Clear/coalesce, then recheck. An enqueue before clear is visible in
            // the inbox; one after recheck publishes a fresh native pipe byte.
            wake.arm();
            match dequeue(requests, self.wake) {
                Ok(request) => {
                    self.receive(request)?;
                    continue;
                }
                Err(mpsc::TryRecvError::Disconnected)
                    if self.jobs.is_empty() && self.producer.is_none() =>
                {
                    return Ok(());
                }
                Err(_) => {}
            }
            if wake.closed.load(Ordering::Acquire) {
                if !draining {
                    control(self.image, &[7])?;
                    draining = true;
                }
                if self.jobs.is_empty() && self.producer.is_none() {
                    return Ok(());
                }
            }
            if wake.failed.load(Ordering::Acquire) {
                return Err(InstanceError::Loader(
                    "Scheme service wake write failed".into(),
                ));
            }
            let remaining = if waiting {
                self.producer
                    .as_ref()
                    .and_then(|producer| producer.deadline())
                    .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            } else {
                None
            };
            if remaining.is_some_and(|value| value.is_zero()) {
                continue;
            }
            let micros = wait_timeout(remaining);
            observed(
                self.image,
                self.wake,
                super::SchemeServicePhase::Wait,
                || control(self.image, &[4, micros]),
            )?;
        }
    }
}
pub(super) fn run(
    image: &Image,
    requests: &mpsc::Receiver<Request>,
    maximum_input: usize,
    maximum_output: usize,
    wake: &Wake,
) {
    let mut pump = Pump {
        image,
        input: maximum_input,
        output: maximum_output,
        jobs: BTreeMap::new(),
        producer: None,
        wake,
    };
    // Application input bounds are also enforced by each admitted dispatcher.
    let result = pump.drive(requests, wake);
    wake.close();
    // Close joins every accepted Scheme child and clears results before any
    // SDK cleanup. Failure marks non-quiescence, keeping the heap/code quarantined.
    close(image);
    if let Err(error) = result {
        for (_, pending) in pump.jobs {
            pending.job.fail(error.clone());
        }
        if let Some(mut producer) = pump.producer {
            producer.reject(error.clone());
        }
        while let Ok(request) = dequeue(requests, wake) {
            if let Work::Service(job) = request.work {
                job.fail(error.clone());
            }
        }
    }
}

// Zero is the Scheme ABI's infinite-wait sentinel, never a finite deadline.
fn wait_timeout(remaining: Option<Duration>) -> i64 {
    remaining.map_or(0, |value| {
        i64::try_from(value.as_nanos().div_ceil(1000))
            .unwrap_or(i64::MAX)
            .max(1)
    })
}
#[cfg(test)]
#[path = "../../tests/unit/service_wait.rs"]
mod tests;
