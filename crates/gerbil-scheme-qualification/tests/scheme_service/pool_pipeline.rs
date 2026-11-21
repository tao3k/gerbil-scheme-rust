//! Native witnesses for preparation/supply overlap and partial-prefix cleanup.
use super::{Tagged, config, messages, producer_free};
use crate::{GATE, batch, command, idle};
use gerbil_scheme::instances::{
    InstanceClientProgressDriver, InstanceDispatcher, InstanceError, NativeRoot, NativeSession,
    SchemeServiceRequest, SchemeServiceStreamConfig, SchemeServiceSupplyAdmission,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::Duration,
};
struct Notice {
    message: Tagged,
    entered: Option<mpsc::SyncSender<()>>,
}
impl SchemeServiceRequest for Notice {
    type Output = (usize, usize, usize);
    fn work_units(&self) -> usize {
        self.message.work_units()
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        let root = self.message.construct(session)?;
        if let Some(send) = self.entered.take() {
            let _ = send.send(());
        }
        Ok(root)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        root: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        self.message.project(session, root)
    }
}
fn inputs(count: usize, entered: &Arc<AtomicUsize>, notify: &mpsc::SyncSender<()>) -> Vec<Notice> {
    messages(count, false, entered)
        .into_iter()
        .enumerate()
        .map(|(i, message)| Notice {
            message,
            entered: (i == 0).then(|| notify.clone()),
        })
        .collect()
}
pub(super) async fn contract(dispatch: &InstanceDispatcher) {
    success(dispatch).await;
    partial_registration(dispatch, false).await;
    partial_registration(dispatch, true).await;
    late_error(dispatch, false).await;
    late_error(dispatch, true).await;
}
async fn success(dispatch: &InstanceDispatcher) {
    let entered = Arc::new(AtomicUsize::new(0));
    let (send, first) = mpsc::sync_channel(1);
    let source = inputs(6, &entered, &send)
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            if i == 2 {
                first.recv_timeout(GATE).unwrap();
            }
            Ok(value)
        });
    let mut cfg = config(vec![0, 1], GATE);
    cfg.maximum_active_per_owner = 1;
    let mut driver = InstanceClientProgressDriver::new(std::future::ready(()));
    let mut stream = dispatch
        .pipeline_scheme_service_pool_ready_stream(
            source,
            cfg,
            SchemeServiceSupplyAdmission::ProgressFair,
            &mut driver,
        )
        .await
        .unwrap();
    let mut seen = Vec::new();
    while let Some(row) = stream.next(GATE).await {
        let row = row.unwrap();
        let value = row.result.unwrap();
        assert_eq!(row.sequence, value.0);
        assert_eq!(row.owner, Some(value.1));
        seen.push(row.sequence);
    }
    seen.sort_unstable();
    assert_eq!(seen, (0..6).collect::<Vec<_>>());
    assert_eq!(entered.load(Ordering::SeqCst), 6);
    producer_free(dispatch, &[0, 1]).await;
    idle(dispatch).await;
}
async fn partial_registration(dispatch: &InstanceDispatcher, expire: bool) {
    command(dispatch, 3, 9);
    let held = dispatch
        .submit_scheme_service_stream_on(
            vec![batch(true)],
            SchemeServiceStreamConfig {
                owner: 3,
                maximum_active: 1,
                maximum_staged_work: 8,
                timeout: GATE,
            },
        )
        .await
        .unwrap();
    let entered = Arc::new(AtomicUsize::new(0));
    let (send, first) = mpsc::sync_channel(1);
    let source = inputs(4, &entered, &send)
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            if i == 2 {
                first.recv_timeout(GATE).unwrap();
            }
            Ok(value)
        });
    let mut cfg = config(
        vec![0, 3],
        if expire {
            Duration::from_millis(100)
        } else {
            GATE
        },
    );
    cfg.maximum_active_per_owner = 1;
    let mut driver = InstanceClientProgressDriver::new(std::future::ready(()));
    let mut pending = Box::pin(dispatch.pipeline_scheme_service_pool_ready_stream(
        source,
        cfg,
        SchemeServiceSupplyAdmission::ProgressFair,
        &mut driver,
    ));
    if expire {
        assert!(matches!(
            pending.as_mut().await,
            Err(InstanceError::Timeout)
        ));
    } else {
        super::reselection::registered(pending.as_mut()).await;
    }
    assert!(
        entered.load(Ordering::SeqCst) > 0,
        "whole preparation/registration barrier remains"
    );
    drop(pending);
    producer_free(dispatch, &[0]).await;
    drop(held);
    command(dispatch, 3, 8);
    producer_free(dispatch, &[3]).await;
    idle(dispatch).await;
}
async fn late_error(dispatch: &InstanceDispatcher, work_overflow: bool) {
    let entered = Arc::new(AtomicUsize::new(0));
    let (send, first) = mpsc::sync_channel(1);
    let source = inputs(3, &entered, &send)
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            if i == 1 {
                first.recv_timeout(GATE).unwrap();
                if !work_overflow {
                    return Err(InstanceError::InvalidConfig(
                        "injected late preparation error",
                    ));
                }
            }
            Ok(value)
        });
    let mut cfg = config(vec![0], GATE);
    cfg.maximum_active_per_owner = 1;
    if work_overflow {
        cfg.maximum_staged_work = 1;
    }
    let mut driver = InstanceClientProgressDriver::new(std::future::ready(()));
    assert!(matches!(
        dispatch
            .pipeline_scheme_service_pool_ready_stream(
                source,
                cfg,
                SchemeServiceSupplyAdmission::ProgressFair,
                &mut driver
            )
            .await,
        Err(InstanceError::InvalidConfig(_))
    ));
    assert!(entered.load(Ordering::SeqCst) > 0);
    producer_free(dispatch, &[0]).await;
    idle(dispatch).await;
}
