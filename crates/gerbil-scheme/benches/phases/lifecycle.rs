//! Root population controls, not parser workloads or parallel foreign entry.

use divan::{Bencher, black_box};
use gerbil_scheme::BytestringDelimiter;

use super::with_runtime;

// Keep fixture roots live outside timing. The oldest root is deliberately used:
// a registry optimized only for the newest token must not hide lookup cost.
#[divan::bench(args = [1, 32, 256, 1000, 1024, 10000], sample_count = 40, sample_size = 25, threads = 1)]
fn oldest_root_lookup(bencher: Bencher<'_, '_>, live_roots: usize) {
    with_runtime(|runtime| {
        let roots: Vec<_> = (0..live_roots)
            .map(|_| {
                runtime
                    .bytevector_from_bytestring("A5", BytestringDelimiter::Compact)
                    .expect("root population fixture")
            })
            .collect();
        assert_eq!(roots[0].to_vec().into_result(), Ok(vec![0xA5]));
        bencher.bench_local(|| black_box(roots[0].len().into_result().expect("oldest live token")));
        for root in &roots {
            assert_eq!(root.len().into_result(), Ok(1));
        }
        // Fixture release is outside lookup timing.
        drop(roots);
        assert_eq!(runtime.add_i64(40, 2).expect("post-drain owner"), 42);
    });
}

// A complete population cycle includes creation AND release. Do not subtract
// another benchmark to label this as isolated release time. Oldest-first drops
// exercise release with retained newer roots; newest-first is a matched control.
#[divan::bench(args = [1, 32, 256, 1000, 1024, 10000], sample_count = 20, sample_size = 1, threads = 1)]
fn oldest_first_population_cycle(bencher: Bencher<'_, '_>, live_roots: usize) {
    population_cycle(bencher, live_roots, false);
}

#[divan::bench(args = [1, 32, 256, 1000, 1024, 10000], sample_count = 20, sample_size = 1, threads = 1)]
fn newest_first_population_cycle(bencher: Bencher<'_, '_>, live_roots: usize) {
    population_cycle(bencher, live_roots, true);
}

fn population_cycle(bencher: Bencher<'_, '_>, live_roots: usize, newest_first: bool) {
    with_runtime(|runtime| {
        bencher.bench_local(|| {
            let roots: Vec<_> = (0..live_roots)
                .map(|_| {
                    runtime
                        .bytevector_from_bytestring("A5", BytestringDelimiter::Compact)
                        .expect("create population")
                })
                .collect();
            black_box(&roots);
            if newest_first {
                for root in roots.into_iter().rev() {
                    drop(root);
                }
            } else {
                for root in roots {
                    drop(root);
                }
            }
        });
        assert_eq!(runtime.add_i64(40, 2).expect("post-cycle owner"), 42);
    });
}
