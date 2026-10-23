//! Actual result-slot release and admission between projections on one native owner.
use super::*;
use gerbil_scheme::instances::InstanceTicket;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
struct CycleBatch {
    input: NumericServiceBatch<i64>,
    projected: Arc<AtomicUsize>,
    admitted: Arc<AtomicBool>,
    dispatch: InstanceDispatcher,
    ticket: mpsc::Sender<InstanceTicket<()>>,
}
impl SchemeServiceRequest for CycleBatch {
    type Output = Vec<Vec<i64>>;
    fn work_units(&self) -> usize {
        self.input.work_units()
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.input.construct(session)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        value: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        let index = self.projected.fetch_add(1, Ordering::AcqRel);
        assert!(
            (session.diagnostics().unwrap()[4] - 2.0).abs() < f64::EPSILON,
            "one batch arena root and this result only"
        );
        session.collect()?;
        {
            let op = session.numeric_vector(&[20_i64])?;
            let state = session.call_export(c"gerbil_owner_service_observe", &op)?;
            let state = session.copy_numeric::<i64>(&state)?;
            assert_eq!(
                state[3],
                i64::try_from(2 - index).unwrap(),
                "taken slot cannot retain its graph"
            );
        }
        if index == 0 {
            let admitted = Arc::clone(&self.admitted);
            let dispatch = self.dispatch.clone();
            let ticket = self.dispatch.submit_native_on(0, move |session| {
                assert!(
                    (session.diagnostics().unwrap()[4] - 1.0).abs() < f64::EPSILON,
                    "previous result root already released"
                );
                let capacity = dispatch.owner_capacity();
                assert!(
                    capacity[0].available >= 1
                        && capacity[0].work_available.is_some_and(|units| units >= 8),
                    "previous result credit already returned"
                );
                full_application_arena(session)?;
                session.collect()?;
                admitted.store(true, Ordering::Release);
                Ok(())
            })?;
            self.ticket.send(ticket).unwrap();
        } else {
            assert!(
                self.admitted.load(Ordering::Acquire),
                "queued owner work must run before later projections"
            );
            if index == 2 {
                // This setter is queued before the last projection/root drop.
                // It must wait for empty-snapshot acknowledgement, not fail idle checks.
                self.ticket
                    .send(self.dispatch.configure_scheme_service_completion_on(
                        0,
                        gerbil_scheme::instances::SchemeServiceCompletion::NativeCycle,
                    )?)
                    .unwrap();
            }
        }
        self.input.project(session, value)
    }
}
struct Immediate(i64);
impl SchemeServiceRequest for Immediate {
    type Output = ();
    fn work_units(&self) -> usize {
        1
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        session.numeric_vector(&[self.0])
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        value: &NativeRoot<'_>,
    ) -> Result<(), InstanceError> {
        session.collect()?;
        if self.0 == 8 {
            assert_eq!(
                session.copy_scalar(value)?,
                gerbil_scheme::SchemeScalar::Boolean(false)
            );
        } else {
            let expected = session.void_object()?;
            assert_eq!(session.kind(value)?, session.kind(&expected)?);
        }
        Ok(())
    }
}
pub(super) fn contract(runtime: &tokio::runtime::Runtime, progress: &mpsc::Sender<&'static str>) {
    let mut config = config();
    config.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), config).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    dispatch
        .configure_scheme_service_completion_on(
            0,
            gerbil_scheme::instances::SchemeServiceCompletion::NativeCycle,
        )
        .unwrap()
        .wait(GATE)
        .unwrap();
    let projected = Arc::new(AtomicUsize::new(0));
    let admitted = Arc::new(AtomicBool::new(false));
    let (ticket, receive) = mpsc::channel();
    runtime.block_on(async {
        let mut tickets = Vec::new();
        for _ in 0..3 {
            tickets.push(
                dispatch
                    .submit_scheme_service_backpressured_on(
                        0,
                        CycleBatch {
                            input: batch(true),
                            projected: Arc::clone(&projected),
                            admitted: Arc::clone(&admitted),
                            dispatch: dispatch.clone(),
                            ticket: ticket.clone(),
                        },
                        GATE,
                    )
                    .await
                    .unwrap(),
            );
        }
        let start = Instant::now();
        while command(&dispatch, 0, 10)[0] < 3 {
            assert!(start.elapsed() < GATE);
            tokio::task::yield_now().await;
        }
        command(&dispatch, 0, 18);
        for ticket in tickets {
            let (values, _) = ticket.wait_completed(GATE).await.unwrap();
            assert_eq!(values.len(), 8);
            corpus::peers(&values);
        }
        assert_eq!(projected.load(Ordering::Acquire), 3);
        for _ in 0..2 {
            receive.recv_timeout(GATE).unwrap().wait(GATE).unwrap();
        }
        for op in [8, 9] {
            dispatch
                .submit_scheme_service_backpressured_on(0, Immediate(op), GATE)
                .await
                .unwrap()
                .wait_completed(GATE)
                .await
                .unwrap();
        }
        idle(&dispatch).await;
    });
    let actual = dispatch
        .scheme_service_completion_on(0)
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert!(!actual.token_owned && !actual.opportunity_pending);
    assert_eq!((actual.claimed, actual.ready_queued), (0, 0));
    let close = pool.shutdown(GATE);
    assert!(close.is_clean(), "{close:?}");
    assert!(
        close
            .owners
            .iter()
            .all(|o| o.roots_before_cleanup == 0 && o.image_file_removed)
    );
    narrow_error_quota(runtime);
    invalidated_slots(runtime);
    progress.send("native cycle clears each slot/root/credit before next projection; queued admission and GC preserve remaining graphs; false/void and close pass").unwrap();
}

