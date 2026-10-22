//! Native producer lifecycle and entry boundaries; no timing-speed assertions.
use super::{GATE, VectorRequest, batch, command, idle, reselection, short};
use gerbil_scheme::instances::{
    InstanceDispatcher, InstanceError, NativeRoot, NativeSession, SchemeServiceRequest,
    SchemeServiceStream, SchemeServiceStreamConfig,
};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
fn config(timeout: Duration) -> SchemeServiceStreamConfig {
    SchemeServiceStreamConfig {
        owner: 0,
        maximum_active: 2,
        maximum_staged_work: 32,
        timeout,
    }
}
struct Independent(VectorRequest);
impl SchemeServiceRequest for Independent {
    type Output = Vec<i64>;
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
async fn drained(dispatch: &InstanceDispatcher) {
    idle(dispatch).await;
    let start = Instant::now();
    while dispatch
        .owner_capacity()
        .iter()
        .any(|c| c.producer_available != 1)
    {
        assert!(
            start.elapsed() < GATE,
            "producer registration did not retire"
        );
        tokio::task::yield_now().await;
    }
}
async fn blocked(dispatch: &InstanceDispatcher) {
    let start = Instant::now();
    while command(dispatch, 0, 10)[0] < 8 {
        assert!(
            start.elapsed() < GATE,
            "two accepted Scheme batches did not suspend"
        );
        tokio::task::yield_now().await;
    }
    let capacity = &dispatch.owner_capacity()[0];
    assert_eq!(
        (
            capacity.available,
            capacity.work_available,
            capacity.producer_available
        ),
        (2, Some(16), 0)
    );
}
pub(super) async fn contract(
    dispatch: &InstanceDispatcher,
    progress: &mpsc::Sender<&'static str>,
) -> SchemeServiceStream<Vec<Vec<i64>>> {
    recovery(dispatch, progress).await;
    let finished = cancellation(dispatch, progress).await;
    expiry(dispatch, progress).await;
    abandonment(dispatch, progress).await;
    finished
}
async fn recovery(dispatch: &InstanceDispatcher, progress: &mpsc::Sender<&'static str>) {
    drained(dispatch).await;
    assert!(matches!(
        dispatch
            .submit_scheme_service_stream_on(
                vec![VectorRequest {
                    input: vec![0, 1],
                    panic: false
                }],
                config(GATE)
            )
            .await,
        Err(InstanceError::InvalidConfig(_))
    ));
    let mut small = config(GATE);
    small.maximum_staged_work = 7;
    assert!(matches!(
        dispatch
            .submit_scheme_service_stream_on(vec![batch(false)], small)
            .await,
        Err(InstanceError::InvalidConfig(_))
    ));
    let mut zero = config(GATE);
    zero.maximum_active = 0;
    assert!(matches!(
        dispatch
            .submit_scheme_service_stream_on(vec![batch(false)], zero)
            .await,
        Err(InstanceError::InvalidConfig(_))
    ));
    let messages = [
        (vec![0, 1], false),
        (vec![-1], false),
        (vec![1], false),
        (vec![0, 1], true),
        (vec![0, 2], false),
    ]
    .into_iter()
    .map(|(input, panic)| Independent(VectorRequest { input, panic }))
    .collect();
    let mut stream = dispatch
        .submit_scheme_service_stream_on(messages, config(GATE))
        .await
        .unwrap();
    assert_eq!(stream.next(GATE).await.unwrap().unwrap().0, [10]);
    assert_eq!(
        stream.next(GATE).await.unwrap().unwrap_err(),
        InstanceError::TaskPanicked
    );
    assert!(matches!(
        stream.next(GATE).await.unwrap(),
        Err(InstanceError::Scheme(_))
    ));
    assert_eq!(
        stream.next(GATE).await.unwrap().unwrap_err(),
        InstanceError::TaskPanicked
    );
    assert_eq!(stream.next(GATE).await.unwrap().unwrap().0, [13]);
    assert!(stream.next(GATE).await.is_none());
    drained(dispatch).await;
    progress.send("owner resident producer: native GC, constructor/projection panic, Scheme exception and independent successor recovered").unwrap();
}
async fn cancellation(
    dispatch: &InstanceDispatcher,
    progress: &mpsc::Sender<&'static str>,
) -> SchemeServiceStream<Vec<Vec<i64>>> {
    command(dispatch, 0, 9);
    let mut stream = dispatch
        .submit_scheme_service_stream_on((0..4).map(|_| batch(true)).collect(), config(GATE))
        .await
        .unwrap();
    blocked(dispatch).await;
    let short = dispatch
        .submit_scheme_service_backpressured_on(0, short::Short(vec![0, 0, 1, 1, 1]), GATE)
        .await
        .unwrap();
    assert_eq!(short.wait_completed(GATE).await.unwrap().0.len(), 1);
    stream.cancel();
    let mut follower =
        Box::pin(dispatch.submit_scheme_service_stream_on(vec![batch(false)], config(GATE)));
    reselection::registered(follower.as_mut()).await;
    drop(follower); // cancelling registration cannot steal/drop the active producer
    assert_eq!(dispatch.owner_capacity()[0].producer_available, 0);
    command(dispatch, 0, 8);
    for _ in 0..2 {
        assert_eq!(stream.next(GATE).await.unwrap().unwrap().0.len(), 8);
    }
    for _ in 0..2 {
        assert_eq!(
            stream.next(GATE).await.unwrap().unwrap_err(),
            InstanceError::Cancelled
        );
    }
    drained(dispatch).await;
    progress.send("owner resident producer: active bound/short bypass, cancelled registration, cancel staged only and accepted drain").unwrap();

    stream
}
async fn expiry(dispatch: &InstanceDispatcher, progress: &mpsc::Sender<&'static str>) {
    dispatch.observe_scheme_service_on(0).unwrap();
    command(dispatch, 0, 9);
    let mut timed = dispatch
        .submit_scheme_service_stream_on(
            (0..4).map(|_| batch(true)).collect(),
            config(Duration::from_millis(500)),
        )
        .await
        .unwrap();
    blocked(dispatch).await;
    assert_eq!(
        timed
            .next(Duration::from_millis(1))
            .await
            .unwrap()
            .unwrap_err(),
        InstanceError::Timeout
    );
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(
        dispatch.owner_capacity()[0].producer_available,
        0,
        "expiry must retain registration through accepted drain"
    );
    let before_idle = dispatch
        .scheme_service_observation_on(0)
        .unwrap()
        .wait(GATE)
        .unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    let after_idle = dispatch
        .scheme_service_observation_on(0)
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert!(
        after_idle.phases[0].calls - before_idle.phases[0].calls < 16,
        "an expired staged deadline must not busy-poll accepted Scheme work"
    );
    command(dispatch, 0, 8);
    assert_eq!(timed.next(GATE).await.unwrap().unwrap().0.len(), 8);
    for _ in 0..2 {
        assert_eq!(
            timed.next(GATE).await.unwrap().unwrap_err(),
            InstanceError::Timeout
        );
    }
    drained(dispatch).await;
    progress.send("owner resident producer: absolute entry deadline, delivery timeout and expired-producer drain").unwrap();
}
async fn abandonment(dispatch: &InstanceDispatcher, progress: &mpsc::Sender<&'static str>) {
    command(dispatch, 0, 9);
    let abandoned = dispatch
        .submit_scheme_service_stream_on((0..4).map(|_| batch(true)).collect(), config(GATE))
        .await
        .unwrap();
    blocked(dispatch).await;
    drop(abandoned);
    command(dispatch, 0, 8);
    drained(dispatch).await;
    progress.send("owner resident producer: abandoned stream drains accepted graphs and releases registration").unwrap();
}

pub(super) async fn partial_wait(
    dispatch: &InstanceDispatcher,
) -> Vec<gerbil_scheme::instances::AsyncInstanceTicket<Vec<Vec<i64>>>> {
    command(dispatch, 2, 9);
    let mut tickets = Vec::new();
    for index in 0..4 {
        let mut message = batch(true);
        if index == 3 {
            message.items.truncate(1);
        }
        tickets.push(
            dispatch
                .submit_scheme_service_backpressured_on(2, message, GATE)
                .await
                .unwrap(),
        );
    }
    let mut settings = config(GATE);
    settings.owner = 2;
    let mut staged = dispatch
        .submit_scheme_service_stream_on(vec![batch(false)], settings)
        .await
        .unwrap();
    let start = Instant::now();
    while dispatch.owner_capacity()[2].work_available != Some(0) {
        assert!(
            start.elapsed() < GATE,
            "producer did not acquire the seven partial native work credits"
        );
        tokio::task::yield_now().await;
    }
    staged.cancel();
    assert_eq!(
        staged.next(GATE).await.unwrap().unwrap_err(),
        InstanceError::Cancelled
    );
    while dispatch.owner_capacity()[2].producer_available != 1 {
        assert!(start.elapsed() < GATE);
        tokio::task::yield_now().await;
    }
    assert_eq!(
        dispatch.owner_capacity()[2].work_available,
        Some(7),
        "cancel must refund Tokio's partial work acquisition before the entered work drains"
    );
    tickets
}
