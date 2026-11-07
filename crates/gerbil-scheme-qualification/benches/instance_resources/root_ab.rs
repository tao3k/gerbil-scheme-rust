//! Rejected root-lifetime experiment retained only for native reproduction.
//! Both arms use the same native task/conversion function and Scheme handler.
//! Bounded qualification inputs stay far below configured transfer limits;
//! the regular public API retains aggregate-output checks and original lifetimes.
use super::{
    Progress, checkpoint, command, config, finish, observed,
    orders::{orders_program, percentile, sample_using},
};
use gerbil_scheme::instances::{InstanceError, InstancePool, NativeSession};
// Original pre-optimization native conversion path, using the same typed ABI.
fn control(
    session: &NativeSession<'_>,
    items: &[Vec<i64>],
    quantum: usize,
    early_retire: bool,
) -> Result<Vec<Vec<i64>>, InstanceError> {
    let roots = items
        .iter()
        .map(|item| session.numeric_vector(item))
        .collect::<Result<Vec<_>, _>>()?;
    let references: Vec<_> = roots.iter().collect();
    let input = session.vector(&references)?;
    if early_retire {
        drop(references);
        drop(roots);
    }
    let quantum = session.numeric_vector(&[i64::try_from(quantum).unwrap()])?;
    let envelope = session.vector(&[&quantum, &input])?;
    if early_retire {
        drop(quantum);
        drop(input);
    }
    let output = session.call(&envelope)?;
    (0..items.len())
        .map(|index| {
            let item = session.vector_element(&output, index)?;
            session.copy_numeric::<i64>(&item)
        })
        .collect()
}
pub(super) fn run(progress: &Progress) {
    let mut settings = config(4);
    settings.queue_capacity = 128;
    let mut pool = InstancePool::start(&orders_program(), settings).unwrap();
    observed(progress, "observed root-lifetime A/B owners initialized");
    let dispatch = pool.dispatcher();
    for mixed in [false, true] {
        for (batch, quantum) in [(1, 0), (32, 1)] {
            let sample = |variant| {
                sample_using(
                    &pool,
                    batch,
                    quantum,
                    mixed,
                    progress,
                    |owner, items, quantum| {
                        dispatch
                            .submit_native_on(owner, move |session| {
                                control(session, &items, quantum, variant == 1)
                            })
                            .unwrap()
                    },
                )
            };
            for variant in [0, 1] {
                sample(variant);
            }
            let mut elapsed: [Vec<u128>; 2] = std::array::from_fn(|_| Vec::new());
            let mut tails: [Vec<u128>; 2] = std::array::from_fn(|_| Vec::new());
            let mut peers = [usize::MAX; 2];
            for block in 0..10 {
                for variant in [0, 1, 1, 0] {
                    let value = sample(variant);
                    eprintln!(
                        "ROOT-SAMPLE mixed={mixed} batch={batch} variant={variant} block={block} ns={} peer-min={}",
                        value.elapsed, value.peer_min
                    );
                    eprintln!(
                        "ROOT-LATENCIES mixed={mixed} batch={batch} variant={variant} block={block} ns={:?}",
                        value.latency
                    );
                    elapsed[variant].push(value.elapsed);
                    tails[variant].extend(value.latency);
                    peers[variant] = peers[variant].min(value.peer_min);
                }
            }
            for variant in 0..2 {
                eprintln!(
                    "ROOT-RESULT mixed={mixed} batch={batch} variant={variant} samples=20 median-ns={} p95-ns={} p99-ns={} peer-min={}",
                    percentile(&mut elapsed[variant], 50),
                    percentile(&mut tails[variant], 95),
                    percentile(&mut tails[variant], 99),
                    peers[variant]
                );
            }
        }
    }
    for owner in 0..4 {
        command(&dispatch, owner, vec![7], progress);
    }
    checkpoint(&pool, progress);
    finish(&mut pool, progress);
}