pub(super) fn policy(owner: usize) -> gerbil_scheme::instances::SchemeServiceCompletion {
    if owner == 1 {
        gerbil_scheme::instances::SchemeServiceCompletion::PipelineCycle
    } else {
        gerbil_scheme::instances::SchemeServiceCompletion::CoalescedHandoff
    }
}

fn narrow_error_quota(runtime: &tokio::runtime::Runtime) {
    let mut settings = config();
    settings.owners = 1;
    settings.maximum_reply_bytes = 8;
    let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    dispatch
        .configure_scheme_service_completion_on(
            0,
            gerbil_scheme::instances::SchemeServiceCompletion::NativeCycle,
        )
        .unwrap()
        .wait(GATE)
        .unwrap();
    let error = runtime.block_on(async {
        dispatch
            .submit_scheme_service_backpressured_on(0, Immediate(1), GATE)
            .await
            .unwrap()
            .wait_completed(GATE)
            .await
            .unwrap_err()
    });
    assert!(
        matches!(error, InstanceError::Scheme(_)),
        "diagnostic must survive an eight-byte application reply quota: {error:?}"
    );
    let close = pool.shutdown(GATE);
    assert!(close.is_clean(), "{close:?}");
}

// A service's one internal arena root must not reduce the public 65536-root quota.
fn full_application_arena(session: &NativeSession<'_>) -> Result<(), InstanceError> {
    let mut values = Vec::new();
    for _ in 0..65536 {
        let value = session.scalar(gerbil_scheme::SchemeScalar::Fixnum(1))?;
        values.push(session.retain(value)?);
    }
    assert!((session.diagnostics().unwrap()[4] - 65537.0).abs() < f64::EPSILON);
    let extra = session.scalar(gerbil_scheme::SchemeScalar::Fixnum(1))?;
    assert!(matches!(
        session.retain(extra),
        Err(InstanceError::InvalidConfig(_))
    ));
    session.collect()?;
    for value in values {
        session.release_retained(value)?;
    }
    assert!((session.diagnostics().unwrap()[4] - 1.0).abs() < f64::EPSILON);
    Ok(())
}

struct CloseDuringProjection {
    input: NumericServiceBatch<i64>,
    projected: Arc<AtomicUsize>,
}
impl SchemeServiceRequest for CloseDuringProjection {
    type Output = Vec<Vec<i64>>;
    fn work_units(&self) -> usize {
        self.input.work_units()
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.input.construct(session)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        value: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        assert_eq!(self.projected.fetch_add(1, Ordering::AcqRel), 0);
        let op = session.numeric_vector(&[21_i64])?;
        let closed = session.call_export(c"gerbil_owner_service_observe", &op)?;
        drop(closed);
        drop(op);
        session.collect()?;
        self.input.project(session, value)
    }
}
fn invalidated_slots(runtime: &tokio::runtime::Runtime) {
    let mut settings = config();
    settings.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    dispatch
        .configure_scheme_service_completion_on(
            0,
            gerbil_scheme::instances::SchemeServiceCompletion::NativeCycle,
        )
        .unwrap()
        .wait(GATE)
        .unwrap();
    let projected = Arc::new(AtomicUsize::new(0));
    runtime.block_on(async {
        let mut tickets = Vec::new();
        for _ in 0..3 {
            tickets.push(
                dispatch
                    .submit_scheme_service_backpressured_on(
                        0,
                        CloseDuringProjection {
                            input: batch(true),
                            projected: Arc::clone(&projected),
                        },
                        GATE,
                    )
                    .await
                    .unwrap(),
            );
        }
        let start = Instant::now();
        while command(&dispatch, 0, 10)[0] < 3 {
            assert!(start.elapsed() < GATE);
            tokio::task::yield_now().await;
        }
        command(&dispatch, 0, 18);
        let mut success = 0;
        let mut failed = 0;
        for ticket in tickets {
            match ticket.wait_completed(GATE).await {
                Ok((values, _)) => {
                    success += 1;
                    assert_eq!(values.len(), 8);
                    corpus::peers(&values);
                }
                Err(_) => failed += 1,
            }
        }
        assert_eq!((success, failed), (1, 2));
    });
    assert_eq!(
        projected.load(Ordering::Acquire),
        1,
        "failed native slot cannot run a projector"
    );
    let close = pool.shutdown(GATE);
    assert!(close.is_clean(), "{close:?}");
    assert!(
        close
            .owners
            .iter()
            .all(|o| o.roots_before_cleanup == 0 && o.image_file_removed)
    );
    assert!(
        dispatch
            .owner_capacity()
            .iter()
            .all(|c| c.available == c.limit && c.work_available == c.work_limit)
    );
}
