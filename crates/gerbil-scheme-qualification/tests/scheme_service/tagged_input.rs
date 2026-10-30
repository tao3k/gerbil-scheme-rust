//! Same graph, byte charge and Actor endpoint; fewer external constructor roots.
use super::*;
struct Tagged {
    value: Vec<i64>,
    fused: bool,
}
impl SchemeServiceRequest for Tagged {
    type Output = Vec<Vec<i64>>;
    fn work_units(&self) -> usize {
        1
    }
    fn short_service(&self) -> bool {
        true
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        let tag = session.numeric_vector(&[77_i64, 1])?;
        let data = session.numeric_vector(&self.value)?;
        session.vector(&[&tag, &data])
    }
    fn construct_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &gerbil_scheme::instances::SchemeServiceInput<'_>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        if !self.fused {
            let payload = self.construct(session)?;
            return input.wrap(session, &payload);
        }
        let before = session.diagnostics().unwrap()[4];
        let envelope = input.tagged_numeric(session, &[77_i64, 1], &self.value)?;
        assert!(
            (session.diagnostics().unwrap()[4] - before - 1.0).abs() < f64::EPSILON,
            "only the parent may own an external root"
        );
        session.collect()?;
        Ok(envelope)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        Ok(vec![session.copy_numeric(output)?])
    }
}
#[test]
fn tagged_constructor_preserves_quota_gc_errors_and_actor_results() {
    let _case = super::native_case();
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
        runtime.block_on(async {
            for fused in [false, true] {
                let mut value = corpus::order(17, false);
                value.truncate(14);
                let result = dispatch
                    .submit_scheme_service_backpressured_on(
                        0,
                        Tagged {
                            value: value.clone(),
                            fused,
                        },
                        GATE,
                    )
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
                        "limit={limit} fused={fused} result={result:?}"
                    );
                } else {
                    corpus::verify(&value, &result.unwrap().0[0]);
                    value[2] = 16;
                    let failed = dispatch
                        .submit_scheme_service_backpressured_on(0, Tagged { value, fused }, GATE)
                        .await
                        .unwrap()
                        .wait_completed(GATE)
                        .await;
                    assert!(matches!(failed, Err(InstanceError::Scheme(_))));
                }
                idle(&dispatch).await;
            }
        });
        let receipt = pool.shutdown(GATE);
        assert!(receipt.is_clean(), "{receipt:?}");
        assert!(
            receipt
                .owners
                .iter()
                .all(|owner| owner.roots_before_cleanup == 0)
        );
    }
    let mut settings = config();
    settings.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    runtime.block_on(all_numeric_families(&dispatch));
    assert!(pool.shutdown(GATE).is_clean());
}

struct TypeProbe<T>(Vec<T>);
impl<T: gerbil_scheme::NumericElement + Send + PartialEq + std::fmt::Debug + 'static>
    SchemeServiceRequest for TypeProbe<T>
{
    type Output = ();
    fn work_units(&self) -> usize {
        1
    }
    fn short_service(&self) -> bool {
        true
    }
    fn construct<'vm>(&mut self, _: &NativeSession<'vm>) -> Result<NativeRoot<'vm>, InstanceError> {
        Err(InstanceError::InvalidConfig("service context required"))
    }
    fn construct_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        input: &gerbil_scheme::instances::SchemeServiceInput<'_>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        let envelope = input.tagged_numeric(session, &[42, -9], &self.0)?;
        session.collect()?;
        let payload = session.vector_element(&envelope, 1)?;
        let tag = session.vector_element(&payload, 0)?;
        assert_eq!(session.copy_numeric::<i64>(&tag)?, [42, -9]);
        let data = session.vector_element(&payload, 1)?;
        assert_eq!(session.copy_numeric::<T>(&data)?, self.0);
        Err(InstanceError::InvalidConfig("type inspection complete"))
    }
    fn project(self, _: &NativeSession<'_>, _: &NativeRoot<'_>) -> Result<(), InstanceError> {
        panic!("type inspection must not enter Actor execution")
    }
}
async fn probe<T: gerbil_scheme::NumericElement + Send + PartialEq + std::fmt::Debug + 'static>(
    dispatch: &InstanceDispatcher,
    values: Vec<T>,
) {
    let result = dispatch
        .submit_scheme_service_backpressured_on(0, TypeProbe(values), GATE)
        .await
        .unwrap()
        .wait_completed(GATE)
        .await;
    assert_eq!(
        result.unwrap_err(),
        InstanceError::InvalidConfig("type inspection complete")
    );
    idle(dispatch).await;
}
async fn all_numeric_families(dispatch: &InstanceDispatcher) {
    probe(dispatch, vec![0_u16, 1, u16::MAX]).await;
    probe(dispatch, vec![0_u32, 1, u32::MAX]).await;
    probe(dispatch, vec![0_u64, 1, u64::MAX]).await;
    probe(dispatch, vec![1.5_f32, -0.0, 3.25]).await;
    probe(dispatch, vec![1.5_f64, -0.0, 3.25]).await;
    probe(dispatch, vec![i16::MIN, -1, i16::MAX]).await;
    probe(dispatch, vec![i32::MIN, -1, i32::MAX]).await;
    probe(dispatch, vec![i64::MIN, -1, i64::MAX]).await;
    probe(dispatch, vec![i8::MIN, -1, i8::MAX]).await;
}
