//! Capability roots, rollback, full application arena and native Scheme errors.
use super::*;
use gerbil_scheme::instances::SchemeServiceCompletion;
fn roots(dispatch: &InstanceDispatcher) -> f64 {
    dispatch
        .submit_native_on(0, |session| Ok(session.diagnostics().unwrap()[4]))
        .unwrap()
        .wait(GATE)
        .unwrap()
}
fn configure(dispatch: &InstanceDispatcher, enabled: bool) {
    assert_eq!(
        dispatch
            .configure_scheme_service_native_entry_on(0, enabled)
            .unwrap()
            .wait(GATE)
            .unwrap(),
        enabled
    );
}
fn abandoned(dispatch: &InstanceDispatcher) {
    let (ready, entered) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let blocked = dispatch
        .submit_native_on(0, move |_| {
            ready.send(()).unwrap();
            wait.recv_timeout(GATE).unwrap();
            Ok(())
        })
        .unwrap();
    entered.recv_timeout(GATE).unwrap();
    drop(
        dispatch
            .configure_scheme_service_native_entry_on(0, true)
            .unwrap(),
    );
    release.send(()).unwrap();
    blocked.wait(GATE).unwrap();
    assert!(roots(dispatch).abs() < f64::EPSILON);
}
pub(super) fn contract(runtime: &tokio::runtime::Runtime, progress: &mpsc::Sender<&'static str>) {
    progress
        .send("PHASE native entry capability contract")
        .unwrap();
    let mut cfg = config();
    cfg.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), cfg).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    dispatch
        .configure_scheme_service_completion_on(0, SchemeServiceCompletion::PipelineCycle)
        .unwrap()
        .wait(GATE)
        .unwrap();
    abandoned(&dispatch);
    configure(&dispatch, true);
    configure(&dispatch, true);
    assert!((roots(&dispatch) - 1.0).abs() < f64::EPSILON);
    runtime_types::contract(&dispatch);
    work::contract(runtime, &dispatch, progress);
    work_plan::contract(runtime, &dispatch, progress);
    joint::contract(runtime, &dispatch, progress);
    joint::enable(&dispatch);
    let retained = dispatch
        .submit_native_on(0, |session| {
            let mut values = Vec::with_capacity(65536);
            for _ in 0..65536 {
                values.push(session.retain(session.numeric_vector(&[1_i64])?)?);
            }
            assert!(matches!(
                session.retain(session.numeric_vector(&[2_i64])?),
                Err(InstanceError::InvalidConfig(_))
            ));
            session.collect()?;
            Ok(values)
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    progress
        .send("native entry capability does not consume any of 65536 application roots")
        .unwrap();
    runtime.block_on(async {
        let failure = dispatch
            .submit_scheme_service_backpressured_on(
                0,
                NumericServiceBatch {
                    items: vec![vec![0_i64]],
                    yield_every: 1,
                },
                GATE,
            )
            .await
            .unwrap()
            .wait_completed(GATE)
            .await
            .unwrap_err();
        assert!(matches!(failure, InstanceError::Scheme(_)));
        let value = dispatch
            .submit_scheme_service_backpressured_on(0, batch(false), GATE)
            .await
            .unwrap()
            .wait_completed(GATE)
            .await
            .unwrap()
            .0;
        corpus::peers(&value);
        idle(&dispatch).await;
    });
    configure(&dispatch, false);
    configure(&dispatch, true);
    dispatch
        .configure_scheme_service_completion_on(0, SchemeServiceCompletion::PipelineCycle)
        .unwrap()
        .wait(GATE)
        .unwrap();
    joint::assert_disabled(&dispatch);
    assert!((roots(&dispatch) - 65536.0).abs() < f64::EPSILON);
    dispatch
        .submit_native_on(0, move |session| {
            for value in retained {
                session.release_retained(value)?;
            }
            Ok(())
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert!(roots(&dispatch).abs() < f64::EPSILON);
    joint::abandon(runtime, &dispatch);
    let report = pool.shutdown(GATE);
    assert!(report.is_clean(), "{report:?}");
    assert!(
        report
            .owners
            .iter()
            .all(|o| o.roots_before_cleanup == 0 && o.image_file_removed)
    );
    small_reply(runtime);
    progress.send("native entry rollback/revocation, full-root Actor error/recovery, small-output quota and clean close passed").unwrap();
}

pub(super) fn reject(dispatch: &InstanceDispatcher) {
    assert!(matches!(
        dispatch
            .configure_scheme_service_native_entry_on(0, true)
            .unwrap()
            .wait(GATE),
        Err(InstanceError::Scheme(_))
    ));
}

fn small_reply(runtime: &tokio::runtime::Runtime) {
    let mut cfg = config();
    cfg.owners = 1;
    cfg.maximum_reply_bytes = 8;
    let mut pool = InstancePool::start(&descriptor(false), cfg).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    dispatch
        .configure_scheme_service_completion_on(0, SchemeServiceCompletion::PipelineCycle)
        .unwrap()
        .wait(GATE)
        .unwrap();
    configure(&dispatch, true);
    runtime.block_on(async {
        let result = dispatch
            .submit_scheme_service_backpressured_on(0, batch(false), GATE)
            .await
            .unwrap()
            .wait_completed(GATE)
            .await;
        assert!(matches!(result, Err(InstanceError::InvalidConfig(_))));
        let result = dispatch
            .submit_scheme_service_backpressured_on(
                0,
                VectorRequest {
                    input: vec![0, 3],
                    panic: false,
                },
                GATE,
            )
            .await
            .unwrap()
            .wait_completed(GATE)
            .await
            .unwrap()
            .0;
        assert_eq!(result, [16]);
        idle(&dispatch).await;
    });
    assert!((roots(&dispatch) - 1.0).abs() < f64::EPSILON);
    let report = pool.shutdown(GATE);
    assert!(report.is_clean(), "{report:?}");
    assert!(
        report
            .owners
            .iter()
            .all(|o| o.roots_before_cleanup == 0 && o.image_file_removed)
    );
}
