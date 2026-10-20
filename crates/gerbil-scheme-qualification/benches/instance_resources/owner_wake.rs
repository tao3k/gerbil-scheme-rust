//! Idle-owner transport diagnostic; not an application performance claim.
use super::{
    GATE, Progress, checkpoint, command, config, finish, observed, orders::orders_program,
};
use gerbil_scheme::instances::InstancePool;
use std::time::{Duration, Instant};
pub(super) fn run(progress: &Progress) {
    let mut pool = InstancePool::start(&orders_program(), config(4)).unwrap();
    observed(progress, "observed owner-wakeup diagnostic initialization");
    let dispatch = pool.dispatcher();
    let mut counts = [None; 4];
    for idle in [false, true] {
        let mut rows = Vec::new();
        for _ in 0..512 {
            if idle {
                std::thread::sleep(Duration::from_millis(1));
            }
            let mut pending = Vec::new();
            for owner in 0..4 {
                let sent = Instant::now();
                let ticket = dispatch
                    .submit_native_on(owner, |session| {
                        let entered = Instant::now();
                        let counters = session.diagnostics().expect("native counters");
                        Ok((entered, counters))
                    })
                    .unwrap();
                pending.push((owner, sent, Instant::now(), ticket));
            }
            for (owner, sent, admitted, ticket) in pending {
                let ((entered, counters), completed) = ticket.wait_completed(GATE).unwrap();
                assert_eq!(counters[4].to_bits(), 0.0_f64.to_bits());
                if let Some(previous) = counts[owner] {
                    assert_eq!(previous, counters[5].to_bits());
                }
                counts[owner] = Some(counters[5].to_bits());
                rows.push([
                    entered.duration_since(sent).as_nanos(),
                    admitted.duration_since(sent).as_nanos(),
                    entered.saturating_duration_since(admitted).as_nanos(),
                    completed.duration_since(entered).as_nanos(),
                ]);
                observed(progress, "observed idle-owner diagnostic completion");
            }
        }
        // No logging or formatting in producer/owner timed spans.
        for row in rows {
            eprintln!("WAKE,{idle},{},{},{},{}", row[0], row[1], row[2], row[3]);
        }
    }
    for owner in 0..4 {
        command(&dispatch, owner, vec![7], progress);
    }
    checkpoint(&pool, progress);
    finish(&mut pool, progress);
}
