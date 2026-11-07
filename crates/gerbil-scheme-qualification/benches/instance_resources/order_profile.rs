//! Correlated wall-clock phase receipts for the existing order workload.
use super::{
    GATE, Progress, checkpoint, command, config, finish, observed,
    orders::{order, orders_program, peers, percentile, verify},
};
use gerbil_scheme::instances::InstancePool;
use std::time::Instant;
struct Detail {
    items: u128,
    envelope: u128,
    total: u128,
    counters: [[f64; 6]; 5],
}
struct Sample {
    elapsed: u128,
    rows: Vec<[u128; 14]>,
    peer_min: usize,
    details: Vec<Detail>,
}
fn sample(pool: &InstancePool, batch: usize, mixed: bool, progress: &Progress) -> Sample {
    let dispatch = pool.dispatcher();
    let mut result = Sample {
        elapsed: 0,
        rows: Vec::new(),
        peer_min: usize::MAX,
        details: Vec::new(),
    };
    let started = Instant::now();
    for base in (0..1024).step_by(128) {
        let inputs: Vec<_> = (base..base + 128).map(|id| order(id, mixed)).collect();
        let released = Instant::now();
        let mut pending = Vec::new();
        for (chunk, items) in inputs.chunks(batch).enumerate() {
            let owner = (base + chunk * batch) / 32 % pool.owners().len();
            let ticket = dispatch
                .submit_numeric_batch_profiled_on(owner, items.to_vec(), usize::from(batch > 1))
                .unwrap();
            pending.push((chunk * batch, Instant::now(), ticket));
        }
        for (offset, admitted, ticket) in pending {
            let (output, completed) = ticket.wait_completed(GATE).unwrap();
            let t = output.timeline;
            assert!(t.entered <= t.items_constructed && t.items_constructed <= t.constructed);
            let diagnostics = t.diagnostics.map(|value| value.expect("diagnostic export"));
            assert!(
                diagnostics
                    .windows(2)
                    .all(|p| p[0][3] <= p[1][3] && p[0][5] <= p[1][5])
            );
            result.details.push(Detail {
                items: t.items_constructed.duration_since(t.entered).as_nanos(),
                envelope: t.constructed.duration_since(t.items_constructed).as_nanos(),
                total: completed.duration_since(released).as_nanos(),
                counters: diagnostics,
            });
            let marks = [
                released,
                t.dispatched,
                t.entered,
                t.constructed,
                t.executed,
                t.projected,
                completed,
            ];
            assert!(marks.windows(2).all(|pair| pair[0] <= pair[1]));
            let mut row = [0; 14];
            row[0] = batch as u128;
            row[1] = completed.duration_since(released).as_nanos();
            for (index, pair) in marks.windows(2).enumerate() {
                row[index + 2] = pair[1].duration_since(pair[0]).as_nanos();
            }
            assert_eq!(row[1], row[2..8].iter().sum::<u128>());
            for (index, count) in t.collections.iter().enumerate() {
                row[index + 8] = u128::from(*count);
            }
            assert!(t.collections.windows(2).all(|pair| pair[0] <= pair[1]));
            // Admission return bounds queue residence without assuming the
            // owner starts after try_send returns to the producer.
            row[12] = admitted.duration_since(t.dispatched).as_nanos();
            row[13] = t.entered.saturating_duration_since(admitted).as_nanos();
            assert!(row[13] <= row[3]);
            assert!(row[12] + row[13] >= row[3]);
            result.rows.push(row);
            result.peer_min = result.peer_min.min(peers(&output.values));
            assert_eq!(output.values.len(), batch);
            for (index, value) in output.values.iter().enumerate() {
                verify(&inputs[offset + index], value);
            }
            observed(progress, "observed validated profiled native order batch");
        }
    }
    result.elapsed = started.elapsed().as_nanos();
    result
}
fn emit(sample: &Sample, mixed: bool, batch: usize, block: usize) {
    eprintln!(
        "PROFILE-SAMPLE mixed={mixed} batch={batch} block={block} ns={} peer-min={}",
        sample.elapsed, sample.peer_min
    );
    // Emitted after the timed sample, never from a Scheme owner or per-item callback.
    for detail in &sample.details {
        eprintln!(
            "DETAIL,{mixed},{batch},{},{},{},{:?}",
            detail.items, detail.envelope, detail.total, detail.counters
        );
    }
    for row in &sample.rows {
        eprintln!(
            "TRACE,{mixed},{block},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            row[0],
            row[1],
            row[2],
            row[3],
            row[4],
            row[5],
            row[6],
            row[7],
            row[8],
            row[9],
            row[10],
            row[11],
            row[12],
            row[13]
        );
    }
}
pub(super) fn run(progress: &Progress) {
    let mut settings = config(4);
    settings.queue_capacity = 128;
    let mut pool = InstancePool::start(&orders_program(), settings).unwrap();
    observed(progress, "observed phase-profile native owners initialized");
    for mixed in [false, true] {
        for batch in [1, 32] {
            sample(&pool, batch, mixed, progress);
        }
        let mut samples: [Vec<u128>; 2] = std::array::from_fn(|_| Vec::new());
        let mut tails: [Vec<u128>; 2] = std::array::from_fn(|_| Vec::new());
        for block in 0..5 {
            for case in [0, 1, 1, 0] {
                let batch = [1, 32][case];
                let value = sample(&pool, batch, mixed, progress);
                emit(&value, mixed, batch, block);
                samples[case].push(value.elapsed);
                for row in value.rows {
                    tails[case].extend(std::iter::repeat_n(row[1], batch));
                }
            }
        }
        for case in 0..2 {
            eprintln!(
                "PROFILE-RESULT mixed={mixed} batch={} median-ns={} p95-ns={} p99-ns={}",
                [1, 32][case],
                percentile(&mut samples[case], 50),
                percentile(&mut tails[case], 95),
                percentile(&mut tails[case], 99)
            );
        }
    }
    for owner in 0..4 {
        command(&pool.dispatcher(), owner, vec![7], progress);
    }
    checkpoint(&pool, progress);
    finish(&mut pool, progress);
}
