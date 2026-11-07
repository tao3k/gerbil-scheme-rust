//! Matched native element-copy versus the public whole-vector projection.
use gerbil_scheme::{GerbilRuntime, NumericBuffer, NumericElement};
use gerbil_scheme_sys as sys;
use std::hint::black_box;

fn family<T: NumericElement>(runtime: &GerbilRuntime, name: &str, rows: &[Vec<T>]) -> bool {
    fn bytes<T: NumericElement>(data: &[T]) -> &[u8] {
        unsafe { std::slice::from_raw_parts(data.as_ptr().cast(), std::mem::size_of_val(data)) }
    }
    let roots: Vec<_> = rows
        .iter()
        .map(|row| runtime.numeric_vector(row).unwrap())
        .collect();
    let raw: Vec<_> = rows
        .iter()
        .map(|row| unsafe {
            sys::gerbil_scheme_rust_numeric_create(T::KIND, row.as_ptr().cast(), row.len() as u64)
        })
        .collect();
    assert!(raw.iter().all(|root| *root > 0));
    let mut scalar = vec![T::default(); rows[0].len()];
    let mut bulk = NumericBuffer::<T>::default();
    roots[0].as_ref().copy_numeric(&mut bulk).unwrap();
    roots[0].as_ref().copy_numeric(&mut bulk).unwrap();
    let memory = [
        scalar.capacity() * std::mem::size_of::<T>(),
        bulk.reserved_bytes(),
    ];
    let mut pass = true;
    for load in [1_000, 10_000, 100_000] {
        pass &= super::measure(name, load, memory, |mode, request| {
            let slot = request % rows.len();
            if mode == 0 {
                for (index, output) in scalar.iter_mut().enumerate() {
                    assert_eq!(
                        unsafe {
                            sys::gerbil_scheme_rust_numeric_copy(
                                raw[slot],
                                -1,
                                T::KIND,
                                std::ptr::from_mut(output).cast(),
                                index as u64,
                                1,
                            )
                        },
                        0
                    );
                }
                assert_eq!(bytes(&scalar), bytes(&rows[slot]));
                black_box(&scalar);
            } else {
                roots[slot].as_ref().copy_numeric(&mut bulk).unwrap();
                assert_eq!(bytes(bulk.as_slice()), bytes(&rows[slot]));
                black_box(&bulk);
            }
        });
    }
    for root in raw {
        assert_eq!(
            unsafe { sys::gerbil_scheme_rust_root_release(sys::GerbilRootId(root)) },
            sys::GerbilStatus::Ok
        );
    }
    pass
}
pub(super) fn run(runtime: &GerbilRuntime) -> bool {
    if std::env::args().any(|arg| arg == "--signed") {
        let i8s: Vec<_> = (0..5_i8)
            .map(|n| (0..32_i8).map(|i| i8::MIN + i + n).collect())
            .collect();
        let i16s: Vec<_> = (0..5_i16)
            .map(|n| (0..32_i16).map(|i| i16::MIN + i + n).collect())
            .collect();
        let i32s: Vec<_> = (0..5_i32)
            .map(|n| (0..32_i32).map(|i| i32::MIN + i + n).collect())
            .collect();
        let i64s: Vec<_> = (0..5_i64)
            .map(|n| (0..32_i64).map(|i| i64::MIN + i + n).collect())
            .collect();
        return family(runtime, "s8vector32", &i8s)
            & family(runtime, "s16vector32", &i16s)
            & family(runtime, "s32vector32", &i32s)
            & family(runtime, "s64vector32", &i64s);
    }
    let u16s: Vec<_> = (0..5_u16)
        .map(|n| (0..32_u16).map(|i| u16::MAX - i - n).collect())
        .collect();
    let u32s: Vec<_> = (0..5_u32)
        .map(|n| (0..32_u32).map(|i| u32::MAX - i - n).collect())
        .collect();
    let u64s: Vec<_> = (0..5_u64)
        .map(|n| (0..32_u64).map(|i| u64::MAX - i - n).collect())
        .collect();
    let f32s: Vec<_> = (0..5_u32)
        .map(|n| {
            (0..32_u32)
                .map(|i| f32::from_bits(0x7fc0_1000 + i + n))
                .collect()
        })
        .collect();
    let f64s: Vec<_> = (0..5_u64)
        .map(|n| {
            (0..32_u64)
                .map(|i| f64::from_bits(0x7ff8_0000_0000_1000 + i + n))
                .collect()
        })
        .collect();
    family(runtime, "u16vector32", &u16s)
        & family(runtime, "u32vector32", &u32s)
        & family(runtime, "u64vector32", &u64s)
        & family(runtime, "f32vector32", &f32s)
        & family(runtime, "f64vector32", &f64s)
}
