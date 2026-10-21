//! Same native order computation and projection for synchronous/asynchronous arms.
use super::{GATE, corpus};
use gerbil_scheme::instances::{
    AsyncInstanceTicket, InstanceDispatcher, InstanceError, NativeRoot, NativeSession,
    NumericServiceBatch, SchemeServiceRequest,
};
use gerbil_scheme_python::{
    owner_dispatch::{OwnerResult, WindowBudget},
    owner_flow::{FlowControl, owner_flow_backpressured},
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
#[derive(Debug)]
pub(super) struct Reply {
    pub values: Vec<Vec<i64>>,
    pub entered: Instant,
    pub constructed: Instant,
    pub projection: Instant,
}
struct Orders {
    request: NumericServiceBatch<i64>,
    entered: Option<Instant>,
    constructed: Option<Instant>,
}
impl SchemeServiceRequest for Orders {
    type Output = Reply;
    fn work_units(&self) -> usize {
        self.request.work_units()
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.entered = Some(Instant::now());
        let value = self.request.construct(session)?;
        self.constructed = Some(Instant::now());
        Ok(value)
    }
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Reply, InstanceError> {
        let projection = Instant::now();
        let values = self.request.project(session, output)?;
        Ok(Reply {
            values,
            entered: self.entered.unwrap(),
            constructed: self.constructed.unwrap(),
            projection,
        })
    }
}
pub(super) async fn submit(
    dispatch: InstanceDispatcher,
    owner: usize,
    items: Vec<Vec<i64>>,
    asynchronous: bool,
    timeout: Duration,
) -> Result<AsyncInstanceTicket<Reply>, InstanceError> {
    let request = Orders {
        request: NumericServiceBatch {
            items,
            yield_every: 1,
        },
        entered: None,
        constructed: None,
    };
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
) -> Vec<OwnerResult<Reply>> {
    let jobs = inputs
        .chunks(8)
        .enumerate()
        .map(|(index, items)| (index / 4 % 4, items.to_vec()))
        .collect();
    owner_flow_backpressured(
        jobs,
        FlowControl {
            budget: WindowBudget {
                released,
                timeout: GATE,
            },
            maximum_in_flight: 128,
        },
        move |owner, items| {
            submit(
                dispatch.clone(),
                owner,
                items,
                asynchronous,
                GATE.saturating_sub(released.elapsed()),
            )
        },
    )
    .await
}
pub(super) struct Probes {
    pub actual: Vec<u128>,
    pub scheduled: Vec<u128>,
    pub admission: Vec<u128>,
    pub service: Vec<u128>,
    pub completed: Vec<Instant>,
}
pub(super) async fn probes(
    dispatch: InstanceDispatcher,
    count: usize,
    asynchronous: bool,
    released: Instant,
) -> Probes {
    let mut values = Probes {
        actual: Vec::new(),
        scheduled: Vec::new(),
        admission: Vec::new(),
        service: Vec::new(),
        completed: Vec::new(),
    };
    for id in 0..count {
        let scheduled = released + Duration::from_micros(u64::try_from(id * 500).unwrap());
        tokio::time::sleep_until(tokio::time::Instant::from_std(scheduled)).await;
        let start = Instant::now();
        let mut input = corpus::order(100_000 + id, false);
        input.truncate(14);
        let ticket = submit(
            dispatch.clone(),
            id % 4,
            vec![input.clone()],
            asynchronous,
            GATE.saturating_sub(released.elapsed()),
        )
        .await
        .unwrap();
        values.admission.push(start.elapsed().as_nanos());
        let (reply, completed) = ticket
            .wait_completed(GATE.saturating_sub(released.elapsed()))
            .await
            .unwrap();
        corpus::verify(&input, &reply.values[0]);
        values
            .actual
            .push(completed.duration_since(start).as_nanos());
        values
            .scheduled
            .push(completed.saturating_duration_since(scheduled).as_nanos());
        values
            .service
            .push(completed.duration_since(reply.entered).as_nanos());
        values.completed.push(completed);
    }
    values
}
