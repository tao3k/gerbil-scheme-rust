use divan::{Bencher, black_box};
use gerbil_scheme::GerbilRuntime;

fn main() {
    divan::main();
}

/// Measures one representative steady-state triplet through the public safe API.
///
/// Runtime initialization and cleanup stay outside the measured closure. Gerbil's
/// process-global runtime is deliberately single-threaded, so this benchmark must
/// not advertise parallel throughput that the binding contract cannot provide.
#[divan::bench(
    sample_count = 100,
    sample_size = 1000,
    min_time = 0,
    max_time = 1,
    threads = 1
)]
fn scalar_triplet(bencher: Bencher<'_, '_>) {
    let runtime = GerbilRuntime::initialize().expect("initialize the live Gerbil runtime");

    bencher.bench_local(|| {
        let value = black_box(41_i64);
        let sum = runtime
            .add_i64(value, black_box(1))
            .expect("benchmark Gerbil addition");
        let even = runtime
            .is_even_i64(value)
            .expect("benchmark Gerbil predicate");
        let ordering = runtime
            .compare_i64(value, black_box(42))
            .expect("benchmark Gerbil comparison");
        black_box((sum, even, ordering))
    });
}
