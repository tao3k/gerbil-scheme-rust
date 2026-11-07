//! Binary transfer phases; no text conversion and no Scheme heap borrowing.

use super::with_runtime;
use divan::{Bencher, black_box};

#[divan::bench(args = [0, 8192, 65536], sample_count = 40, sample_size = 25, threads = 1)]
fn create_drop(bencher: Bencher<'_, '_>, size: usize) {
    let input = vec![0xA5; size];
    with_runtime(|runtime| {
        assert_eq!(
            runtime
                .bytevector_from_bytes(&input)
                .unwrap()
                .to_vec()
                .into_result(),
            Ok(input.clone())
        );
        bencher.bench_local(|| {
            let root = runtime
                .bytevector_from_bytes(black_box(&input))
                .expect("binary input");
            black_box(root.root_id());
            drop(root);
        });
    });
}

#[divan::bench(args = [0, 1, 8, 16, 17, 64, 65, 256, 8192, 65536], sample_count = 40, sample_size = 25, threads = 1)]
fn reused_output(bencher: Bencher<'_, '_>, size: usize) {
    reused_output_at_offset(bencher, size, 0);
}

#[divan::bench(args = [0, 1, 8, 16, 17, 64, 65, 256, 8192, 65536], sample_count = 40, sample_size = 25, threads = 1)]
fn reused_output_unaligned(bencher: Bencher<'_, '_>, size: usize) {
    reused_output_at_offset(bencher, size, 1);
}

fn reused_output_at_offset(bencher: Bencher<'_, '_>, size: usize, offset: usize) {
    let input: Vec<u8> = (0..=u8::MAX).cycle().take(size).collect();
    with_runtime(|runtime| {
        let root = runtime
            .bytevector_from_bytes(&input)
            .expect("untimed binary fixture");
        let mut storage = vec![0x5A; size + 2];
        let output = &mut storage[offset..offset + size];
        root.copy_into(output)
            .into_result()
            .expect("copy preflight");
        assert_eq!(output, input.as_slice());
        bencher.bench_local(|| {
            root.copy_into(black_box(&mut *output))
                .into_result()
                .expect("safe reused copy");
            black_box(&output);
        });
        assert_eq!(output, input.as_slice());
        assert!(storage[..offset].iter().all(|byte| *byte == 0x5A));
        assert!(storage[offset + size..].iter().all(|byte| *byte == 0x5A));
    });
}

#[divan::bench(args = [0, 8192, 65536], sample_count = 40, sample_size = 25, threads = 1)]
fn round_trip(bencher: Bencher<'_, '_>, size: usize) {
    let input = vec![0xA5; size];
    with_runtime(|runtime| {
        assert_eq!(
            runtime
                .bytevector_from_bytes(&input)
                .unwrap()
                .to_vec()
                .into_result(),
            Ok(input.clone())
        );
        bencher.bench_local(|| {
            let root = runtime
                .bytevector_from_bytes(black_box(&input))
                .expect("binary input");
            let output = root.to_vec().into_result().expect("owned binary result");
            black_box(&output);
            drop(root);
            drop(output);
        });
    });
}

// Same input allocation, Scheme fill, output copy and root release as
// round_trip, but storage belongs to this synchronous caller and is reused.
// This is NOT a retained-result transport substitute: callers that keep a
// response still need independent owned storage. Do not subtract independent
// medians to claim an isolated allocator cost.
#[divan::bench(args = [0, 8192, 65536], sample_count = 40, sample_size = 25, threads = 1)]
fn round_trip_reused_output(bencher: Bencher<'_, '_>, size: usize) {
    let input = vec![0xA5; size];
    let mut output = vec![0x5A; size];
    with_runtime(|runtime| {
        let mut transfer = || {
            let root = runtime
                .bytevector_from_bytes(black_box(&input))
                .expect("binary input");
            root.copy_into(black_box(&mut output))
                .into_result()
                .expect("caller-owned reused output");
            black_box(&output);
            drop(root);
        };
        transfer();
        bencher.bench_local(&mut transfer);
        assert_eq!(output, input);
    });
}
