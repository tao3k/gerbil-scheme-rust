use core::ffi::c_char;

use crate::{
    GERBIL_SCHEME_RUST_ABI_ID, GERBIL_SCHEME_RUST_ABI_VERSION, GerbilBorrowedUtf8, GerbilStatus,
};

use crate::{
    GerbilBoolean, GerbilBorrowedBytevector, GerbilBorrowedVector, GerbilChar, GerbilFixnum,
    GerbilFlonum, GerbilPair, GerbilProcedureCallback, GerbilValueHandle,
    gerbil_scheme_rust_pair_car, gerbil_scheme_rust_pair_cdr, gerbil_scheme_rust_pair_parts,
    gerbil_scheme_rust_value_is_list, gerbil_scheme_rust_value_is_null,
    gerbil_scheme_rust_value_is_pair,
};

mod abi_integer_bytes;
mod scenario_benchmark_suite;
mod scheme_exact_integer;

#[test]
fn abi_identity_is_nul_terminated() {
    assert_eq!(GERBIL_SCHEME_RUST_ABI_ID.last(), Some(&0));
    assert_eq!(GERBIL_SCHEME_RUST_ABI_VERSION, 1);
}

#[test]
fn public_header_matches_the_live_scalar_abi() {
    let header = crate::GERBIL_SCHEME_RUST_HEADER_SOURCE;

    assert!(header.contains("#define GERBIL_SCHEME_RUST_ABI_VERSION 1u"));
    assert!(header.contains("GERBIL_STATUS_ABI_MISMATCH = 2"));
    assert!(header.contains("int64_t gerbil_scheme_rust_add_i64(int64_t left, int64_t right);"));
    assert!(header.contains("int32_t gerbil_scheme_rust_is_even_i64(int64_t value);"));
    assert!(
        header.contains("int32_t gerbil_scheme_rust_compare_i64(int64_t left, int64_t right);")
    );
    assert!(header.contains("GERBIL_BYTE_ORDER_NATIVE = 2"));
    assert!(header.contains("gerbil_scheme_rust_bytevector_to_uint("));
    assert!(header.contains("gerbil_scheme_rust_bytevector_to_sint("));
    assert!(header.contains("gerbil_scheme_rust_uint_to_bytevector_root("));
    assert!(header.contains("gerbil_scheme_rust_sint_to_bytevector_root("));
    assert!(header.contains("gerbil_scheme_rust_scheme_object_is_exact_integer("));
    assert!(header.contains("gerbil_scheme_rust_scheme_object_exact_integer_to_i64("));
    assert!(header.contains("gerbil_scheme_rust_scheme_object_exact_integer_to_u64("));
    assert!(header.contains("gerbil_scheme_rust_i64_to_exact_integer_root("));
    assert!(header.contains("gerbil_scheme_rust_u64_to_exact_integer_root("));
    assert!(header.contains("gerbil_scheme_rust_root_exact_integer_to_i64("));
    assert!(header.contains("gerbil_scheme_rust_root_exact_integer_to_u64("));
    assert!(!header.contains("int64_t *result"));
}

#[test]
fn status_values_are_stable() {
    assert_eq!(GerbilStatus::Ok as i32, 0);
    assert_eq!(GerbilStatus::Panic as i32, 5);
    assert_eq!(GerbilStatus::AlreadyInitialized as i32, 6);
    assert_eq!(GerbilStatus::NotInitialized as i32, 7);
    assert_eq!(GerbilStatus::RuntimeFinalized as i32, 8);
}

#[test]
fn borrowed_utf8_matches_pointer_and_length_layout() {
    assert_eq!(
        core::mem::size_of::<GerbilBorrowedUtf8>(),
        core::mem::size_of::<*const c_char>() + core::mem::size_of::<usize>()
    );
}

