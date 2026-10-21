//! Explicit native short layout and completion-ready flow acceptance.
use super::*;
use gerbil_scheme_python::{
    owner_dispatch::WindowBudget,
    owner_flow::{FlowControl, owner_flow_ready_backpressured},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
pub(super) struct Short(pub Vec<i64>);
impl SchemeServiceRequest for Short {
    type Output = Vec<Vec<i64>>;
    fn work_units(&self) -> usize {
        1
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        let tag = session.numeric_vector(&[77_i64, 1])?;
        let input = session.numeric_vector(&self.0)?;
        session.vector(&[&tag, &input])
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        Ok(vec![session.copy_numeric(output)?])
    }
}
pub(super) async fn completion_ready(
    dispatch: &InstanceDispatcher,
    progress: &mpsc::Sender<&'static str>,
) {
    let oversized = dispatch
        .submit_scheme_service_backpressured_on(0, Short(corpus::order(500, false)), GATE)
        .await
        .unwrap()
        .wait_completed(GATE)
        .await;
    assert!(
        matches!(oversized, Err(InstanceError::Scheme(_))),
        "short contract must not silently route long work"
    );
    command(dispatch, 0, 9);
    let offered = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&offered);
    let submit = dispatch.clone();
    let mut inputs = vec![batch(true), batch(false), batch(false)];
    for (index, input) in inputs.iter_mut().enumerate() {
        for row in &mut input.items {
            row[0] += i64::try_from(index * 100).unwrap();
        }
    }
    let oracle: Vec<_> = inputs.iter().map(|r| r.items.clone()).collect();
    let flow = tokio::spawn(owner_flow_ready_backpressured(
        inputs.into_iter().map(|r| (0, r)).collect(),
        FlowControl {
            budget: WindowBudget {
                released: Instant::now(),
                timeout: GATE,
            },
            maximum_in_flight: 2,
        },
        move |owner, input| {
            count.fetch_add(1, Ordering::AcqRel);
            let submit = submit.clone();
            async move {
                submit
                    .submit_scheme_service_backpressured_on(owner, input, GATE)
                    .await
            }
        },
    ));
    let start = Instant::now();
    while offered.load(Ordering::Acquire) < 3 {
        assert!(
            start.elapsed() < GATE,
            "ready peer did not refill behind pending head"
        );
        tokio::task::yield_now().await;
    }
    assert!(
        command(dispatch, 0, 10)[0] > 0,
        "head must remain latched during refill"
    );
    command(dispatch, 0, 8);
    for (expected, row) in oracle.iter().zip(flow.await.unwrap()) {
        let (values, _) = row.unwrap();
        for (input, output) in expected.iter().zip(values) {
            corpus::verify(input, &output);
        }
    }
    idle(dispatch).await;
    progress.send("completion-ready peer refills while first job remains latched; output positions preserved").unwrap();
}
