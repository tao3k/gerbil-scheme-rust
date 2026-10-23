//! Same-image, same-owner controls remove whole-process scheduling drift.
use super::{
    Progress, checkpoint, command, config, finish, observed,
    orders::{orders_program, percentile, sample},
};
use gerbil_scheme::instances::InstancePool;
fn select(pool: &InstancePool, variant: i64, progress: &Progress) {
    for owner in 0..pool.owners().len() {
        assert_eq!(
            command(&pool.dispatcher(), owner, vec![9, variant], progress),
            [0]
        );
    }
}
pub(super) fn run(progress: &Progress) {
    let mut settings = config(4);
    settings.queue_capacity = 128;
    let mut pool = InstancePool::start(&orders_program(), settings).unwrap();
    observed(
        progress,
        "observed shared scheduler A/B native owners initialized",
    );
    for mixed in [false, true] {
        for (batch, quantum) in [(1, 0), (32, 1)] {
            for variant in [0, 1] {
                select(&pool, variant, progress);
                sample(&pool, batch, quantum, mixed, progress);
            }
            let mut elapsed: [Vec<u128>; 2] = std::array::from_fn(|_| Vec::new());
            let mut tails: [Vec<u128>; 2] = std::array::from_fn(|_| Vec::new());
            let mut peers = [usize::MAX; 2];
            for block in 0..10 {
                for variant in [0, 1, 1, 0] {
                    select(&pool, i64::try_from(variant).unwrap(), progress);
                    let value = sample(&pool, batch, quantum, mixed, progress);
                    eprintln!(
                        "SCHEDULER-SAMPLE mixed={mixed} batch={batch} variant={variant} block={block} ns={} peer-min={}",
                        value.elapsed, value.peer_min
                    );
                    eprintln!(
                        "SCHEDULER-LATENCIES mixed={mixed} batch={batch} variant={variant} block={block} ns={:?}",
                        value.latency
                    );
                    elapsed[variant].push(value.elapsed);
                    tails[variant].extend(value.latency);
                    peers[variant] = peers[variant].min(value.peer_min);
                }
            }
            for variant in 0..2 {
                eprintln!(
                    "SCHEDULER-RESULT mixed={mixed} batch={batch} variant={variant} samples=20 median-ns={} p95-ns={} p99-ns={} peer-min={}",
                    percentile(&mut elapsed[variant], 50),
                    percentile(&mut tails[variant], 95),
                    percentile(&mut tails[variant], 99),
                    peers[variant]
                );
            }
        }
    }
    select(&pool, 1, progress);
    for owner in 0..4 {
        command(&pool.dispatcher(), owner, vec![7], progress);
    }
    checkpoint(&pool, progress);
    finish(&mut pool, progress);
}