#[test]
fn borrowed_utf8_constructors_preserve_rust_string_bytes() {
    let empty = GerbilBorrowedUtf8::empty();
    assert!(empty.is_empty());
    assert!(empty.ptr.is_null());
    assert_eq!(empty.len, 0);
    // SAFETY: empty() guarantees null with zero length.
    assert_eq!(unsafe { empty.as_bytes() }, b"");

    let text = "hello λ";
    let borrowed = GerbilBorrowedUtf8::from(text);
    assert!(!borrowed.is_empty());
    assert_eq!(borrowed.len, text.len());
    assert_eq!(borrowed.ptr.cast::<u8>(), text.as_ptr());
    // SAFETY: `borrowed` points into `text`, which is still alive here.
    assert_eq!(unsafe { borrowed.as_bytes() }, text.as_bytes());
    // SAFETY: `borrowed` points into valid UTF-8 owned by `text`.
    assert_eq!(unsafe { borrowed.as_str() }, Ok(text));
}

#[test]
fn borrowed_utf8_as_str_rejects_invalid_utf8() {
    let bytes = [0xff_u8];
    let borrowed = GerbilBorrowedUtf8 {
        ptr: bytes.as_ptr().cast(),
        len: bytes.len(),
    };

    // SAFETY: `borrowed` points into `bytes`, which is alive for this call.
    assert!(unsafe { borrowed.as_str() }.is_err());
}

#[test]
fn scalar_scheme_value_wrappers_have_stable_c_abi_shapes() {
    assert_eq!(
        core::mem::size_of::<GerbilFixnum>(),
        core::mem::size_of::<isize>()
    );
    assert_eq!(GerbilFixnum(-42), GerbilFixnum(-42));

    assert_eq!(GerbilBoolean::from_bool(false), GerbilBoolean::FALSE);
    assert_eq!(GerbilBoolean::from_bool(true), GerbilBoolean::TRUE);
    assert!(!GerbilBoolean::FALSE.as_bool());
    assert!(GerbilBoolean::TRUE.as_bool());
    assert!(GerbilBoolean(7).as_bool());

    let lambda = GerbilChar::from_char('λ');
    assert_eq!(char::try_from(lambda), Ok('λ'));
    assert!(char::try_from(GerbilChar(0xD800)).is_err());

    assert_eq!(GerbilFlonum(1.25), GerbilFlonum(1.25));
}

#[test]
fn borrowed_collection_surfaces_preserve_pointer_and_length_contracts() {
    let bytes = [1_u8, 2, 3];
    let borrowed_bytes = GerbilBorrowedBytevector::from_slice(&bytes);
    assert_eq!(borrowed_bytes.ptr, bytes.as_ptr());
    assert_eq!(borrowed_bytes.len, bytes.len());
    assert!(GerbilBorrowedBytevector::EMPTY.ptr.is_null());
    assert_eq!(GerbilBorrowedBytevector::EMPTY.len, 0);

    let mut left = 1_u8;
    let mut right = 2_u8;
    let handles = [(&raw mut left).addr(), (&raw mut right).addr()];
    let pair = GerbilPair {
        car: handles[0],
        cdr: handles[1],
    };
    assert_eq!(pair.car, handles[0]);
    assert_eq!(pair.cdr, handles[1]);

    let vector = GerbilBorrowedVector::from_slice(&handles);
    assert_eq!(vector.ptr, handles.as_ptr());
    assert_eq!(vector.len, handles.len());
    assert!(GerbilBorrowedVector::EMPTY.ptr.is_null());
    assert_eq!(GerbilBorrowedVector::EMPTY.len, 0);
}

#[test]
fn procedure_callback_type_projects_value_handle_status_boundary() {
    unsafe extern "C" fn callback(
        value: GerbilValueHandle,
        context: *mut core::ffi::c_void,
    ) -> GerbilStatus {
        if value == 0 || context.is_null() {
            GerbilStatus::NullPointer
        } else {
            GerbilStatus::Ok
        }
    }

    let callback: GerbilProcedureCallback = callback;
    let mut value = 1_u8;
    let mut context = 2_u8;

    // SAFETY: both pointers are derived from live stack values for this call.
    let status = unsafe {
        callback(
            (&raw mut value).addr(),
            (&raw mut context).cast::<core::ffi::c_void>(),
        )
    };
    assert_eq!(status, GerbilStatus::Ok);

    // SAFETY: null pointers are intentionally used to validate fail-closed status projection.
    let status = unsafe { callback(0, core::ptr::null_mut()) };
    assert_eq!(status, GerbilStatus::NullPointer);
}

