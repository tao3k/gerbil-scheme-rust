//! Actual native ready-owner delivery, stable indices and independent retirement.
use super::{config, idle, producer_free};
use crate::{GATE, VectorRequest, stream::Independent};
use gerbil_scheme::instances::{
    InstanceDispatcher, InstanceError, NativeRoot, NativeSession, SchemeServiceRequest,
    SchemeServiceSupplyAdmission,
};
use std::{sync::mpsc, time::Duration};
struct Held {
    message: Independent,
    gate: Option<(mpsc::Sender<usize>, mpsc::Receiver<()>)>,
}
impl SchemeServiceRequest for Held {
    type Output = Vec<i64>;
    fn work_units(&self) -> usize {
        self.message.work_units()
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.message.construct(session)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        root: &NativeRoot<'_>,
    ) -> Result<Vec<i64>, InstanceError> {
        let value = self.message.project(session, root)?;
        if let Some((entered, release)) = self.gate {
            entered.send(session.owner_index()).unwrap();
            release.recv_timeout(GATE).unwrap();
        }
        Ok(value)
    }
}
pub(super) async fn contract(dispatch: &InstanceDispatcher) {
    let mut driver =
        gerbil_scheme::instances::InstanceClientProgressDriver::new(std::future::ready(()));
    let failed = dispatch
        .prepare_scheme_service_pool_ready_stream(
            vec![Err::<Held, _>(InstanceError::Closed)].into_iter(),
            config(vec![0, 1], GATE),
            SchemeServiceSupplyAdmission::ProgressFair,
            &mut driver,
        )
        .await;
    assert!(matches!(failed, Err(InstanceError::Closed)));
    idle(dispatch).await;
    producer_free(dispatch, &[0, 1]).await;
    bypass(dispatch).await;
    errors(dispatch).await;
    cancellation(dispatch).await;
}
async fn bypass(dispatch: &InstanceDispatcher) {
    let (entered, ready) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let messages = vec![
        Held {
            message: Independent(VectorRequest {
                input: vec![0, 1],
                panic: false,
            }),
            gate: Some((entered, wait)),
        },
        Held {
            message: Independent(VectorRequest {
                input: vec![0, 2],
                panic: false,
            }),
            gate: None,
        },
        Held {
            message: Independent(VectorRequest {
                input: vec![0, 3],
                panic: false,
            }),
            gate: None,
        },
    ];
    let mut cfg = config(vec![0, 1], GATE);
    cfg.maximum_active_per_owner = 1;
    let mut driver =
        gerbil_scheme::instances::InstanceClientProgressDriver::new(std::future::ready(()));
    let mut stream = dispatch
        .prepare_scheme_service_pool_ready_stream(
            messages.into_iter().map(Ok),
            cfg,
            SchemeServiceSupplyAdmission::ProgressFair,
            &mut driver,
        )
        .await
        .unwrap();
    let held_owner = ready.recv_timeout(GATE).unwrap();
    let other = 1 - held_owner;
    producer_free(dispatch, &[other]).await;
    assert_eq!(
        dispatch.owner_capacity()[other].available,
        4,
        "ready output must not hold native credit until client consumption"
    );
    let mut seen = Vec::new();
    for _ in 0..2 {
        let row = stream.next(GATE).await.unwrap().unwrap();
        assert_ne!(
            row.sequence, 0,
            "another owner must publish without waiting for the first input"
        );
        assert_ne!(row.owner, Some(held_owner));
        assert!(row.result.is_ok());
        seen.push(row.sequence);
    }
    assert!(matches!(
        stream.next(Duration::from_millis(1)).await,
        Some(Err(InstanceError::Timeout))
    ));
    assert_eq!(
        dispatch.owner_capacity()[held_owner].available,
        3,
        "client timeout cannot release a native graph"
    );
    release.send(()).unwrap();
    let row = stream.next(GATE).await.unwrap().unwrap();
    assert_eq!(row.sequence, 0);
    assert_eq!(row.owner, Some(held_owner));
    assert_eq!(row.result.unwrap(), vec![10]);
    seen.push(0);
    seen.sort_unstable();
    assert_eq!(seen, vec![0, 1, 2]);
    assert!(stream.next(GATE).await.is_none());
    idle(dispatch).await;
    producer_free(dispatch, &[0, 1]).await;
}
async fn errors(dispatch: &InstanceDispatcher) {
    let messages = [
        (vec![0, 1], false),
        (vec![-1], false),
        (vec![1], false),
        (vec![0, 2], true),
    ]
    .into_iter()
    .map(|(input, panic)| Independent(VectorRequest { input, panic }))
    .collect();
    let mut errors = dispatch
        .submit_scheme_service_pool_ready_stream(
            messages,
            config(vec![0, 1], GATE),
            SchemeServiceSupplyAdmission::ProgressFair,
        )
        .await
        .unwrap();
    let mut rows = (0..4).map(|_| None).collect::<Vec<_>>();
    while let Some(row) = errors.next(GATE).await {
        let row = row.unwrap();
        assert!(rows[row.sequence].is_none());
        rows[row.sequence] = Some(row.result);
    }
    assert_eq!(rows[0].take().unwrap().unwrap(), vec![10]);
    assert_eq!(
        rows[1].take().unwrap().unwrap_err(),
        InstanceError::TaskPanicked
    );
    assert!(matches!(
        rows[2].take().unwrap(),
        Err(InstanceError::Scheme(_))
    ));
    assert_eq!(
        rows[3].take().unwrap().unwrap_err(),
        InstanceError::TaskPanicked
    );
    idle(dispatch).await;
    producer_free(dispatch, &[0, 1]).await;
}
async fn cancellation(dispatch: &InstanceDispatcher) {
    // All inputs remain Rust-only while owners are blocked. Drop must cancel
    // staged work, close result receivers and retire registrations after release.
    let blocked: Vec<_> = (0..2)
        .map(|owner| crate::deferred::block(dispatch, owner))
        .collect();
    let messages = (0..8)
        .map(|_| {
            Independent(VectorRequest {
                input: vec![0, 1],
                panic: false,
            })
        })
        .collect();
    let stream = dispatch
        .submit_scheme_service_pool_ready_stream(
            messages,
            config(vec![0, 1], GATE),
            SchemeServiceSupplyAdmission::Queued,
        )
        .await
        .unwrap();
    stream.cancel();
    drop(stream);
    crate::deferred::unblock(blocked);
    idle(dispatch).await;
    producer_free(dispatch, &[0, 1]).await;
}
