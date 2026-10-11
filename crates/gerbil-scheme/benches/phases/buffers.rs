//! Actual owner-affine bytevector transfers; not queue or parser benchmarks.

use super::with_runtime;
use divan::{Bencher, black_box};

#[divan::bench(args = [1_000, 10_000, 100_000], consts = [64, 8192], sample_count = 20, sample_size = 1, threads = 1)]
fn allocated<const SIZE: usize>(bencher: Bencher<'_, '_>, jobs: usize) {
    batches::<SIZE>(bencher, jobs, false);
}

#[divan::bench(args = [1_000, 10_000, 100_000], consts = [64, 8192], sample_count = 20, sample_size = 1, threads = 1)]
fn reused<const SIZE: usize>(bencher: Bencher<'_, '_>, jobs: usize) {
    batches::<SIZE>(bencher, jobs, true);
}

#[divan::bench(args = [1_000, 10_000, 100_000], sample_count = 20, sample_size = 1, threads = 1)]
fn utf8_fresh(bencher: Bencher<'_, '_>, jobs: usize) {
    utf8_batches(bencher, jobs, false);
}

#[divan::bench(args = [1_000, 10_000, 100_000], sample_count = 20, sample_size = 1, threads = 1)]
fn utf8_snapshot(bencher: Bencher<'_, '_>, jobs: usize) {
    utf8_batches(bencher, jobs, true);
}

fn utf8_batches(bencher: Bencher<'_, '_>, jobs: usize, snapshot: bool) {
    with_runtime(|runtime| {
        let texts: Vec<_> = ["a\0汉字😀", "b\0漢語🚀", "c\0中文🌍", "d\0文字🎉"]
            .into_iter()
            .map(|text| text.repeat(683))
            .collect();
        let values: Vec<_> = texts
            .iter()
            .map(|text| runtime.string_from_utf8(text).unwrap())
            .collect();
        let expected = &texts[0];
        let value = &values[0];
        // Encoding is explicitly outside the reusable snapshot's timed body.
        // Fresh conversion includes encoding; both produce validated String.
        let encoded = value.to_utf8_bytes().into_result().unwrap();
        let mut output = Vec::with_capacity(expected.len());
        encoded.copy_to_vec(&mut output).into_result().unwrap();
        assert_eq!(output, expected.as_bytes());
        assert_eq!(&value.to_string().into_result().unwrap(), expected);
        for (value, expected) in values.iter().zip(&texts) {
            assert_eq!(&value.to_string().into_result().unwrap(), expected);
        }
        let pointer = output.as_ptr();
        let mut completed = 0;
        eprintln!(
            "UTF8-LOAD jobs={jobs} bytes={} snapshot={snapshot} scheme-owners=1",
            expected.len()
        );
        bencher.bench_local(|| {
            let mut total = 0;
            for index in 0..jobs {
                if snapshot {
                    encoded.copy_to_vec(&mut output).into_result().unwrap();
                    let text = String::from_utf8(std::mem::take(&mut output)).unwrap();
                    total += black_box(&text).len();
                    output = text.into_bytes();
                } else {
                    let text = values[index % values.len()]
                        .to_string()
                        .into_result()
                        .unwrap();
                    assert_eq!(&text, &texts[index % texts.len()]);
                    total += black_box(text).len();
                }
            }
            assert_eq!(total, jobs * expected.len());
            if snapshot {
                assert_eq!(output.as_ptr(), pointer);
            }
            completed += 1;
            eprintln!("UTF8-SAMPLE jobs={jobs} snapshot={snapshot} completed-batches={completed}");
            black_box(total)
        });
    });
}

fn batches<const SIZE: usize>(bencher: Bencher<'_, '_>, jobs: usize, reuse: bool) {
    with_runtime(|runtime| {
        // Identical varying-length workload for both policies. Roots stay live
        // on this owner; output is owned Rust memory, never a Scheme heap view.
        let expected: Vec<Vec<u8>> = [0, 1, SIZE / 2, SIZE]
            .into_iter()
            .map(|length| (0..=u8::MAX).cycle().take(length).collect())
            .collect();
        let values: Vec<_> = expected
            .iter()
            .map(|bytes| runtime.bytevector_from_bytes(bytes).unwrap())
            .collect();
        let mut output = Vec::with_capacity(SIZE);
        let pointer = output.as_ptr();
        for (value, bytes) in values.iter().zip(&expected) {
            value.copy_to_vec(&mut output).into_result().unwrap();
            assert_eq!(&output, bytes);
            assert_eq!(value.to_vec().into_result().unwrap(), *bytes);
            assert_eq!(output.as_ptr(), pointer);
        }
        let mut completed = 0;
        eprintln!("BUFFER-LOAD jobs={jobs} max-bytes={SIZE} reuse={reuse} scheme-owners=1");
        bencher.bench_local(|| {
            let mut total = 0;
            for index in 0..jobs {
                let value = &values[index % values.len()];
                if reuse {
                    value.copy_to_vec(&mut output).into_result().unwrap();
                    total += black_box(&output).len();
                } else {
                    total += black_box(value.to_vec().into_result().unwrap()).len();
                }
            }
            let cycle_bytes = 1 + SIZE / 2 + SIZE;
            assert_eq!(total, jobs / 4 * cycle_bytes);
            if reuse { assert_eq!(output.as_ptr(), pointer); }
            completed += 1;
            eprintln!("BUFFER-SAMPLE jobs={jobs} max-bytes={SIZE} reuse={reuse} completed-batches={completed}");
            black_box(total)
        });
    });
}
