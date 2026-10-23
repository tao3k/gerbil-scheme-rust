//! Actual fused native numeric families, generic fallback and cumulative quotas.
use super::*;
use gerbil_scheme::{
    NumericElement,
    instances::{SchemeServiceCompletion, SchemeServiceInput},
};
use std::fmt::Debug;
struct Probe<T> {
    items: Vec<Vec<T>>,
}
impl<T: NumericElement + PartialEq + Debug + Send + 'static> SchemeServiceRequest for Probe<T> {
    type Output = Vec<i64>;
    fn work_units(&self) -> usize {
        1
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        session.numeric_vector(&[0_i64, 3])
    }
    fn construct_service<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
        context: &SchemeServiceInput<'_>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        let snapshot = {
            let packet = context.numeric_batch(session, &self.items, 1)?;
            session.collect()?;
            let header = session.vector_element(&packet, 0)?;
            assert_eq!(session.copy_numeric::<i64>(&header)?, [1, -1, 0, 1]);
            let payload = session.vector_element(&packet, 1)?;
            let quantum = session.vector_element(&payload, 0)?;
            assert_eq!(session.copy_numeric::<i64>(&quantum)?, [1]);
            let rows = session.vector_element(&payload, 1)?;
            for (index, expected) in self.items.iter().enumerate() {
                assert_eq!(&session.copy_numeric_element::<T>(&rows, index)?, expected);
            }
            assert!(
                session
                    .numeric_batch_snapshot::<T>(&rows, self.items.len() + 1)
                    .is_err()
            );
            if T::KIND == 7 {
                assert!(
                    session
                        .numeric_batch_snapshot::<f64>(&rows, self.items.len())
                        .is_err()
                );
            } else {
                assert!(
                    session
                        .numeric_batch_snapshot::<i64>(&rows, self.items.len())
                        .is_err()
                );
            }
            session.numeric_batch_snapshot::<T>(&rows, self.items.len())?
        };
        // Only the private snapshot now pins the child identities.
        session.collect()?;
        assert_eq!(snapshot.copy(session)?, self.items);
        let value = self.construct(session)?;
        context.wrap(session, &value)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        value: &NativeRoot<'_>,
    ) -> Result<Vec<i64>, InstanceError> {
        session.copy_numeric(value)
    }
}
async fn numeric<T: NumericElement + PartialEq + Debug + Send + 'static>(
    dispatch: &InstanceDispatcher,
    values: Vec<T>,
) {
    let actual = dispatch
        .submit_scheme_service_backpressured_on(
            0,
            Probe {
                items: vec![values, vec![]],
            },
            GATE,
        )
        .await
        .unwrap()
        .wait_completed(GATE)
        .await
        .unwrap()
        .0;
    assert_eq!(actual, [16]);
}
fn clean(pool: &mut InstancePool) {
    let receipt = pool.shutdown(GATE);
    assert!(receipt.is_clean(), "{receipt:?}");
    assert!(
        receipt
            .owners
            .iter()
            .all(|o| o.roots_before_cleanup == 0 && o.image_file_removed)
    );
}
pub(super) fn contract(runtime: &tokio::runtime::Runtime, progress: &mpsc::Sender<&'static str>) {
    for policy in [
        SchemeServiceCompletion::PreparedCycle,
        SchemeServiceCompletion::PipelineCycle,
    ] {
        let mut cfg = config();
        cfg.owners = 1;
        let mut pool = InstancePool::start_observed(&descriptor(false), cfg, |_| {
            progress
                .send("typed numeric native owner initialized")
                .unwrap();
        })
        .unwrap();
        let dispatch = pool.dispatcher();
        dispatch.configure_owner_budgets(4, Some(32)).unwrap();
        dispatch
            .configure_scheme_service_completion_on(0, policy)
            .unwrap()
            .wait(GATE)
            .unwrap();
        dispatch
            .enable_scheme_service_progress_on(0)
            .unwrap()
            .wait(GATE)
            .unwrap();
        runtime.block_on(async {
            numeric(&dispatch, vec![0_u16, u16::MAX]).await;
            numeric(&dispatch, vec![0_u32, u32::MAX]).await;
            numeric(&dispatch, vec![0_u64, u64::MAX]).await;
            numeric(&dispatch, vec![-3.25_f32, 0.0, 99.0]).await;
            numeric(&dispatch, vec![-3.25_f64, 0.0, 99.0]).await;
            numeric(&dispatch, vec![i16::MIN, i16::MAX]).await;
            numeric(&dispatch, vec![i32::MIN, i32::MAX]).await;
            numeric(&dispatch, vec![i64::MIN, i64::MAX]).await;
            numeric(&dispatch, vec![i8::MIN, i8::MAX]).await;
            idle(&dispatch).await;
        });
        clean(&mut pool);
        quota(runtime, policy, progress);
        progress.send("native service envelope: nine numeric families, empty rows, GC, metadata, generic fallback and exact cumulative quota passed").unwrap();
    }
}
fn quota(
    runtime: &tokio::runtime::Runtime,
    policy: SchemeServiceCompletion,
    progress: &mpsc::Sender<&'static str>,
) {
    let input = batch(false);
    let bytes = input
        .items
        .iter()
        .map(|x| std::mem::size_of_val(x.as_slice()))
        .sum::<usize>()
        + (input.items.len() + 3 + 2 + 2) * 8;
    for limit in [bytes - 1, bytes] {
        let mut cfg = config();
        cfg.owners = 1;
        cfg.maximum_request_bytes = limit;
        let mut pool = InstancePool::start_observed(&descriptor(false), cfg, |_| {
            progress
                .send("typed numeric native owner initialized")
                .unwrap();
        })
        .unwrap();
        let dispatch = pool.dispatcher();
        dispatch.configure_owner_budgets(4, Some(32)).unwrap();
        dispatch
            .configure_scheme_service_completion_on(0, policy)
            .unwrap()
            .wait(GATE)
            .unwrap();
        if policy == SchemeServiceCompletion::PipelineCycle {
            dispatch
                .configure_scheme_service_native_entry_on(0, true)
                .unwrap()
                .wait(GATE)
                .unwrap();
        }
        runtime.block_on(async {
            let value = dispatch
                .submit_scheme_service_backpressured_on(0, batch(false), GATE)
                .await
                .unwrap()
                .wait_completed(GATE)
                .await;
            if limit == bytes {
                corpus::peers(&value.unwrap().0);
            } else {
                assert_eq!(
                    value.unwrap_err(),
                    InstanceError::InvalidConfig("native input bound")
                );
            }
            idle(&dispatch).await;
        });
        clean(&mut pool);
    }
}
