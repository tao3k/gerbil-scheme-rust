//! User declarations are sampled once before any class/native wait.
use super::{GATE, corpus, idle, short};
use gerbil_scheme::instances::{
    InstanceDispatcher, InstanceError, NativeRoot, NativeSession, SchemeRoutingAdmission,
    SchemeRoutingConfig, SchemeServiceLane, SchemeServiceRequest, SchemeServiceRouter,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
struct Once {
    request: short::Short,
    reads: Arc<[AtomicUsize; 3]>,
}
impl SchemeServiceRequest for Once {
    type Output = Vec<Vec<i64>>;
    fn work_units(&self) -> usize {
        assert_eq!(self.reads[0].fetch_add(1, Ordering::Relaxed), 0);
        1
    }
    fn short_service(&self) -> bool {
        assert_eq!(self.reads[1].fetch_add(1, Ordering::Relaxed), 0);
        true
    }
    fn lane(&self) -> Option<SchemeServiceLane> {
        assert_eq!(self.reads[2].fetch_add(1, Ordering::Relaxed), 0);
        None
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.request.construct(session)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        self.request.project(session, output)
    }
}
pub(super) async fn contract(dispatch: &InstanceDispatcher) {
    assert_eq!(
        SchemeRoutingAdmission::default(),
        SchemeRoutingAdmission::Pinned
    );
    for admission in [
        SchemeRoutingAdmission::Pinned,
        SchemeRoutingAdmission::Reselect,
    ] {
        let router = SchemeServiceRouter::with_admission(
            dispatch.clone(),
            SchemeRoutingConfig {
                owners: vec![0, 1],
                short_slots: 1,
                batch_slots: 3,
                short_max_work: 1,
                batch_max_work: 8,
            },
            admission,
        )
        .unwrap();
        let reads = Arc::new(std::array::from_fn(|_| AtomicUsize::new(0)));
        let mut input = corpus::order(811, false);
        input.truncate(14);
        let reply = router
            .submit_short(
                Once {
                    request: short::Short(input.clone()),
                    reads: Arc::clone(&reads),
                },
                GATE,
            )
            .await
            .unwrap();
        let values = reply.ticket.wait_completed(GATE).await.unwrap().0;
        corpus::verify(&input, &values[0]);
        assert!(reads.iter().all(|count| count.load(Ordering::Relaxed) == 1));
        let failure = router
            .submit_short(short::Short(corpus::order(812, false)), GATE)
            .await
            .unwrap()
            .ticket
            .wait_completed(GATE)
            .await;
        assert!(
            matches!(failure, Err(InstanceError::Scheme(_))),
            "oversized short handler must still reject natively"
        );
        idle(dispatch).await;
        assert!(router.loads().iter().all(|load| load.short_requests == 0
            && load.batch_requests == 0
            && load.batch_work == 0));
    }
}
