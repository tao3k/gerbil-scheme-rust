//! Single-entry Scheme input construction versus rooted reference construction.
//! Both arms retain stable child identities during native numeric projection.
use gerbil_scheme::instances::{
    AsyncInstanceTicket, BatchTimeline, InstanceDispatcher, InstanceError, NativeSession,
    ProfiledBatch,
};
use std::time::{Duration, Instant};

pub(super) async fn submit(
    dispatch: InstanceDispatcher,
    owner: usize,
    items: Vec<Vec<i64>>,
    native_constructor: bool,
    timeout: Duration,
) -> Result<AsyncInstanceTicket<ProfiledBatch<i64>>, InstanceError> {
    let dispatched = Instant::now();
    dispatch
        .submit_native_weighted_backpressured_on(
            owner,
            items.len(),
            move |session| execute(session, &items, dispatched, native_constructor),
            timeout,
        )
        .await
}
fn execute(
    session: &NativeSession<'_>,
    items: &[Vec<i64>],
    dispatched: Instant,
    native_constructor: bool,
) -> Result<ProfiledBatch<i64>, InstanceError> {
    let count = session.gc_collections();
    let entered = Instant::now();
    let mut timeline = BatchTimeline {
        dispatched,
        entered,
        native_constructor,
        items_constructed: entered,
        constructed: entered,
        executed: entered,
        projected: entered,
        retired: entered,
        collections: [count; 4],
        collections_after_retirement: count,
        diagnostics: [session.diagnostics(); 5],
        diagnostics_after_retirement: None,
    };
    // Public native APIs in both arms. Full inputs/outputs, handler, quantum,
    // weighted admission and Scheme GC agree. Bulk construction is one phase.
    let values = {
        let roots = if native_constructor {
            Vec::new()
        } else {
            items
                .iter()
                .map(|item| session.numeric_vector(item))
                .collect::<Result<Vec<_>, _>>()?
        };
        if !native_constructor {
            timeline.items_constructed = Instant::now();
            timeline.diagnostics[1] = session.diagnostics();
        }
        let references: Vec<_> = roots.iter().collect();
        let input = if native_constructor {
            None
        } else {
            Some(session.vector(&references)?)
        };
        let quantum = if native_constructor {
            None
        } else {
            Some(session.numeric_vector(&[1_i64])?)
        };
        let envelope = if native_constructor {
            session.numeric_batch_envelope(items, 1)?
        } else {
            session.vector(&[quantum.as_ref().unwrap(), input.as_ref().unwrap()])?
        };
        timeline.constructed = Instant::now();
        timeline.collections[1] = session.gc_collections();
        timeline.diagnostics[2] = session.diagnostics();
        if native_constructor {
            // One Scheme entry exposes a combined construction boundary. Do not
            // invent an earlier item-only marker inside that foreign call.
            timeline.items_constructed = timeline.constructed;
            timeline.diagnostics[1] = timeline.diagnostics[2];
        }
        let output = session.call(&envelope)?;
        timeline.executed = Instant::now();
        timeline.collections[2] = session.gc_collections();
        timeline.diagnostics[3] = session.diagnostics();
        let values = (0..items.len())
            .map(|index| session.copy_numeric_element(&output, index))
            .collect::<Result<Vec<_>, _>>()?;
        timeline.projected = Instant::now();
        timeline.collections[3] = session.gc_collections();
        timeline.diagnostics[4] = session.diagnostics();
        values
    };
    timeline.retired = Instant::now();
    timeline.collections_after_retirement = session.gc_collections();
    timeline.diagnostics_after_retirement = session.diagnostics();
    Ok(ProfiledBatch { values, timeline })
}
