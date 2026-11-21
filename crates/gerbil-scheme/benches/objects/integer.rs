//! Byte-wise executable reference versus limb-wise binary transfer.
use gerbil_scheme_sys::{self as sys, GerbilRootId, GerbilStatus};
unsafe extern "C" {
    fn gerbil_scheme_rust_bigint_create_byte_control(
        bytes: *const u8,
        len: u64,
        negative: i32,
    ) -> i64;
    fn gerbil_scheme_rust_bigint_copy_byte_control(root: i64, bytes: *mut u8, len: u64) -> i32;
}
struct Root(i64);
impl Drop for Root {
    fn drop(&mut self) {
        assert_eq!(
            unsafe { sys::gerbil_scheme_rust_root_release(GerbilRootId(self.0)) },
            GerbilStatus::Ok
        );
    }
}
pub(super) fn run() -> bool {
    let mut admitted = true;
    for (size, name) in [
        (1, "integer-1-byte"),
        (32, "integer-32-byte"),
        (256, "integer-256-byte"),
    ] {
        let sources: Vec<Vec<u8>> = (1..=5)
            .map(|seed| (1..=u8::MAX).cycle().skip(seed).take(size).collect())
            .collect();
        for requests in [1000, 10_000, 100_000] {
            let mut output = vec![0_u8; size];
            admitted &= super::measure(name, requests, [output.capacity(); 2], |mode, index| {
                let source = &sources[index % sources.len()];
                let negative = i32::try_from(index % 2).unwrap();
                // SAFETY: same live owner, canonical bytes, disjoint Rust output.
                let root = unsafe {
                    if mode == 0 {
                        gerbil_scheme_rust_bigint_create_byte_control(
                            source.as_ptr(),
                            size as u64,
                            negative,
                        )
                    } else {
                        sys::gerbil_scheme_rust_bigint_create_raw(
                            source.as_ptr(),
                            size as u64,
                            negative,
                        )
                    }
                };
                assert!(root > 0);
                let root = Root(root);
                assert_eq!(
                    unsafe { sys::gerbil_scheme_rust_bigint_size_raw(root.0) },
                    i64::try_from(size).unwrap()
                );
                let sign = unsafe {
                    if mode == 0 {
                        gerbil_scheme_rust_bigint_copy_byte_control(
                            root.0,
                            output.as_mut_ptr(),
                            size as u64,
                        )
                    } else {
                        sys::gerbil_scheme_rust_bigint_copy_raw(
                            root.0,
                            output.as_mut_ptr(),
                            size as u64,
                        )
                    }
                };
                assert_eq!(sign, negative);
                assert_eq!(&output, source);
            });
        }
    }
    admitted
}
