//! Equal native values, Scheme closure computation and bulk result projection.
use super::{GATE, Progress, bounded_integer, config, finish, normal, observed};
use gerbil_scheme::instances::InstancePool;
use std::time::Instant;
fn batch(pool: &InstancePool, elements: usize, progress: &Progress) {
    let tickets: Vec<_> = (0..8)
        .map(|salt| {
            pool.dispatcher()
                .submit_native_on(salt % pool.owners().len(), move |session| {
                    let mut values: Vec<i64> = (0..elements)
                        .map(|index| bounded_integer(index % 997 + salt))
                        .collect();
                    values[0] = 0;
                    let input = session.numeric_vector(&values)?;
                    let output = session.call(&input)?;
                    session.copy_numeric::<i64>(&output)
                })
                .unwrap()
        })
        .collect();
    for (salt, ticket) in tickets.iter().enumerate() {
        let output = ticket.wait(GATE).unwrap();
        assert_eq!(output.len(), elements - 1);
        for (index, value) in output.iter().enumerate() {
            assert_eq!(*value, bounded_integer((index + 1) % 997 + salt) * 3 + 7);
        }
        observed(progress, "observed matched native Scheme result");
    }
}
pub(super) fn run(progress: &Progress) {
    let mut pools = Vec::new();
    for owners in [1, 4] {
        let start = Instant::now();
        let pool = InstancePool::start(&normal(), config(owners)).unwrap();
        observed(progress, "observed matched native pool initialized");
        eprintln!(
            "NATIVE-AB-SETUP owners={owners} ns={}",
            start.elapsed().as_nanos()
        );
        pools.push(pool);
    }
    for elements in [8192, 32768, 131_072] {
        for pool in &pools {
            batch(pool, elements, progress);
            batch(pool, elements, progress);
        }
        let mut samples: [Vec<u128>; 2] = [Vec::new(), Vec::new()];
        for block in 0..10 {
            for variant in [0, 1, 1, 0] {
                let start = Instant::now();
                batch(&pools[variant], elements, progress);
                let ns = start.elapsed().as_nanos();
                samples[variant].push(ns);
                eprintln!(
                    "NATIVE-AB-SAMPLE block={block} owners={} elements={elements} tasks=8 ns={ns} validated=8 json=false",
                    pools[variant].owners().len()
                );
            }
        }
        for values in &mut samples {
            values.sort_unstable();
        }
        let median = |values: &[u128]| values[9].midpoint(values[10]);
        let a = median(&samples[0]);
        let b = median(&samples[1]);
        eprintln!(
            "NATIVE-AB-RESULT elements={elements} samples-per-variant=20 a-owners=1 b-owners=4 a-median-ns={a} b-median-ns={b} ratio-milli={}",
            a * 1000 / b
        );
    }
    for pool in &mut pools {
        finish(pool, progress);
    }
}
