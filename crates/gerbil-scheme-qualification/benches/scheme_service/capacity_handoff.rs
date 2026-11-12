//! Directed zero-work-capacity to short-claim handoff, never a throughput sample.
use super::{Comparison, GATE, Mode, corpus, measurement, settings, stall_forensics, workload};
use gerbil_scheme::instances::{
    InstanceDispatcher, InstanceError, InstancePool, NativeRoot, NativeSession,
    NumericServiceBatch, SchemeServiceInput, SchemeServiceRequest, SchemeServiceSubmission,
};
use std::{future::Future, sync::mpsc, task::Poll};
use tokio::sync::oneshot;
struct HeldProjection {
    message: NumericServiceBatch<i64>,
    entered: oneshot::Sender<()>,
    release: mpsc::Receiver<()>,
}
impl SchemeServiceRequest for HeldProjection {
    type Output = Vec<Vec<i64>>;
    fn work_units(&self) -> usize {
        self.message.work_units()
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.message.construct(session)
    }
    fn enter_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &SchemeServiceInput<'_>,
    ) -> Result<SchemeServiceSubmission<'vm>, InstanceError> {
        self.message.enter_service(session, input)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        root: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        let result = self.message.project(session, root)?;
        let _ = self.entered.send(());
        // No deliberate delay: the controller releases immediately after real queue publication.
        if self.release.recv_timeout(GATE).is_err() {
            eprintln!("CAPACITY-FIXTURE release=missing native-reproduction=false");
            return Err(InstanceError::Timeout);
        }
        Ok(result)
    }
}
fn verify(start: usize, rows: &[Vec<i64>]) {
    for (offset, row) in rows.iter().enumerate() {
        corpus::verify(&corpus::order(start + offset, true), row);
    }
}
fn short_input(id: usize) -> Vec<i64> {
    let mut input = corpus::order(id, false);
    input.truncate(14);
    input
}
fn verify_short(id: usize, rows: &[Vec<i64>]) {
    assert_eq!(rows.len(), 1);
    corpus::verify(&short_input(id), &rows[0]);
}
async fn occupy(dispatch: InstanceDispatcher, owner: usize, request: HeldProjection) {
    let (rows, _) = dispatch
        .submit_scheme_service_backpressured_on(owner, request, GATE)
        .await
        .unwrap()
        .wait_completed(GATE)
        .await
        .unwrap();
    assert_eq!(rows.len(), 32);
    verify(owner * 32, &rows);
}
async fn handoff(dispatch: InstanceDispatcher) {
    let mut owners = Vec::new();
    let mut signals = Vec::new();
    let mut releases = Vec::new();
    stall_forensics::phase(22);
    for owner in 0..4 {
        let (entered, at_projection) = oneshot::channel();
        let (release, wait) = mpsc::channel();
        let request = HeldProjection {
            message: NumericServiceBatch {
                items: (owner * 32..owner * 32 + 32)
                    .map(|id| corpus::order(id, true))
                    .collect(),
                yield_every: 1,
            },
            entered,
            release: wait,
        };
        owners.push(tokio::spawn(occupy(dispatch.clone(), owner, request)));
        signals.push(at_projection);
        releases.push(release);
    }
    for signal in signals {
        signal.await.unwrap();
    }
    assert!(
        dispatch
            .owner_capacity()
            .iter()
            .all(|c| c.work_available == Some(0))
    );
    stall_forensics::phase(23);
    let short = dispatch.scheme_short_pool().unwrap();
    let mut tickets = Vec::new();
    for id in 128..132 {
        let request = workload::orders(
            vec![short_input(id)],
            workload::ShortMode::Dedicated(Mode::Off),
            None,
        );
        tickets.push(short.submit_progress(request, GATE).await.unwrap());
    }
    // Poll a real claim/final observer while no owner can acquire short work.
    let first = tickets.remove(0);
    let mut waiting = Box::pin(first.wait_completed(GATE));
    std::future::poll_fn(|cx| {
        assert!(
            waiting.as_mut().poll(cx).is_pending(),
            "zero work capacity must defer claim"
        );
        Poll::Ready(())
    })
    .await;
    for release in releases {
        release.send(()).unwrap();
    }
    stall_forensics::phase(24);
    // No post-release submit/control call can supply an accidental extra owner wake.
    let (reply, _) = waiting.await.unwrap();
    verify_short(128, &reply.values);
    for (id, ticket) in (129..132).zip(tickets) {
        let (reply, _) = ticket.wait_completed(GATE).await.unwrap();
        verify_short(id, &reply.values);
    }
    stall_forensics::phase(25);
    for owner in owners {
        owner.await.unwrap();
    }
}
pub(super) fn run(pool: &InstancePool, runtime: &tokio::runtime::Runtime) {
    measurement::idle(pool);
    settings::configure(pool, Comparison::ProgressFair, false);
    measurement::idle(pool);
    settings::short_wake(pool, Comparison::ProgressFair, false);
    measurement::idle(pool);
    runtime
        .block_on(runtime.spawn(handoff(pool.dispatcher())))
        .unwrap();
    measurement::idle(pool);
    eprintln!(
        "CAPACITY-HANDOFF owners=4 queued-short=4 result=validated historical-cause-closed=false"
    );
}
