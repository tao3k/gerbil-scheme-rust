//! Owner-batched native projection and idempotent retirement. No Scheme root migrates.
use super::{Answer, Command, GATE, InstanceError, InstanceService, Registry, execute};
use crate::runtime::{Projection, Scalar};
use gerbil_scheme::instances::NativeSession;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, atomic::Ordering},
    time::Instant,
};

impl InstanceService {
    pub(in super::super) fn project_many(&self, ids: &[u64]) -> Answer<Vec<Scalar>> {
        let apply = self.host.apply;
        let mut values =
            self.native_batch(ids, false, move |session, registry, id| {
                match execute(
                    session,
                    Command::Inspect(id, Projection::Scalar),
                    registry,
                    apply,
                )? {
                    super::NativeOutput::Plain(super::Output::Scalar(value)) => Ok(value),
                    _ => Err(InstanceError::InvalidConfig("native projection result")),
                }
            })?;
        values.sort_by_key(|(index, _)| *index);
        Ok(values.into_iter().map(|(_, value)| value).collect())
    }
    pub(in super::super) fn release_many(&self, ids: &[u64]) -> Answer<()> {
        if self.host.closed.load(Ordering::Acquire) {
            return Ok(());
        }
        let ids = ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let apply = self.host.apply;
        self.native_batch(&ids, true, move |session, registry, id| {
            execute(session, Command::Release(id), registry, apply).map(|_| ())
        })
        .map(|_| ())
    }
    fn native_batch<R, F>(
        &self,
        ids: &[u64],
        missing_ok: bool,
        operation: F,
    ) -> Answer<Vec<(usize, R)>>
    where
        R: Send + 'static,
        F: for<'vm> Fn(&NativeSession<'vm>, &Registry, u64) -> Result<R, InstanceError>
            + Send
            + Sync
            + 'static,
    {
        if ids.len() > 4096 {
            return Err("native value batch exceeds 4096".into());
        }
        let _runtime = self.runtime()?;
        // Validate every token before entering any owner. Release tolerates retired IDs.
        let mut jobs = Vec::new();
        for (index, id) in ids.iter().copied().enumerate() {
            match self.host.registry.get(id) {
                Ok(value) => jobs.push((index, id, value.owner())),
                Err(_) if missing_ok => (),
                Err(error) => return Err(error.to_string()),
            }
        }
        let operation = Arc::new(operation);
        let mut output = Vec::new();
        for window in jobs.chunks(128) {
            let mut owners: BTreeMap<usize, Vec<(usize, u64)>> = BTreeMap::new();
            for &(index, id, owner) in window {
                owners.entry(owner).or_default().push((index, id));
            }
            let started = Instant::now();
            let mut pending = Vec::new();
            for (owner, values) in owners {
                let registry = Arc::clone(&self.host.registry);
                let operation = Arc::clone(&operation);
                let queued = Instant::now();
                pending.push(self.host.dispatch.submit_native_on(owner, move |session| {
                    registry.trace.run(
                        owner,
                        if missing_ok {
                            "release-turn"
                        } else {
                            "projection-turn"
                        },
                        queued,
                        session,
                        |_| {
                            values
                                .into_iter()
                                .map(|(index, id)| {
                                    operation(session, &registry, id).map(|value| (index, value))
                                })
                                .collect::<Result<Vec<_>, _>>()
                        },
                    )
                }));
            }
            // Observe all accepted tickets. A release delivery error may leave a subset retired.
            let mut failure = None;
            for ticket in pending {
                match ticket.and_then(|ticket| ticket.wait(GATE.saturating_sub(started.elapsed())))
                {
                    Ok(values) => output.extend(values),
                    Err(error) => {
                        failure.get_or_insert(error.to_string());
                    }
                }
            }
            if let Some(error) = failure {
                return Err(error);
            }
        }
        Ok(output)
    }
}
