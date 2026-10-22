//! Native progress is observable while the task and its resource charge remain live.
use super::{GATE, batch, command, idle};
use gerbil_scheme::instances::{
    InstanceDispatcher, InstanceError, NativeRoot, NativeSession, NumericServiceBatch,
    SchemeServiceRequest,
};
use std::time::Instant;
struct Coarse(NumericServiceBatch<i64>);
impl SchemeServiceRequest for Coarse {
    type Output = Vec<Vec<i64>>;
    fn work_units(&self) -> usize {
        1
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.0.construct(session)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        self.0.project(session, output)
    }
}
pub(super) async fn enable(dispatch: &InstanceDispatcher) {
    idle(dispatch).await;
    for owner in 0..4 {
        dispatch
            .enable_scheme_service_progress_on(owner)
            .unwrap()
            .wait(GATE)
            .unwrap();
        assert!(dispatch.scheme_service_progress_on(owner).unwrap().enabled);
    }
    idle(dispatch).await;
}
pub(super) async fn partial(dispatch: &InstanceDispatcher) {
    assert_eq!(command(dispatch, 0, 16), [2, 1, 0, 2]);
    let coarse = dispatch
        .submit_scheme_service_backpressured_on(0, Coarse(batch(false)), GATE)
        .await
        .unwrap();
    assert_eq!(coarse.wait_completed(GATE).await.unwrap().0.len(), 8);
    idle(dispatch).await;
    command(dispatch, 0, 9);
    let mut request = batch(false);
    request.items[0][1] = 2;
    let ticket = dispatch
        .submit_scheme_service_backpressured_on(0, request, GATE)
        .await
        .unwrap();
    let start = Instant::now();
    loop {
        command(dispatch, 0, 10);
        let p = dispatch.scheme_service_progress_on(0).unwrap();
        if p.bulk_remaining == 2 {
            break;
        }
        assert!(
            start.elapsed() < GATE,
            "native Actor units did not report partial progress: {p:?}"
        );
        tokio::task::yield_now().await;
    }
    let c = &dispatch.owner_capacity()[0];
    assert!(c.available < c.limit);
    assert!(
        c.work_available.unwrap() <= 24,
        "progress released retained work credit"
    );
    assert!(dispatch.enable_scheme_service_progress_on(0).is_err());
    command(dispatch, 0, 8);
    assert_eq!(ticket.wait_completed(GATE).await.unwrap().0.len(), 8);
    idle(dispatch).await;
    assert_eq!(
        dispatch
            .scheme_service_progress_on(0)
            .unwrap()
            .bulk_remaining,
        0
    );
    command(dispatch, 0, 9);
}

pub(super) async fn capabilities(
    dispatch: &InstanceDispatcher,
    progress: &std::sync::mpsc::Sender<&'static str>,
) {
    enable(dispatch).await;
    partial(dispatch).await;
    super::capabilities(dispatch, progress).await;
}
