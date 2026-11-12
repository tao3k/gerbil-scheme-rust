//! Phase controls for changing text; no parser or parallelism admission.

use super::with_runtime;
use divan::{Bencher, black_box};

#[divan::bench(args = [1_000, 10_000, 100_000], sample_count = 20, sample_size = 1, threads = 1)]
fn encode_and_root(bencher: Bencher<'_, '_>, jobs: usize) {
    batches::<0>(bencher, jobs);
}

#[divan::bench(args = [1_000, 10_000, 100_000], sample_count = 20, sample_size = 1, threads = 1)]
fn copy_owned(bencher: Bencher<'_, '_>, jobs: usize) {
    batches::<1>(bencher, jobs);
}

#[divan::bench(args = [1_000, 10_000, 100_000], sample_count = 20, sample_size = 1, threads = 1)]
fn validate_and_compare(bencher: Bencher<'_, '_>, jobs: usize) {
    batches::<2>(bencher, jobs);
}

#[divan::bench(args = [1_000, 10_000, 100_000], sample_count = 20, sample_size = 1, threads = 1)]
fn validate_simd_and_compare(bencher: Bencher<'_, '_>, jobs: usize) {
    batches::<4>(bencher, jobs);
}

#[divan::bench(args = [1_000, 10_000, 100_000], sample_count = 20, sample_size = 1, threads = 1)]
fn fresh_reused_output(bencher: Bencher<'_, '_>, jobs: usize) {
    batches::<3>(bencher, jobs);
}

fn batches<const PHASE: usize>(bencher: Bencher<'_, '_>, jobs: usize) {
    with_runtime(|runtime| {
        let texts: Vec<_> = ["a\0汉字😀", "b\0漢語🚀", "c\0中文🌍", "d\0文字🎉"]
            .into_iter()
            .map(|text| text.repeat(683))
            .collect();
        let values: Vec<_> = texts
            .iter()
            .map(|text| runtime.string_from_utf8(text).unwrap())
            .collect();
        // Pre-encoded values are phase controls only. Fresh phases always
        // encode the current string; no memoized bytevector substitutes for it.
        let encoded: Vec<_> = values
            .iter()
            .map(|value| value.to_utf8_bytes().into_result().unwrap())
            .collect();
        for (value, text) in encoded.iter().zip(&texts) {
            assert_eq!(value.to_vec().into_result().unwrap(), text.as_bytes());
        }
        let mut output = Vec::with_capacity(texts[0].len());
        let pointer = output.as_ptr();
        let mut completed = 0;
        bencher.bench_local(|| {
            let mut total = 0;
            for index in 0..jobs {
                let slot = index % texts.len();
                let expected = &texts[slot];
                match PHASE {
                    0 => {
                        let bytes = values[slot].to_utf8_bytes().into_result().unwrap();
                        let length = bytes.len().into_result().unwrap();
                        assert_eq!(length, expected.len());
                        total += black_box(length);
                    }
                    1 => {
                        let bytes = encoded[slot].to_vec().into_result().unwrap();
                        assert_eq!(bytes, expected.as_bytes());
                        total += black_box(bytes).len();
                    }
                    2 => {
                        let text = std::str::from_utf8(black_box(expected.as_bytes())).unwrap();
                        assert_eq!(text, expected);
                        total += black_box(text).len();
                    }
                    3 => {
                        let bytes = values[slot].to_utf8_bytes().into_result().unwrap();
                        bytes.copy_to_vec(&mut output).into_result().unwrap();
                        let text = String::from_utf8(std::mem::take(&mut output)).unwrap();
                        assert_eq!(&text, expected);
                        total += black_box(&text).len();
                        output = text.into_bytes();
                    }
                    4 => {
                        let text =
                            simdutf8::basic::from_utf8(black_box(expected.as_bytes())).unwrap();
                        assert_eq!(text, expected);
                        total += black_box(text).len();
                    }
                    _ => unreachable!(),
                }
            }
            assert_eq!(total, jobs * texts[0].len());
            if PHASE == 3 {
                assert_eq!(output.as_ptr(), pointer);
            }
            completed += 1;
            // One real completion event per sample, not per transfer/chunk.
            eprintln!("UTF8-PHASE phase={PHASE} jobs={jobs} completed-batches={completed}");
            black_box(total)
        });
    });
}
