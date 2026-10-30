//! Actual per-result retirement and oneshot publication, including abandoned roots.
use super::{GATE, batch, idle};
use gerbil_scheme::instances::{
    InstanceArrival, InstanceDispatcher, InstanceError, NativeRoot, NativeSession,
    NumericServiceBatch, RetainedValue, SchemeCompletionPublication, SchemeCompletionTrace,
    SchemeServiceRequest,
};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
struct Traced {
    batch: NumericServiceBatch<i64>,
    trace: Arc<SchemeCompletionTrace>,
    retained: Arc<Mutex<Option<RetainedValue>>>,
    fail: bool,
}
impl SchemeServiceRequest for Traced {
    type Output = RetainedValue;
    fn completion_trace(&self) -> Option<Arc<SchemeCompletionTrace>> {
        Some(Arc::clone(&self.trace))
    }
    fn work_units(&self) -> usize {
        self.batch.work_units()
    }
    fn construct<'vm>(&mut self, s: &NativeSession<'vm>) -> Result<NativeRoot<'vm>, InstanceError> {
        self.batch.construct(s)
    }
    fn project(
        self,
        s: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        assert_eq!(self.batch.project(s, output)?.len(), 8);
        let value = s.numeric_vector(&[99_i64])?;
        let token = s.retain(value)?;
        *self.retained.lock().unwrap() = Some(token);
        if self.fail {
            Err(InstanceError::InvalidConfig(
                "injected publication projection error",
            ))
        } else {
            Ok(token)
        }
    }
}
fn request(fail: bool) -> Traced {
    Traced {
        batch: batch(false),
        trace: Arc::new(SchemeCompletionTrace::default()),
        retained: Arc::new(Mutex::new(None)),
        fail,
    }
}
fn owner_barrier(dispatch: &InstanceDispatcher) {
    dispatch
        .submit_native_on(0, |_| Ok(()))
        .unwrap()
        .wait(GATE)
        .unwrap();
}
async fn successful(dispatch: &InstanceDispatcher, after: bool) {
    let job = request(false);
    let trace = Arc::clone(&job.trace);
    let issue = InstanceArrival::new(Instant::now(), None)
        .unwrap()
        .issue()
        .unwrap();
    let reply = dispatch
        .submit_scheme_service_backpressured_on(0, job, GATE)
        .await
        .unwrap()
        .wait_observed(issue, GATE)
        .await
        .unwrap();
    let token = reply.result.unwrap();
    idle(dispatch).await;
    owner_barrier(dispatch);
    let observation = trace.snapshot();
    let (native_ready, retired, send_start, send_end) = (
        observation.native_ready.unwrap(),
        observation.retired.unwrap(),
        observation.send_start.unwrap(),
        observation.send_end.unwrap(),
    );
    assert!(native_ready <= send_start && send_start <= send_end);
    assert_eq!(observation.delivered, Some(true));
    if after {
        assert!(native_ready <= retired && retired <= send_start);
    } else {
        assert!(send_end <= retired);
    }
    let progress = reply.progress.unwrap();
    assert!(
        progress.response_poll_started <= progress.response_poll_ready
            && progress.response_poll_ready <= progress.client_observed
    );
    // Application-retained values survive native credit return until explicitly released.
    dispatch
        .submit_native_on(0, move |s| {
            let value = s.retained(token)?;
            assert_eq!(s.copy_numeric::<i64>(&value)?, [99]);
            drop(value);
            s.release_retained(token)
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
}
async fn invalid_observation_keeps_value(dispatch: &InstanceDispatcher) {
    let job = request(false);
    let ticket = dispatch
        .submit_scheme_service_backpressured_on(0, job, GATE)
        .await
        .unwrap();
    idle(dispatch).await;
    owner_barrier(dispatch);
    let issue = InstanceArrival::new(Instant::now(), None)
        .unwrap()
        .issue()
        .unwrap();
    let observed = ticket.wait_observed(issue, GATE).await.unwrap();
    assert!(observed.progress.is_err());
    let token = observed.result.unwrap();
    dispatch
        .submit_native_on(0, move |s| s.release_retained(token))
        .unwrap()
        .wait(GATE)
        .unwrap();
}
async fn abandoned_or_failed(dispatch: &InstanceDispatcher, fail: bool) {
    let job = request(fail);
    let retained = Arc::clone(&job.retained);
    let trace = Arc::clone(&job.trace);
    let ticket = dispatch
        .submit_scheme_service_backpressured_on(0, job, GATE)
        .await
        .unwrap();
    if fail {
        assert!(ticket.wait_completed(GATE).await.is_err());
    } else {
        drop(ticket);
    }
    idle(dispatch).await;
    owner_barrier(dispatch);
    let token = retained.lock().unwrap().unwrap();
    assert!(
        dispatch
            .submit_native_on(0, move |s| Ok(s.retained(token).is_err()))
            .unwrap()
            .wait(GATE)
            .unwrap()
    );
    let t = trace.snapshot();
    assert!(t.retired.is_some() && t.send_end.is_some());
    assert_eq!(t.delivered, Some(fail));
}
pub(super) async fn contract(
    dispatch: &InstanceDispatcher,
    progress: &std::sync::mpsc::Sender<&'static str>,
) {
    idle(dispatch).await;
    for policy in [
        SchemeCompletionPublication::Immediate,
        SchemeCompletionPublication::AfterRetirement,
    ] {
        assert_eq!(
            dispatch
                .configure_scheme_completion_publication_on(0, policy)
                .unwrap(),
            policy
        );
        successful(
            dispatch,
            policy == SchemeCompletionPublication::AfterRetirement,
        )
        .await;
        abandoned_or_failed(dispatch, false).await;
        abandoned_or_failed(dispatch, true).await;
    }
    invalid_observation_keeps_value(dispatch).await;
    stream_drain(dispatch).await;
    dispatch
        .configure_scheme_completion_publication_on(0, SchemeCompletionPublication::Immediate)
        .unwrap();
    idle(dispatch).await;
    progress.send("per-result publication ordering, retained application values, abandoned/projector-error rollback and receiver poll intervals passed").unwrap();
}
async fn stream_drain(dispatch: &InstanceDispatcher) {
    use gerbil_scheme::instances::SchemeServiceStreamConfig;
    let config = SchemeServiceStreamConfig {
        owner: 0,
        maximum_active: 1,
        maximum_staged_work: 16,
        timeout: GATE,
    };
    let mut stream = dispatch
        .submit_scheme_service_stream_on(vec![batch(false), batch(false)], config)
        .await
        .unwrap();
    while let Some(result) = stream.next(GATE).await {
        assert_eq!(result.unwrap().0.len(), 8);
    }
    idle(dispatch).await;
    super::pool_stream::producer_free(dispatch, &[0]).await;
    let stream = dispatch
        .submit_scheme_service_stream_on(vec![batch(false), batch(false)], config)
        .await
        .unwrap();
    stream.cancel();
    drop(stream);
    idle(dispatch).await;
    super::pool_stream::producer_free(dispatch, &[0]).await;
    owner_barrier(dispatch);
}
