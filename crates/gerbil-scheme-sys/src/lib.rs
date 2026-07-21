//! Raw C ABI bindings for the Gerbil Scheme runtime bridge.

mod abi;
mod abi_bytevector;
mod abi_integer_bytes;
mod abi_rooted_bytes;
mod abi_sentinel;
mod scheme_char;
mod scheme_fixnum;
mod scheme_flonum;

pub use abi::{
    GerbilBoolean, GerbilBorrowedBytevector, GerbilBorrowedVector, GerbilChar, GerbilFixnum,
    GerbilFlonum, GerbilPair, GerbilProcedureCallback,
};

pub use abi::{
    gerbil_scheme_rust_fixture_false, gerbil_scheme_rust_fixture_improper_list,
    gerbil_scheme_rust_fixture_null, gerbil_scheme_rust_fixture_pair,
    gerbil_scheme_rust_fixture_proper_list, gerbil_scheme_rust_fixture_true,
    gerbil_scheme_rust_pair_car, gerbil_scheme_rust_pair_cdr, gerbil_scheme_rust_pair_parts,
    gerbil_scheme_rust_runtime_sentinel_value, gerbil_scheme_rust_scheme_object_as_boolean,
    gerbil_scheme_rust_scheme_object_is_boolean, gerbil_scheme_rust_scheme_object_is_list,
    gerbil_scheme_rust_scheme_object_is_null, gerbil_scheme_rust_scheme_object_is_pair,
    gerbil_scheme_rust_scheme_object_pair_car, gerbil_scheme_rust_scheme_object_pair_cdr,
    gerbil_scheme_rust_scheme_object_pair_parts, gerbil_scheme_rust_value_is_list,
    gerbil_scheme_rust_value_is_null, gerbil_scheme_rust_value_is_pair,
};
pub use abi_bytevector::{
    gerbil_scheme_rust_fixture_bytevector, gerbil_scheme_rust_scheme_object_bytevector_length,
    gerbil_scheme_rust_scheme_object_bytevector_u8_ref,
    gerbil_scheme_rust_scheme_object_is_bytevector,
};
pub use abi_integer_bytes::{
    GERBIL_SCHEME_RUST_MAX_INTEGER_BYTES, GerbilByteOrder, gerbil_scheme_rust_bytevector_to_sint,
    gerbil_scheme_rust_bytevector_to_uint, gerbil_scheme_rust_root_bytevector_to_sint,
    gerbil_scheme_rust_root_bytevector_to_uint, gerbil_scheme_rust_sint_to_bytevector_root,
    gerbil_scheme_rust_uint_to_bytevector_root,
};
pub use abi_rooted_bytes::{
    GerbilRootId, gerbil_scheme_rust_bytestring_to_bytevector_root,
    gerbil_scheme_rust_bytevector_to_bytestring_root, gerbil_scheme_rust_root_bytevector_length,
    gerbil_scheme_rust_root_bytevector_u8_ref, gerbil_scheme_rust_root_release,
    gerbil_scheme_rust_root_string_char_ref, gerbil_scheme_rust_root_string_length,
};
pub use abi_sentinel::{gerbil_scheme_rust_fixture_void, gerbil_scheme_rust_scheme_object_is_void};

pub use abi::{
    GERBIL_SCHEME_RUST_ABI_ID, GERBIL_SCHEME_RUST_ABI_VERSION, GERBIL_SCHEME_RUST_HEADER_PATH,
    GERBIL_SCHEME_RUST_HEADER_SOURCE, GerbilBorrowedUtf8, GerbilI64Callback, GerbilRuntimeOpaque,
    GerbilStatus, GerbilValueHandle, gerbil_scheme_rust_abi_version, gerbil_scheme_rust_add_i64,
    gerbil_scheme_rust_compare_i64, gerbil_scheme_rust_identity_i64,
    gerbil_scheme_rust_is_even_i64, gerbil_scheme_rust_runtime_cleanup,
    gerbil_scheme_rust_runtime_init,
};

pub use scheme_char::{
    gerbil_scheme_rust_fixture_char_ascii, gerbil_scheme_rust_fixture_char_bmp,
    gerbil_scheme_rust_fixture_char_non_bmp, gerbil_scheme_rust_scheme_object_as_char,
    gerbil_scheme_rust_scheme_object_is_char,
};
pub use scheme_fixnum::{
    gerbil_scheme_rust_fixture_fixnum, gerbil_scheme_rust_scheme_object_as_fixnum,
    gerbil_scheme_rust_scheme_object_is_fixnum,
};
pub use scheme_flonum::{
    gerbil_scheme_rust_fixture_flonum_finite, gerbil_scheme_rust_fixture_flonum_nan,
    gerbil_scheme_rust_fixture_flonum_neg_inf, gerbil_scheme_rust_fixture_flonum_neg_zero,
    gerbil_scheme_rust_fixture_flonum_pos_inf, gerbil_scheme_rust_scheme_object_as_flonum,
    gerbil_scheme_rust_scheme_object_is_flonum,
};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
