//! Real native queue ordering, bounded retirement and rejected declarations.
use super::*;
struct PriorityRequest {
    id: i64,
    short: bool,
    units: usize,
    affine: bool,
}
impl SchemeServiceRequest for PriorityRequest {
    type Output = Vec<i64>;
    fn short_service(&self) -> bool {
        self.short
    }
    fn work_units(&self) -> usize {
        self.units
    }
    fn lane(&self) -> Option<SchemeServiceLane> {
        self.affine.then_some(SchemeServiceLane(0))
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        session.numeric_vector(&[self.id])
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Self::Output, InstanceError> {
        session.copy_numeric(output)
    }
}
pub(super) fn contract(runtime: &tokio::runtime::Runtime, progress: &mpsc::Sender<&'static str>) {
    // SAFETY: same build-owned private service ABI, one persistent Scheme worker.
    let program = unsafe {
        IsolatedProgram::from_image(
            gerbil_scheme_qualification::owner_instance_image(),
            c"gerbil_owner_orders_call",
        )
        .unwrap()
        .with_exports(&[c"gerbil_owner_service_observe"])
        .with_scheme_service(c"gerbil_owner_service_single")
    };
    let mut settings = config();
    settings.owners = 1;
    let mut pool = InstancePool::start(&program, settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(8, Some(8)).unwrap();
    runtime.block_on(async {
        for (units, affine) in [(2, false), (1, true)] {
            let invalid = PriorityRequest {
                id: 0,
                short: true,
                units,
                affine,
            };
            assert!(matches!(
                dispatch
                    .submit_scheme_service_backpressured_on(0, invalid, GATE)
                    .await,
                Err(InstanceError::InvalidConfig(_))
            ));
        }
        let mut tickets = Vec::new();
        // First handler blocks; general requests arrive before both short requests.
        for (id, short) in [(90, false), (2, false), (4, false), (1, true), (91, true)] {
            tickets.push(
                dispatch
                    .submit_scheme_service_backpressured_on(
                        0,
                        PriorityRequest {
                            id,
                            short,
                            units: 1,
                            affine: false,
                        },
                        GATE,
                    )
                    .await
                    .unwrap(),
            );
        }
        let start = Instant::now();
        while command(&dispatch, 0, 12)[0] < 5 {
            assert!(
                start.elapsed() < GATE,
                "service did not accept queue-order fixture"
            );
            tokio::task::yield_now().await;
        }
        assert_eq!(command(&dispatch, 0, 12)[2..], [90]);
        command(&dispatch, 0, 8);
        for (index, ticket) in tickets.into_iter().enumerate() {
            let result = ticket.wait_completed(GATE).await;
            if index == 4 {
                assert!(matches!(result, Err(InstanceError::Scheme(_))));
            } else {
                assert_eq!(result.unwrap().0, [[90, 2, 4, 1][index]]);
            }
        }
        idle(&dispatch).await;
        let observed = command(&dispatch, 0, 12);
        assert_eq!(
            observed[2..],
            [90, 1, 2, 91, 4],
            "bounded short/general turns must preserve each FIFO"
        );
        assert!(
            observed[1] <= 2,
            "short-active completion turn exceeded its budget"
        );
    });
    let shutdown = pool.shutdown(GATE);
    assert!(shutdown.is_clean(), "{shutdown:?}");
    assert_eq!(shutdown.joined, 1);
    assert!(
        shutdown
            .owners
            .iter()
            .all(|owner| owner.roots_before_cleanup == 0 && owner.image_file_removed)
    );
    progress.send("native single-worker short/general FIFO, alternating work budget, exception and retirement contract passed").unwrap();
}