#[test]
fn pair_and_list_status_abi_fails_closed_until_runtime_backed() {
    let mut value = 1_u8;
    let handle = (&raw mut value).addr();
    let mut predicate = GerbilBoolean::TRUE;
    let mut projected = handle;
    let mut pair = GerbilPair {
        car: handle,
        cdr: handle,
    };

    // SAFETY: output pointers are valid for one value each.
    assert_eq!(
        unsafe { gerbil_scheme_rust_value_is_pair(handle, &raw mut predicate) },
        GerbilStatus::InvalidValue
    );
    assert_eq!(predicate, GerbilBoolean::FALSE);

    predicate = GerbilBoolean::TRUE;
    // SAFETY: output pointers are valid for one value each.
    assert_eq!(
        unsafe { gerbil_scheme_rust_value_is_list(handle, &raw mut predicate) },
        GerbilStatus::InvalidValue
    );
    assert_eq!(predicate, GerbilBoolean::FALSE);

    predicate = GerbilBoolean::TRUE;
    // SAFETY: output pointers are valid for one value each.
    assert_eq!(
        unsafe { gerbil_scheme_rust_value_is_null(handle, &raw mut predicate) },
        GerbilStatus::InvalidValue
    );
    assert_eq!(predicate, GerbilBoolean::FALSE);

    // SAFETY: output pointers are valid for one value each.
    assert_eq!(
        unsafe { gerbil_scheme_rust_pair_car(handle, &raw mut projected) },
        GerbilStatus::InvalidValue
    );
    assert_eq!(projected, 0);

    projected = handle;
    // SAFETY: output pointers are valid for one value each.
    assert_eq!(
        unsafe { gerbil_scheme_rust_pair_cdr(handle, &raw mut projected) },
        GerbilStatus::InvalidValue
    );
    assert_eq!(projected, 0);

    // SAFETY: output pointers are valid for one value each.
    assert_eq!(
        unsafe { gerbil_scheme_rust_pair_parts(handle, &raw mut pair) },
        GerbilStatus::InvalidValue
    );
    assert_eq!(pair.car, handle);
    assert_eq!(pair.cdr, handle);
}

#[test]
fn pair_and_list_status_abi_rejects_null_inputs() {
    let mut predicate = GerbilBoolean::FALSE;
    let mut projected = 0;
    let mut pair = GerbilPair { car: 0, cdr: 0 };
    let mut value = 1_u8;
    let handle = (&raw mut value).addr();

    // SAFETY: null inputs are intentional fail-closed boundary checks.
    assert_eq!(
        unsafe { gerbil_scheme_rust_value_is_pair(0, &raw mut predicate) },
        GerbilStatus::NullPointer
    );
    // SAFETY: null inputs are intentional fail-closed boundary checks.
    assert_eq!(
        unsafe { gerbil_scheme_rust_value_is_list(handle, core::ptr::null_mut()) },
        GerbilStatus::NullPointer
    );
    // SAFETY: null inputs are intentional fail-closed boundary checks.
    assert_eq!(
        unsafe { gerbil_scheme_rust_value_is_null(handle, core::ptr::null_mut()) },
        GerbilStatus::NullPointer
    );
    // SAFETY: null inputs are intentional fail-closed boundary checks.
    assert_eq!(
        unsafe { gerbil_scheme_rust_pair_car(0, &raw mut projected) },
        GerbilStatus::NullPointer
    );
    // SAFETY: null inputs are intentional fail-closed boundary checks.
    assert_eq!(
        unsafe { gerbil_scheme_rust_pair_cdr(handle, core::ptr::null_mut()) },
        GerbilStatus::NullPointer
    );
    // SAFETY: null inputs are intentional fail-closed boundary checks.
    assert_eq!(
        unsafe { gerbil_scheme_rust_pair_parts(handle, core::ptr::null_mut()) },
        GerbilStatus::NullPointer
    );
    // SAFETY: output pointer is valid for one pair.
    assert_eq!(
        unsafe { gerbil_scheme_rust_pair_parts(0, &raw mut pair) },
        GerbilStatus::NullPointer
    );
}
