//! Identical fresh native construction and owned projection in every arm.
use gerbil_scheme::instances::{
    InstanceError, NativeRoot, NativeSession, NumericServiceBatch, SchemeServiceRequest,
};
use std::time::Instant;

pub(super) struct Request {
    pub batch: NumericServiceBatch<i64>,
    pub short: bool,
    entered: Option<Instant>,
    constructed: Option<Instant>,
}
pub(super) struct Reply {
    pub values: Vec<Vec<i64>>,
    pub entered: Instant,
    pub constructed: Instant,
    pub projecting: Instant,
    pub projected: Instant,
}
impl Request {
    pub fn new(items: Vec<Vec<i64>>, short: bool) -> Self {
        Self {
            batch: NumericServiceBatch {
                items,
                yield_every: 1,
            },
            short,
            entered: None,
            constructed: None,
        }
    }
    pub fn native(mut self, session: &NativeSession<'_>) -> Result<Reply, InstanceError> {
        let input = self.construct(session)?;
        let header = session.numeric_vector(&[
            49_i64,
            i64::from(self.short),
            i64::try_from(self.batch.items.len()).expect("bounded batch"),
        ])?;
        let command = session.vector(&[&header, &input])?;
        let output = session.call_export(c"gerbil_owner_service_reference", &command)?;
        self.project(session, &output)
    }
}
impl SchemeServiceRequest for Request {
    type Output = Reply;
    fn short_service(&self) -> bool {
        self.short
    }
    fn work_units(&self) -> usize {
        self.batch.items.len()
    }
    fn construct<'vm>(
        &mut self,
        session: &NativeSession<'vm>,
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.entered = Some(Instant::now());
        let value = if self.short {
            let tag = session.numeric_vector(&[77_i64, 1])?;
            let row = session.numeric_vector(&self.batch.items[0])?;
            session.vector(&[&tag, &row])?
        } else {
            self.batch.construct(session)?
        };
        self.constructed = Some(Instant::now());
        Ok(value)
    }
    // Deliberately retain the generic service constructor: each arm constructs
    // the same native payload, rather than comparing different fused builders.
    fn project(
        self,
        session: &NativeSession<'_>,
        output: &NativeRoot<'_>,
    ) -> Result<Reply, InstanceError> {
        let projecting = Instant::now();
        let values = if self.short {
            vec![session.copy_numeric(output)?]
        } else {
            self.batch.project(session, output)?
        };
        Ok(Reply {
            values,
            entered: self.entered.unwrap(),
            constructed: self.constructed.unwrap(),
            projecting,
            projected: Instant::now(),
        })
    }
}
