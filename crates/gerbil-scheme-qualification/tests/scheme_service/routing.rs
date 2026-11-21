//! Native class routing, quota, cancellation and close acceptance.
use super::*;
use gerbil_scheme::instances::{SchemeRoutingConfig, SchemeServiceRouter};
fn routing_config() -> SchemeRoutingConfig {
    SchemeRoutingConfig {
        owners: vec![0, 1],
        short_slots: 1,
        batch_slots: 3,
        short_max_work: 1,
        batch_max_work: 8,
    }
}
fn long(width: usize) -> NumericServiceBatch<i64> {
    let mut request = batch(true);
    request.items.truncate(width);
    request
}
async fn seed(
    router: &SchemeServiceRouter,
    width: usize,
    owner: usize,
) -> (
    Vec<Vec<i64>>,
    gerbil_scheme::instances::AsyncInstanceTicket<Vec<Vec<i64>>>,
) {
    let request = long(width);
    let oracle = request.items.clone();
    let selected = router.submit_batch(request, GATE).await.unwrap();
    assert_eq!(selected.owner, owner, "batch work/capacity routing");
    (oracle, selected.ticket)
}
fn held(router: &SchemeServiceRouter) -> usize {
    router.loads().iter().map(|l| l.batch_requests).sum()
}
async fn queued_close(router: &SchemeServiceRouter) {
    let before = router.loads();
    assert!(matches!(
        router.submit_batch(long(1), Duration::from_millis(2)).await,
        Err(InstanceError::Timeout)
    ));
    assert_eq!(
        router.loads(),
        before,
        "timed-out class reservation leaked charge"
    );
    {
        let mut future = Box::pin(router.submit_batch(long(1), GATE));
        assert!(
            tokio::time::timeout(Duration::from_millis(2), future.as_mut())
                .await
                .is_err(),
            "class must remain full"
        );
        assert_eq!(held(router), 7);
    }
    assert_eq!(
        router.loads(),
        before,
        "cancelled class reservation leaked charge"
    );
    let submit = router.clone();
    let waiter = tokio::spawn(async move { submit.submit_batch(long(1), GATE).await });
    let started = Instant::now();
    while held(router) != 7 {
        assert!(started.elapsed() < GATE);
        tokio::task::yield_now().await;
    }
    router.close();
    assert!(matches!(waiter.await.unwrap(), Err(InstanceError::Closed)));
    assert_eq!(
        router.loads(),
        before,
        "close must fail queued class work without releasing accepted work"
    );
    assert!(matches!(
        router
            .submit_short(short::Short(vec![0, 0, 1, 1, 1]), GATE)
            .await,
        Err(InstanceError::Closed)
    ));
}
pub(super) async fn contract(dispatch: &InstanceDispatcher, progress: &mpsc::Sender<&'static str>) {
    let mut invalid = routing_config();
    invalid.owners = vec![0, 0];
    assert!(SchemeServiceRouter::new(dispatch.clone(), invalid).is_err());
    let mut invalid = routing_config();
    invalid.short_slots = 2;
    assert!(SchemeServiceRouter::new(dispatch.clone(), invalid).is_err());
    let mut invalid = routing_config();
    invalid.batch_max_work = 32;
    assert!(SchemeServiceRouter::new(dispatch.clone(), invalid).is_err());
    let router = SchemeServiceRouter::new(dispatch.clone(), routing_config()).unwrap();
    assert!(
        matches!(
            router
                .submit_short(
                    VectorRequest {
                        input: vec![0, 1],
                        panic: false
                    },
                    GATE
                )
                .await,
            Err(InstanceError::InvalidConfig(_))
        ),
        "stateful lanes cannot be rerouted"
    );
    command(dispatch, 0, 9);
    command(dispatch, 1, 9);
    let mut tickets = vec![
        seed(&router, 8, 0).await,
        seed(&router, 1, 1).await,
        seed(&router, 1, 1).await,
    ];
    let mut input = corpus::order(700, false);
    input.truncate(14);
    let selected = router
        .submit_short(short::Short(input.clone()), GATE)
        .await
        .unwrap();
    assert_eq!(
        selected.owner, 0,
        "short route minimizes request queue, not batch work score"
    );
    corpus::verify(
        &input,
        &selected.ticket.wait_completed(GATE).await.unwrap().0[0],
    );
    assert!(
        command(dispatch, 0, 10)[0] > 0,
        "long work must remain latched"
    );
    tickets.push(seed(&router, 1, 1).await);
    tickets.push(seed(&router, 8, 0).await);
    tickets.push(seed(&router, 8, 0).await);
    assert_eq!(
        router
            .loads()
            .iter()
            .map(|l| l.batch_work)
            .collect::<Vec<_>>(),
        [24, 3]
    );
    progress
        .send("distinct class routing under latched native work")
        .unwrap();
    let selected = router
        .submit_short(short::Short(input.clone()), GATE)
        .await
        .unwrap();
    assert_eq!(selected.owner, 1);
    drop(selected.ticket); // Accepted native work/charge survives an abandoned receiver.
    let started = Instant::now();
    while router.loads().iter().any(|l| l.short_requests != 0) {
        assert!(started.elapsed() < GATE);
        tokio::task::yield_now().await;
    }
    queued_close(&router).await;
    command(dispatch, 0, 8);
    command(dispatch, 1, 8);
    for (expected, ticket) in tickets {
        let (output, _) = ticket.wait_completed(GATE).await.unwrap();
        for (input, output) in expected.iter().zip(output) {
            corpus::verify(input, &output);
        }
    }
    idle(dispatch).await;
    assert!(
        router
            .loads()
            .iter()
            .all(|l| l.short_requests == 0 && l.batch_requests == 0 && l.batch_work == 0)
    );
    progress
        .send("class routing timeout/cancel/abandon/close drains; no charge remains")
        .unwrap();
}
