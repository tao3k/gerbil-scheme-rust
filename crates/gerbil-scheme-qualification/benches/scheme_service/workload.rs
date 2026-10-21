//! Same native order computation and projection for synchronous/asynchronous arms.
use super::{GATE, corpus};
use gerbil_scheme::instances::{
    AsyncInstanceTicket, InstanceDispatcher, InstanceError, NativeRoot, NativeSession,
    NumericServiceBatch, SchemeServiceRequest, SchemeServiceRouter,
};
use gerbil_scheme_python::{
    owner_dispatch::{OwnerResult, WindowBudget},
    owner_flow::{FlowControl, owner_flow_backpressured, owner_flow_ready_backpressured},
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ShortMode {
    Batch,
    Native,
    Dedicated,
}
impl ShortMode {
    pub(super) fn native(self) -> bool {
        self != Self::Batch
    }
    pub(super) fn dedicated(self) -> bool {
        self == Self::Dedicated
    }
}
#[derive(Debug)]
pub(super) struct Reply {
    pub values: Vec<Vec<i64>>,
    pub owner: usize,
    pub entered: Instant,
    pub constructed: Instant,
    pub projection: Instant,
}
struct Orders {
    request: NumericServiceBatch<i64>,
    entered: Option<Instant>,
    constructed: Option<Instant>,
    short_mode: ShortMode,
    _bulk_permit: Option<tokio::sync::OwnedSemaphorePermit>,
}
impl SchemeServiceRequest for Orders {
    type Output = Reply;
    fn short_service(&self) -> bool {
        self.short_mode.dedicated()
    }
    fn work_units(&self) -> usize {
        self.request.work_units()
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.entered = Some(Instant::now());
        let value = if self.short_mode.native() {
            assert_eq!(self.request.items.len(), 1);
            let tag = session.numeric_vector(&[77_i64, 1])?;
            let input = session.numeric_vector(&self.request.items[0])?;
            session.vector(&[&tag, &input])?
        } else {
            self.request.construct(session)?
        };
        self.constructed = Some(Instant::now());
        Ok(value)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Reply, InstanceError> {
        let projection = Instant::now();
        let values = if self.short_mode.native() {
            vec![session.copy_numeric(output)?]
        } else {
            self.request.project(session, output)?
        };
        Ok(Reply {
            values,
            owner: session.owner_index(),
            entered: self.entered.unwrap(),
            constructed: self.constructed.unwrap(),
            projection,
        })
    }
}
fn orders(
    items: Vec<Vec<i64>>,
    short_mode: ShortMode,
    bulk_permit: Option<tokio::sync::OwnedSemaphorePermit>,
) -> Orders {
    Orders {
        request: NumericServiceBatch {
            items,
            yield_every: 1,
        },
        entered: None,
        constructed: None,
        short_mode,
        _bulk_permit: bulk_permit,
    }
}
async fn routed(
    router: &SchemeServiceRouter,
    items: Vec<Vec<i64>>,
    short_mode: ShortMode,
    timeout: Duration,
) -> Result<AsyncInstanceTicket<Reply>, InstanceError> {
    let request = orders(items, short_mode, None);
    let selected = if short_mode.native() {
        router.submit_short(request, timeout).await?
    } else {
        router.submit_batch(request, timeout).await?
    };
    Ok(selected.ticket)
}
pub(super) async fn submit(
    dispatch: InstanceDispatcher,
    owner: usize,
    items: Vec<Vec<i64>>,
    asynchronous: bool,
    short_mode: ShortMode,
    timeout: Duration,
    bulk_permit: Option<tokio::sync::OwnedSemaphorePermit>,
) -> Result<AsyncInstanceTicket<Reply>, InstanceError> {
    let request = orders(items, short_mode, bulk_permit);
    if asynchronous {
        dispatch
            .submit_scheme_service_backpressured_on(owner, request, timeout)
            .await
    } else {
        dispatch
            .submit_native_weighted_backpressured_on(
                owner,
                request.work_units(),
                move |session| {
                    let mut request = request;
                    let payload = request.construct(session)?;
                    let operation = session.numeric_vector(&[6_i64])?;
                    let input = session.vector(&[&operation, &payload])?;
                    let output = session.call_export(c"gerbil_owner_service_reference", &input)?;
                    request.project(session, &output)
                },
                timeout,
            )
            .await
    }
}
pub(super) async fn bulk(
    dispatch: InstanceDispatcher,
    inputs: Arc<Vec<Vec<i64>>>,
    asynchronous: bool,
    released: Instant,
    ready_completion: bool,
    split_pair: bool,
    router: Option<SchemeServiceRouter>,
) -> Vec<OwnerResult<Reply>> {
    let jobs = inputs
        .chunks(8)
        .enumerate()
        .map(|(index, items)| (index / 4 % 4, items.to_vec()))
        .collect();
    let control = FlowControl {
        budget: WindowBudget {
            released,
            timeout: GATE,
        },
        maximum_in_flight: 128,
    };
    // Equal explicit client admission cap in both split-layout arms. This
    // producer cannot occupy all four native request slots with bulk callbacks.
    let limits: Arc<Vec<_>> = Arc::new(
        (0..4)
            .map(|_| Arc::new(tokio::sync::Semaphore::new(3)))
            .collect(),
    );
    let submitter = move |owner: usize, items| {
        let limit = Arc::clone(&limits[owner]);
        let dispatch = dispatch.clone();
        let router = router.clone();
        async move {
            if let Some(router) = router {
                return routed(
                    &router,
                    items,
                    ShortMode::Batch,
                    GATE.saturating_sub(released.elapsed()),
                )
                .await;
            }
            let permit = if split_pair {
                Some(
                    tokio::time::timeout(
                        GATE.saturating_sub(released.elapsed()),
                        limit.acquire_owned(),
                    )
                    .await
                    .map_err(|_| InstanceError::Timeout)?
                    .map_err(|_| InstanceError::Closed)?,
                )
            } else {
                None
            };
            submit(
                dispatch,
                owner,
                items,
                asynchronous,
                ShortMode::Batch,
                GATE.saturating_sub(released.elapsed()),
                permit,
            )
            .await
        }
    };
    if ready_completion {
        owner_flow_ready_backpressured(jobs, control, submitter).await
    } else {
        owner_flow_backpressured(jobs, control, submitter).await
    }
}
pub(super) struct Probes {
    pub actual: Vec<u128>,
    pub scheduled: Vec<u128>,
    pub admission: Vec<u128>,
    pub service: Vec<u128>,
    pub construct: Vec<u128>,
    pub post_admission: Vec<u128>,
    pub pending: Vec<u128>,
    pub projection: Vec<u128>,
    pub completed: Vec<Instant>,
}
pub(super) async fn probes(
    dispatch: InstanceDispatcher,
    count: usize,
    asynchronous: bool,
    released: Instant,
    short_mode: ShortMode,
    router: Option<SchemeServiceRouter>,
) -> Probes {
    let mut values = Probes {
        actual: Vec::new(),
        scheduled: Vec::new(),
        admission: Vec::new(),
        service: Vec::new(),
        construct: Vec::new(),
        post_admission: Vec::new(),
        pending: Vec::new(),
        projection: Vec::new(),
        completed: Vec::new(),
    };
    for id in 0..count {
        let scheduled = released + Duration::from_micros(u64::try_from(id * 500).unwrap());
        tokio::time::sleep_until(tokio::time::Instant::from_std(scheduled)).await;
        let start = Instant::now();
        let mut input = corpus::order(100_000 + id, false);
        input.truncate(14);
        let ticket = if let Some(router) = router.as_ref() {
            routed(
                router,
                vec![input.clone()],
                short_mode,
                GATE.saturating_sub(released.elapsed()),
            )
            .await
        } else {
            submit(
                dispatch.clone(),
                id % 4,
                vec![input.clone()],
                asynchronous,
                short_mode,
                GATE.saturating_sub(released.elapsed()),
                None,
            )
            .await
        }
        .unwrap();
        let admitted = Instant::now();
        values
            .admission
            .push(admitted.duration_since(start).as_nanos());
        let (reply, completed) = ticket
            .wait_completed(GATE.saturating_sub(released.elapsed()))
            .await
            .unwrap();
        corpus::verify(&input, &reply.values[0]);
        values
            .post_admission
            .push(reply.entered.saturating_duration_since(admitted).as_nanos());
        values
            .actual
            .push(completed.duration_since(start).as_nanos());
        values
            .scheduled
            .push(completed.saturating_duration_since(scheduled).as_nanos());
        values
            .service
            .push(completed.duration_since(reply.entered).as_nanos());
        values
            .construct
            .push(reply.constructed.duration_since(reply.entered).as_nanos());
        values.pending.push(
            reply
                .projection
                .duration_since(reply.constructed)
                .as_nanos(),
        );
        values
            .projection
            .push(completed.duration_since(reply.projection).as_nanos());
        values.completed.push(completed);
    }
    values
}
