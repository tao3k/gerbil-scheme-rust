use divan::{Bencher, black_box};
use gerbil_scheme::{BytestringDelimiter, GerbilRuntime};

#[path = "phases/binary.rs"]
mod binary;
#[path = "phases/buffers.rs"]
mod buffers;
#[path = "phases/conversion.rs"]
mod conversion;
#[path = "phases/lifecycle.rs"]
mod lifecycle;
#[path = "phases/utf8.rs"]
mod utf8;

fn main() {
    divan::main();
}

std::thread_local! {
    static RUNTIME: std::cell::OnceCell<GerbilRuntime> = const { std::cell::OnceCell::new() };
}

fn with_runtime(run: impl FnOnce(&GerbilRuntime)) {
    RUNTIME.with(|runtime| {
        run(runtime.get_or_init(|| {
            let runtime = GerbilRuntime::initialize().expect("initialize the live Gerbil runtime");
            assert_eq!(runtime.add_i64(41, 1).expect("addition preflight"), 42);
            assert!(!runtime.is_even_i64(41).expect("predicate preflight"));
            assert_eq!(
                runtime.compare_i64(41, 42).expect("comparison preflight"),
                std::cmp::Ordering::Less
            );
            runtime
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

#[divan::bench(sample_count = 100, sample_size = 1000, threads = 1)]
fn identity(bencher: Bencher<'_, '_>) {
    with_runtime(|runtime| {
        assert_eq!(runtime.identity_i64(41).expect("identity preflight"), 41);
        bencher
            .bench_local(|| black_box(runtime.identity_i64(black_box(41)).expect("safe identity")));
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

// These are owner-local transport microbenchmarks, not concurrent parser
// throughput. A non-Send runtime must never be shared across Divan workers.
#[divan::bench(args = [0, 8192, 65536], sample_count = 40, sample_size = 25, threads = 1)]
fn rust_bulk_copy(bencher: Bencher<'_, '_>, size: usize) {
    let source = vec![0xA5_u8; size];
    bencher.bench_local(|| {
        let bytes = black_box(source.as_slice()).to_vec();
        black_box(&bytes);
        drop(bytes);
    });
}

// One raw ABI copy into a reused buffer. Allocation and root-length lookup are
// deliberately absent, unlike the safe owned-result benchmark below.
#[divan::bench(args = [0, 8192, 65536], sample_count = 40, sample_size = 25, threads = 1)]
fn rooted_reused_buffer_copy(bencher: Bencher<'_, '_>, size: usize) {
    let source = "A5".repeat(size);
    with_runtime(|runtime| {
        let root = runtime
            .bytevector_from_bytestring(&source, BytestringDelimiter::Compact)
            .expect("root untimed buffer-copy fixture");
        assert_eq!(root.len().into_result(), Ok(size));
        let mut output = vec![0; size];
        let mut copy = || {
            // SAFETY: this initialized owner retains the root for the entire
            // call and output is writable for the exact admitted root length.
            let status = unsafe {
                gerbil_scheme_sys::gerbil_scheme_rust_root_bytevector_copy(
                    root.root_id(),
                    output.as_mut_ptr(),
                    size,
                )
            };
            assert_eq!(status, gerbil_scheme_sys::GerbilStatus::Ok);
            black_box(&output);
        };
        copy();
        bencher.bench_local(&mut copy);
        assert_eq!(output, vec![0xA5; size]);
    });
}

#[divan::bench(args = [0, 8192, 65536], sample_count = 40, sample_size = 25, threads = 1)]
fn rooted_create_drop(bencher: Bencher<'_, '_>, size: usize) {
    let source = "A5".repeat(size);
    with_runtime(|runtime| {
        let fixture = runtime
            .bytevector_from_bytestring(&source, BytestringDelimiter::Compact)
            .expect("root create/drop preflight fixture");
        assert_eq!(fixture.to_vec().into_result(), Ok(vec![0xA5; size]));
        drop(fixture);
        bencher.bench_local(|| {
            let root = runtime
                .bytevector_from_bytestring(black_box(&source), BytestringDelimiter::Compact)
                .expect("Scheme decoding/allocation/rooting");
            black_box(root.root_id());
            drop(root);
        });
        assert_eq!(runtime.add_i64(40, 2).expect("usable after root churn"), 42);
    });
}

#[divan::bench(args = [0, 8192, 65536], sample_count = 40, sample_size = 25, threads = 1)]
fn rooted_bulk_copy(bencher: Bencher<'_, '_>, size: usize) {
    let source = "A5".repeat(size);
    with_runtime(|runtime| {
        let root = runtime
            .bytevector_from_bytestring(&source, BytestringDelimiter::Compact)
            .expect("create untimed bytevector fixture");
        assert_eq!(
            root.to_vec().into_result().expect("copy preflight"),
            vec![0xA5; size]
        );
        bencher.bench_local(|| {
            let bytes = root.to_vec().into_result().expect("copy rooted bytes");
            black_box(&bytes);
            // Dispose inside the timed closure, not in Divan's output drop.
            drop(bytes);
        });
    });
}

#[divan::bench(args = [0, 8192, 65536], sample_count = 40, sample_size = 25, threads = 1)]
fn rooted_round_trip(bencher: Bencher<'_, '_>, size: usize) {
    let source = "A5".repeat(size);
    with_runtime(|runtime| {
        let fixture = runtime
            .bytevector_from_bytestring(&source, BytestringDelimiter::Compact)
            .expect("create round-trip preflight fixture");
        assert_eq!(
            fixture
                .to_vec()
                .into_result()
                .expect("round-trip preflight"),
            vec![0xA5; size]
        );
        drop(fixture);
        bencher.bench_local(|| {
            let root = runtime
                .bytevector_from_bytestring(black_box(&source), BytestringDelimiter::Compact)
                .expect("allocate and root bytevector");
            let bytes = root.to_vec().into_result().expect("copy round-trip bytes");
            black_box(&bytes);
            // Include both Scheme root release and Rust result disposal.
            drop(root);
            drop(bytes);
        });
        assert_eq!(runtime.add_i64(40, 2).expect("owner remains usable"), 42);
    });
}
