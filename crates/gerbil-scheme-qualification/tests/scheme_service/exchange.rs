//! Direct typed exchange preserves generic Scheme execution and slot custody.
use super::*;
use gerbil_scheme::instances::{SchemeServiceInput, SchemeServiceOutput, SchemeServiceSubmission};
struct Exchange {
    value: Vec<i64>,
    bound: usize,
    wrong_kind: bool,
}
impl SchemeServiceRequest for Exchange {
    type Output = Vec<i64>;
    fn work_units(&self) -> usize {
        1
    }
    fn short_service(&self) -> bool {
        true
    }
    fn construct<'vm>(&mut self, _: &NativeSession<'vm>) -> Result<NativeRoot<'vm>, InstanceError> {
        Err(InstanceError::InvalidConfig("exchange context required"))
    }
    fn enter_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &SchemeServiceInput<'_>,
    ) -> Result<SchemeServiceSubmission<'vm>, InstanceError> {
        let roots = session.diagnostics().unwrap()[4].to_bits();
        let result = input.enter_tagged_numeric(session, &[77_i64, 1], &self.value);
        assert_eq!(
            session.diagnostics().unwrap()[4].to_bits(),
            roots,
            "no input or identity root"
        );
        session.collect()?;
        result
    }
    fn project_service(
        self,
        session: &NativeSession<'_>,
        output: &SchemeServiceOutput<'_>,
    ) -> Result<Vec<i64>, InstanceError> {
        let roots = session.diagnostics().unwrap()[4].to_bits();
        session.collect()?;
        let values = if self.wrong_kind {
            output
                .copy_numeric::<f64>(session, self.bound)
                .map(|_| Vec::new())
        } else {
            output.copy_numeric::<i64>(session, self.bound)
        };
        assert_eq!(
            session.diagnostics().unwrap()[4].to_bits(),
            roots,
            "no projected value root"
        );
        assert!(matches!(
            output.copy_numeric::<i64>(session, self.bound),
            Err(InstanceError::InvalidConfig(
                "service result already projected"
            ))
        ));
        values
    }
    fn project(self, _: &NativeSession<'_>, _: &NativeRoot<'_>) -> Result<Vec<i64>, InstanceError> {
        unreachable!()
    }
}
fn request(bound: usize, wrong_kind: bool) -> Exchange {
    let mut value = corpus::order(17, false);
    value.truncate(14);
    Exchange {
        value,
        bound,
        wrong_kind,
    }
}
fn enable(dispatch: &InstanceDispatcher, enabled: bool) {
    dispatch
        .configure_scheme_service_exchange_on(0, enabled)
        .unwrap()
        .wait(GATE)
        .unwrap();
}
#[test]
fn exchange_retains_quota_gc_exception_and_exactly_once_slot_custody() {
    let _case = native_case();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    for limit in [183, 184] {
        let mut settings = config();
        settings.owners = 1;
        settings.maximum_request_bytes = limit;
        let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
        let dispatch = pool.dispatcher();
        dispatch.configure_owner_budgets(4, Some(32)).unwrap();
        enable(&dispatch, true);
        runtime.block_on(async {
            let input = request(21, false);
            let expected = input.value.clone();
            let result = dispatch
                .submit_scheme_service_backpressured_on(0, input, GATE)
                .await
                .unwrap()
                .wait_completed(GATE)
                .await;
            if limit == 183 {
                assert!(
                    matches!(
                        result,
                        Err(InstanceError::InvalidConfig("native input bound"))
                    ),
                    "{result:?}"
                );
            } else {
                corpus::verify(&expected, &result.unwrap().0);
                for (bound, wrong) in [(1, false), (21, true), (usize::MAX, false)] {
                    assert!(
                        dispatch
                            .submit_scheme_service_backpressured_on(0, request(bound, wrong), GATE)
                            .await
                            .unwrap()
                            .wait_completed(GATE)
                            .await
                            .is_err()
                    );
                    idle(&dispatch).await;
                }
                let mut failed = request(21, false);
                failed.value[2] = 16;
                let result = dispatch
                    .submit_scheme_service_backpressured_on(0, failed, GATE)
                    .await
                    .unwrap()
                    .wait_completed(GATE)
                    .await;
                assert!(
                    matches!(result, Err(InstanceError::Scheme(_))),
                    "{result:?}"
                );
                let good = dispatch
                    .submit_scheme_service_backpressured_on(0, request(21, false), GATE)
                    .await
                    .unwrap()
                    .wait_completed(GATE)
                    .await
                    .unwrap();
                corpus::verify(&expected, &good.0);
            }
            idle(&dispatch).await;
        });
        enable(&dispatch, false);
        let receipt = pool.shutdown(GATE);
        assert!(receipt.is_clean(), "{receipt:?}");
        assert!(receipt.owners.iter().all(|o| o.roots_before_cleanup == 0));
    }
}
#[test]
fn exchange_preserves_generic_multishot_continuation_and_error_drain() {
    let _case = native_case();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut settings = config();
    settings.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    enable(&dispatch, true);
    runtime.block_on(async {
        for (op, expected) in [(1, 23), (2, 107), (3, 109), (4, 31)] {
            assert_eq!(
                vector(&dispatch, 0, vec![2, op, 100], false).await.unwrap(),
                [expected]
            );
        }
        assert_eq!(
            vector(&dispatch, 0, vec![0, 2, 3], false).await.unwrap(),
            [13, 16]
        );
        assert!(matches!(
            vector(&dispatch, 0, vec![1], false).await,
            Err(InstanceError::Scheme(_))
        ));
        assert_eq!(vector(&dispatch, 0, vec![5], false).await.unwrap(), [1, 1]);
        idle(&dispatch).await;
    });
    enable(&dispatch, false);
    let receipt = pool.shutdown(GATE);
    assert!(receipt.is_clean(), "{receipt:?}");
    assert!(receipt.owners.iter().all(|o| o.roots_before_cleanup == 0));
}
#[test]
fn exchange_continuous_window_survives_unread_cancel_expiry_and_close() {
    let _case = native_case();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut settings = config();
    settings.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    enable(&dispatch, true);
    runtime.block_on(async {
        let feed = dispatch
            .open_scheme_service_continuous::<Exchange>(
                gerbil_scheme::instances::SchemeServiceStreamConfig {
                    owner: 0,
                    maximum_active: 3,
                    maximum_staged_work: 2,
                    timeout: GATE,
                },
            )
            .await
            .unwrap();
        let a = feed
            .try_submit_with(Instant::now() + GATE, || Ok(request(21, false)))
            .unwrap();
        let b = feed
            .try_submit_with(Instant::now() + GATE, || Ok(request(21, false)))
            .unwrap();
        let start = Instant::now();
        while !a.is_finished() || !b.is_finished() {
            assert!(start.elapsed() < GATE);
            tokio::task::yield_now().await;
        }
        idle(&dispatch).await;
        assert!(matches!(
            feed.try_submit_with(Instant::now() + GATE, || panic!(
                "unread window must not prepare"
            )),
            Err(InstanceError::QueueFull)
        ));
        assert!(matches!(
            feed.submit_with(Instant::now() + Duration::from_millis(10), || panic!(
                "expiry must not prepare"
            ))
            .await,
            Err(InstanceError::Timeout)
        ));
        drop(a);
        let c = feed
            .submit_with(Instant::now() + GATE, || Ok(request(21, false)))
            .await
            .unwrap();
        b.wait_completed(GATE).await.unwrap();
        drop(c);
        feed.cancel();
        drop(feed);
        idle(&dispatch).await;
    });
    enable(&dispatch, false);
    let receipt = pool.shutdown(GATE);
    assert!(receipt.is_clean(), "{receipt:?}");
    assert!(receipt.owners.iter().all(|o| o.roots_before_cleanup == 0));
}
