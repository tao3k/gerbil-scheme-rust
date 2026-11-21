//! Matched whole-value conversion controls. Scalar loops are benchmark-only.

use super::with_runtime;
use divan::{Bencher, black_box};

#[divan::bench(args = [0, 8192, 65536], sample_count = 20, sample_size = 5, threads = 1)]
fn bytes_bulk(bencher: Bencher<'_, '_>, size: usize) {
    with_runtime(|runtime| {
        let expected = vec![0xA5; size];
        let value = runtime.bytevector_from_bytes(&expected).unwrap();
        assert_eq!(value.to_vec().into_result().unwrap(), expected);
        bencher.bench_local(|| black_box(value.to_vec().into_result().unwrap()));
    });
}

#[divan::bench(args = [0, 8192, 65536], sample_count = 20, sample_size = 5, threads = 1)]
fn bytes_scalar_control(bencher: Bencher<'_, '_>, size: usize) {
    with_runtime(|runtime| {
        let expected = vec![0xA5; size];
        let value = runtime.bytevector_from_bytes(&expected).unwrap();
        let copy = || {
            let length = value.len().into_result().unwrap();
            let mut bytes = Vec::with_capacity(length);
            for index in 0..length {
                bytes.push(value.u8_at(index).into_result().unwrap());
            }
            bytes
        };
        assert_eq!(copy(), expected);
        bencher.bench_local(|| black_box(copy()));
    });
}

#[divan::bench(args = [0, 8192, 65536], sample_count = 20, sample_size = 5, threads = 1)]
fn string_bulk(bencher: Bencher<'_, '_>, ascii_bytes: usize) {
    with_runtime(|runtime| {
        let expected = "a".repeat(ascii_bytes) + "汉字😀\0";
        let value = runtime.string_from_utf8(&expected).unwrap();
        assert_eq!(value.to_string().into_result().unwrap(), expected);
        bencher.bench_local(|| black_box(value.to_string().into_result().unwrap()));
    });
}

#[divan::bench(args = [0, 8192, 65536], sample_count = 20, sample_size = 5, threads = 1)]
fn string_scalar_control(bencher: Bencher<'_, '_>, ascii_bytes: usize) {
    with_runtime(|runtime| {
        let expected = "a".repeat(ascii_bytes) + "汉字😀\0";
        let value = runtime.string_from_utf8(&expected).unwrap();
        let copy = || {
            let length = value.len().into_result().unwrap();
            let mut text = String::with_capacity(length);
            for index in 0..length {
                text.push(value.char_at(index).into_result().unwrap());
            }
            text
        };
        assert_eq!(copy(), expected);
        bencher.bench_local(|| black_box(copy()));
    });
}
