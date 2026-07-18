use divan::{Bencher, black_box};
use gerbil_scheme::GerbilRuntime;

fn main() {
    divan::main();
}

std::thread_local! {
    static RUNTIME: std::cell::OnceCell<GerbilRuntime> = const { std::cell::OnceCell::new() };
}

fn with_runtime(run: impl FnOnce(&GerbilRuntime)) {
    RUNTIME.with(|runtime| {
        run(runtime.get_or_init(|| {
            GerbilRuntime::initialize().expect("initialize the live Gerbil runtime")
        }));
    });
}

#[divan::bench(
    sample_count = 100,
    sample_size = 1000,
    min_time = 0,
    max_time = 1,
    threads = 1
)]
fn c_identity(bencher: Bencher<'_, '_>) {
    bencher.bench_local(|| {
        black_box(unsafe { gerbil_scheme_sys::gerbil_scheme_rust_identity_i64(black_box(41)) })
    });
}

#[divan::bench(
    sample_count = 100,
    sample_size = 1000,
    min_time = 0,
    max_time = 1,
    threads = 1
)]
fn raw_add(bencher: Bencher<'_, '_>) {
    with_runtime(|_runtime| {
        bencher.bench_local(|| {
            black_box(unsafe {
                gerbil_scheme_sys::gerbil_scheme_rust_add_i64(black_box(41), black_box(1))
            })
        });
    });
}

#[divan::bench(
    sample_count = 100,
    sample_size = 1000,
    min_time = 0,
    max_time = 1,
    threads = 1
)]
fn add(bencher: Bencher<'_, '_>) {
    with_runtime(|runtime| {
        bencher.bench_local(|| {
            black_box(
                runtime
                    .add_i64(black_box(41), black_box(1))
                    .expect("benchmark Gerbil addition"),
            )
        });
    });
}

#[divan::bench(
    sample_count = 100,
    sample_size = 1000,
    min_time = 0,
    max_time = 1,
    threads = 1
)]
fn raw_even(bencher: Bencher<'_, '_>) {
    with_runtime(|_runtime| {
        bencher.bench_local(|| {
            black_box(unsafe { gerbil_scheme_sys::gerbil_scheme_rust_is_even_i64(black_box(41)) })
        });
    });
}

#[divan::bench(
    sample_count = 100,
    sample_size = 1000,
    min_time = 0,
    max_time = 1,
    threads = 1
)]
fn even(bencher: Bencher<'_, '_>) {
    with_runtime(|runtime| {
        bencher.bench_local(|| {
            black_box(
                runtime
                    .is_even_i64(black_box(41))
                    .expect("benchmark Gerbil predicate"),
            )
        });
    });
}

#[divan::bench(
    sample_count = 100,
    sample_size = 1000,
    min_time = 0,
    max_time = 1,
    threads = 1
)]
fn raw_compare(bencher: Bencher<'_, '_>) {
    with_runtime(|_runtime| {
        bencher.bench_local(|| {
            black_box(unsafe {
                gerbil_scheme_sys::gerbil_scheme_rust_compare_i64(black_box(41), black_box(42))
            })
        });
    });
}

#[divan::bench(
    sample_count = 100,
    sample_size = 1000,
    min_time = 0,
    max_time = 1,
    threads = 1
)]
fn compare(bencher: Bencher<'_, '_>) {
    with_runtime(|runtime| {
        bencher.bench_local(|| {
            black_box(
                runtime
                    .compare_i64(black_box(41), black_box(42))
                    .expect("benchmark Gerbil comparison"),
            )
        });
    });
}
