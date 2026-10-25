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
struct DeferredCredits {
    _native: Option<super::capacity::CapacityPermit>,
    _stream: Vec<tokio::sync::OwnedSemaphorePermit>,
}
struct Pending {
    job: Box<dyn Job>,
    permit: Option<super::capacity::CapacityPermit>,
}
fn control(image: &Image, header: &[i64]) -> Result<Vec<i64>, InstanceError> {
    let mut result = None;
    // Protocol metadata is bounded independently of application payload bytes.
    super::native_dispatch::run_native(
        image,
        1024,
        (65536 + 5) * 8,
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
        || !matches!(initialized[1], 1..=32)
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
// Linear, owner-borrowed custody of handed-out identities, never Scheme values.
// Projection returns each credit before the next advance consumes this token.
struct ReadyToken<'a> {
    image: &'a Image,
    epoch: i64,
    _owner: std::marker::PhantomData<std::rc::Rc<()>>,
}
impl<'a> ReadyToken<'a> {
    fn parse(image: &'a Image, completed: &mut Vec<i64>) -> Result<Option<Self>, InstanceError> {
        if completed.first() != Some(&-8) {
            return Ok(None);
        }
        if completed.len() < 2 || completed[1] < 0 {
            return Err(InstanceError::InvalidConfig(
                "invalid completion epoch receipt",
            ));
        }
        let epoch = completed[1];
        completed.drain(..2);
        Ok(Some(Self {
            image,
            epoch,
            _owner: std::marker::PhantomData,
        }))
    }
    fn advance(self) -> Result<Vec<i64>, InstanceError> {
        control(self.image, &[3, self.epoch])
    }
}
struct Pump<'a> {
    image: &'a Image,
    input: usize,
    output: usize,
    jobs: BTreeMap<JobId, Pending>,
    completed: Option<ReadyToken<'a>>,
    retirement_drain: Vec<DeferredCredits>,
    deferred: Option<Request>,
    prepared: Option<super::service_prepare::Prepared>,
    producer: Option<Box<dyn super::batch_stream_runner::Producer>>,
    wake: &'a Wake,
}
impl Pump<'_> {
    fn receive(&mut self, mut request: Request) -> Result<(), InstanceError> {
        if matches!(&request.work, Work::Control(_))
            && let Some(token) = self.completed.take()
        {
            control(token.image, &[14, token.epoch])?;
        }
        if let Some(port) = self.wake.short_pool.get() {
            port.interrupted();
        }
        if let Work::Service(job) = &request.work {
            if let Some(trace) = job.entry_trace() {
                trace.mark(super::SchemeServiceEntryPhase::OwnerTurn);
            }
        }
        request.started.store(true, Ordering::Release);
        let units = request.work_units;
        match request.work {
            Work::Service(mut job) => match observed(
                self.image,
                self.wake,
                super::SchemeServicePhase::Submit,
                || {
                    job.start(
                        self.image,
                        &service::Start {
                            input: self.input,
                            output: self.output,
                            progress: self.wake.progress.enabled.load(Ordering::Acquire),
                            fused: self.wake.pipeline_cycle.load(Ordering::Acquire),
                        },
                    )
                },
            ) {
                Ok(id) => {
                    self.wake.progress.charge(units, job.short_service());
                    // Move advisory inbox work to native progress only after
                    // submission. Resource credits still span projection.
                    if let Some(permit) = request.capacity.as_mut() {
                        permit.entered();
                    }
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
                            permit: request.capacity,
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
    fn collect(&mut self, requests: &mpsc::Receiver<Request>) -> Result<(), InstanceError> {
        if self.wake.direct.load(Ordering::Acquire)
            && self.jobs.is_empty()
            && self.completed.is_none()
        {
            return Ok(());
        }

        let cycle = self.wake.native_cycle.load(Ordering::Acquire);
        let retire_before_publish = self.wake.retire_before_publish.load(Ordering::Acquire);
        let (mut completed, batch) = observed(
            self.image,
            self.wake,
            super::SchemeServicePhase::Advance,
            || {
                if cycle {
                    super::service_cycle::harvest(
                        self.image,
                        self.completed.take().map(|t| t.epoch),
                    )
                } else {
                    let completed = match self.completed.take() {
                        Some(token) => token.advance(),
                        None => control(self.image, &[3]),
                    }?;
                    Ok((completed, None))
                }
            },
        )?;
        let token = ReadyToken::parse(self.image, &mut completed)?;
        self.wake.progress.receipt(&mut completed)?;
        if let Some(batch) = &batch {
            batch.validate(completed.len())?;
        } else if cycle && !completed.is_empty() {
            return Err(InstanceError::InvalidConfig(
                "native completion missing result slots",
            ));
        }
        let refill = completed.len().min(
            self.producer
                .as_ref()
                .map_or(0, |p| p.retirement_refill_budget()),
        );
        let retired_count = completed.len();
        for (index, raw) in completed.into_iter().enumerate() {
            let id = JobId(raw);
            let pending = self.jobs.remove(&id).ok_or(InstanceError::InvalidConfig(
                "unknown Scheme service completion",
            ))?;
            self.retire_one(
                pending,
                service::Completion {
                    id,
                    ready: batch.as_ref().map(|b| b.slot(index)),
                    retire_before_publish,
                    input: self.input,
                    output: self.output,
                },
            )?;
            if cycle {
                self.cycle_refill(requests)?;
            }
        }
        drop(batch);
        // Only readiness custody is deferred. Every result's native permit has
        // already returned before any later arbitrary projector ran.
        self.completed = token;
        if self.wake.pipeline_cycle.load(Ordering::Acquire) {
            self.pipeline_fill(requests, retired_count)?;
        }
        // Retire the full snapshot before constructing new payloads. Refill is
        // bounded by both released results and the registered active budget.
        for _ in 0..if cycle { 0 } else { refill } {
            if !self.refill_retired()? {
                break;
            }
        }
        Ok(())
    }
    fn next_request(&mut self, requests: &mpsc::Receiver<Request>) -> Option<Request> {
        self.deferred
            .take()
            .or_else(|| dequeue(requests, self.wake).ok())
    }
    fn retire_one(
        &mut self,
        pending: Pending,
        completion: service::Completion,
    ) -> Result<(), InstanceError> {
        let retire_before_publish = completion.retire_before_publish;
        let trace = pending.job.completion_trace();
        let mut permit = Some(pending.permit);
        let mut retire = || {
            observed(
                self.image,
                self.wake,
                super::SchemeServicePhase::Release,
                || drop(permit.take().expect("native credit returned once")),
            );
            if let Some(trace) = &trace {
                trace.retired();
            }
        };
        let retired = observed(
            self.image,
            self.wake,
            super::SchemeServicePhase::Retire,
            || pending.job.complete(self.image, completion, &mut retire),
        );
        if let service::Retirement::Drain(stream) = retired {
            // The native graph is still owned. Close joins/clears it before
            // either native or stream credit can return, even on failed take.
            self.retirement_drain.push(DeferredCredits {
                _native: permit.take().expect("failed slot kept native credit"),
                _stream: stream,
            });
            return Err(InstanceError::InvalidConfig(
                "native result slot transfer failed",
            ));
        }
        // The retained work/slot permit drops only after projection/root balance.
        if !retire_before_publish {
            retire();
        }
        debug_assert!(
            permit.is_none(),
            "result must return exactly one native credit"
        );
        Ok(())
    }
    fn cycle_refill(&mut self, requests: &mpsc::Receiver<Request>) -> Result<(), InstanceError> {
        // One returned result's opportunity, never recursively collect while the
        // native token is owned. Closing owners admit no fresh pool supply.
        if self.wake.closed.load(Ordering::Acquire) {
            return Ok(());
        }
        if self
            .deferred
            .as_ref()
            .is_some_and(|r| matches!(&r.work, Work::Control(_)))
        {
            return Ok(());
        }
        if self.wake.prepared_cycle.load(Ordering::Acquire) {
            return self.prepare_refill(requests);
        }
        let request = self.next_request(requests);
        if let Some(request) = request {
            if matches!(&request.work, Work::Control(_)) {
                self.deferred = Some(request);
            } else {
                self.receive(request)?;
            }
        } else if !self.refill_retired()? {
            self.claim_offer()?;
        }
        Ok(())
    }
    fn prepare_refill(&mut self, requests: &mpsc::Receiver<Request>) -> Result<(), InstanceError> {
        // Ordinary owner callbacks may run; fresh Scheme construction cannot
        // overtake any remaining slot in this harvested snapshot. Preserve FIFO.
        if let Some(request) = self.next_request(requests) {
            if matches!(&request.work, Work::Service(_) | Work::Control(_)) {
                self.deferred = Some(request);
            } else {
                self.receive(request)?;
            }
        }
        if self.prepared.is_none() && self.deferred.is_none() {
            self.prepared = observed(
                self.image,
                self.wake,
                super::SchemeServicePhase::Supply,
                || self.producer.as_mut().and_then(|p| p.prepare()),
            );
        }
        // Offers remain in their existing revocable port: no claim/entry fence,
        // active credit or native work credit is consumed during preparation.
        Ok(())
    }
    fn pipeline_fill(
        &mut self,
        requests: &mpsc::Receiver<Request>,
        retired: usize,
    ) -> Result<(), InstanceError> {
        // Each opportunity comes from an independently retired result. Never
        // harvest recursively, wait for input, or hold more than one prepared input.
        for _ in 0..retired {
            if self.wake.closed.load(Ordering::Acquire) {
                break;
            }
            if let Some(request) = self.next_request(requests) {
                // An admitted service is work, not a reason to abandon all
                // returned opportunities. A wake has no native ownership. Keep
                // both inside this bounded turn; never bypass a FIFO barrier.
                if self.wake.coupled_refill.load(Ordering::Acquire)
                    && matches!(&request.work, Work::Service(_) | Work::Wake)
                {
                    self.receive(request)?;
                    continue;
                }
                self.deferred = Some(request);
                break; // controls/arbitrary callbacks remain FIFO barriers
            }
            if let Some(request) = self
                .wake
                .short_pool
                .get()
                .and_then(super::short_pool::Port::claim)
            {
                if self.wake.coupled_refill.load(Ordering::Acquire) {
                    self.receive(request)?;
                    continue;
                }
                self.deferred = Some(request);
                break;
            }
            if self.prepared.is_none() {
                self.prepared = observed(
                    self.image,
                    self.wake,
                    super::SchemeServicePhase::Supply,
                    || self.producer.as_mut().and_then(|p| p.prepare()),
                );
            }
            if !self.enter_prepared()? && !self.claim_offer()? {
                break;
            }
        }
        Ok(())
    }
    fn enter_prepared(&mut self) -> Result<bool, InstanceError> {
        let Some(prepared) = self.prepared.take() else {
            return Ok(false);
        };
        if let Some(request) = prepared.enter(self.wake) {
            self.receive(request)?;
        }
        Ok(true)
    }
    fn refill_retired(&mut self) -> Result<bool, InstanceError> {
        use super::batch_stream_runner::ProducerStep;
        // No recursive collect or native wait; Scheme owns execution. Never
        // retain returned credit behind a later constructor or projector.
        if let Some(request) = self
            .wake
            .short_pool
            .get()
            .and_then(super::short_pool::Port::claim)
        {
            self.receive(request)?;
            return Ok(true);
        }
        let Some(producer) = self.producer.as_mut() else {
            return Ok(false);
        };
        let step = observed(
            self.image,
            self.wake,
            super::SchemeServicePhase::Supply,
            || producer.step(self.image),
        );
        match step {
            ProducerStep::Service(request) => {
                self.receive(request)?;
                Ok(true)
            }
            ProducerStep::Finished => {
                self.producer = None;
                Ok(false)
            }
            ProducerStep::Progress | ProducerStep::Waiting => Ok(false),
        }
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
                return self.collect(requests);
            }
            match dequeue(requests, self.wake) {
                Ok(next) if matches!(&next.work, Work::Service(_)) => request = next,
                Ok(next) => {
                    self.deferred = Some(next);
                    self.collect(requests)?;
                    if let Some(next) = self.deferred.take() {
                        self.receive(next)?;
                    }
                    return Ok(());
                }
                Err(_) => return self.collect(requests),
            }
        }
        unreachable!("positive validated feed bound")
    }
    fn claim_short(&mut self, requests: &mpsc::Receiver<Request>) -> Result<bool, InstanceError> {
        if self
            .deferred
            .as_ref()
            .is_some_and(|r| matches!(&r.work, Work::Control(_)))
        {
            return Ok(false);
        }
        let request = self
            .wake
            .short_pool
            .get()
            .and_then(super::short_pool::Port::claim);
        if let Some(request) = request {
            self.receive(request)?;
            self.collect(requests)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    fn claim_offer(&mut self) -> Result<bool, InstanceError> {
        let Some(offer) = self.wake.offers.peek() else {
            return Ok(false);
        };
        let Some(request) = offer.claim(self.wake).enter() else {
            return Ok(false);
        };
        let result = self.receive(request);
        // Source fanout follows native construction, and only a price/EOF event.
        offer.wake_source();
        result.map(|()| true)
    }
    fn wait_remaining(&self, waiting: bool) -> Option<Duration> {
        waiting
            .then(|| self.producer.as_ref().and_then(|p| p.deadline()))
            .flatten()
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
    }
    fn wait_native(&self, waiting: bool) -> Result<(), InstanceError> {
        let remaining = self.wait_remaining(waiting);
        if remaining.is_some_and(|value| value.is_zero()) {
            return Ok(());
        }
        let micros = wait_timeout(remaining);
        observed(
            self.image,
            self.wake,
            super::SchemeServicePhase::Wait,
            || control(self.image, &[4, micros]),
        )?;
        Ok(())
    }
    fn shutdown_turn(&self, draining: &mut bool) -> Result<bool, InstanceError> {
        if self.wake.closed.load(Ordering::Acquire) {
            if !*draining {
                control(self.image, &[7])?;
                *draining = true;
            }
            if self.jobs.is_empty() && self.producer.is_none() {
                return Ok(true);
            }
        }
        if self.wake.failed.load(Ordering::Acquire) {
            return Err(InstanceError::Loader(
                "Scheme service wake write failed".into(),
            ));
        }
        Ok(false)
    }
    fn drive(
        &mut self,
        requests: &mpsc::Receiver<Request>,
        wake: &Wake,
    ) -> Result<(), InstanceError> {
        use super::batch_stream_runner::ProducerStep;
        let mut draining = false;
        let mut short_turn = true;
        loop {
            if wake.progress.enabled.load(Ordering::Acquire) {
                wake.progress
                    .waiting_for_claim
                    .store(false, Ordering::Release);
            }
            if short_turn && self.claim_short(requests)? {
                short_turn = false;
                continue;
            }
            // One shared short claim alternates with one ordinary owner turn.
            // Scheme still owns green-thread execution and class fairness.
            short_turn = true;
            let incoming = self.next_request(requests);
            let feed_limit = wake.feed_limit.load(Ordering::Acquire);
            if feed_limit > 1
                && !wake.prepared_cycle.load(Ordering::Acquire)
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
                self.collect(requests)?;
                continue;
            }
            self.deferred = incoming;
            self.collect(requests)?;
            if let Some(request) = self.next_request(requests) {
                self.receive(request)?;
                continue;
            }
            if self.enter_prepared()? {
                continue;
            }
            // A separate bulk slot must not overtake ordinary queued short or
            // lifecycle messages. Shared short claims already had their turn.
            if self.claim_offer()? {
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
            // Reopen notification, then claim durable producer duty. A source
            // publication or returned capacity before this boundary needs a new
            // producer turn even when its pipe notification was coalesced. After
            // this boundary a publisher owes a fresh native pipe notification.
            // No producer callback or native work runs under a transport lock.
            if wake.arm() {
                continue;
            }
            // Application inbox and short/offer ports retain their own rechecks.
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
            // Work conservation: try visible work before entering native wait.
            // Registration then checks publication under the shared queue lock.
            if self.claim_short(requests)? {
                short_turn = false;
                continue;
            }
            if self.claim_offer()? {
                continue;
            }
            register_short_wait(wake);
            if self.shutdown_turn(&mut draining)? {
                return Ok(());
            }
            self.wait_native(waiting)?;
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
        completed: None,
        retirement_drain: Vec::new(),
        deferred: None,
        prepared: None,
        producer: None,
        wake,
    };
    // Application input bounds are also enforced by each admitted dispatcher.
    let result = pump.drive(requests, wake);
    wake.close();
    // Close joins every accepted Scheme child and clears results before any
    // SDK cleanup. Failure marks non-quiescence, keeping the heap/code quarantined.
    close(image);
    if let Some(prepared) = pump.prepared.take() {
        prepared.reject(result.clone().err().unwrap_or(InstanceError::Closed));
    }
    if let Err(error) = result {
        if let Some(Request {
            work: Work::Service(job),
            ..
        }) = pump.deferred.take()
        {
            job.fail(error.clone());
        }
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

fn register_short_wait(wake: &Wake) {
    if wake.progress.enabled.load(Ordering::Acquire) {
        wake.progress
            .waiting_for_claim
            .store(true, Ordering::Release);
    }
    if let Some(port) = wake.short_pool.get() {
        port.arm_wait();
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
