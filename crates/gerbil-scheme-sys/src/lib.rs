//! Raw C ABI bindings for the Gerbil Scheme runtime bridge.

mod abi;
mod scheme_fixnum;

pub use abi::gerbil_scheme_rust_fixture_false;
pub use abi::gerbil_scheme_rust_fixture_improper_list;
pub use abi::gerbil_scheme_rust_fixture_pair;
pub use abi::gerbil_scheme_rust_fixture_proper_list;
pub use abi::gerbil_scheme_rust_fixture_true;
pub use abi::gerbil_scheme_rust_scheme_object_as_boolean;
pub use abi::gerbil_scheme_rust_scheme_object_is_boolean;
pub use abi::gerbil_scheme_rust_scheme_object_is_list;
pub use abi::gerbil_scheme_rust_scheme_object_is_null;
pub use abi::gerbil_scheme_rust_scheme_object_is_pair;
pub use abi::gerbil_scheme_rust_scheme_object_pair_car;
pub use abi::gerbil_scheme_rust_scheme_object_pair_cdr;
pub use abi::gerbil_scheme_rust_scheme_object_pair_parts;
pub use scheme_fixnum::gerbil_scheme_rust_fixture_fixnum;
pub use scheme_fixnum::gerbil_scheme_rust_scheme_object_as_fixnum;
pub use scheme_fixnum::gerbil_scheme_rust_scheme_object_is_fixnum;

pub use abi::{
    GerbilBoolean, GerbilBorrowedBytevector, GerbilBorrowedVector, GerbilChar, GerbilFixnum,
    GerbilFlonum, GerbilPair, GerbilProcedureCallback,
};

pub use abi::{
    gerbil_scheme_rust_fixture_null, gerbil_scheme_rust_pair_car, gerbil_scheme_rust_pair_cdr,
    gerbil_scheme_rust_pair_parts, gerbil_scheme_rust_runtime_sentinel_value,
    gerbil_scheme_rust_value_is_list, gerbil_scheme_rust_value_is_null,
    gerbil_scheme_rust_value_is_pair,
};

pub use abi::{
    GERBIL_SCHEME_RUST_ABI_ID, GERBIL_SCHEME_RUST_ABI_VERSION, GERBIL_SCHEME_RUST_HEADER_PATH,
    GERBIL_SCHEME_RUST_HEADER_SOURCE, GerbilBorrowedUtf8, GerbilI64Callback, GerbilRuntimeOpaque,
    GerbilStatus, GerbilValueHandle, gerbil_scheme_rust_abi_version, gerbil_scheme_rust_add_i64,
    gerbil_scheme_rust_compare_i64, gerbil_scheme_rust_identity_i64,
    gerbil_scheme_rust_is_even_i64, gerbil_scheme_rust_runtime_cleanup,
    gerbil_scheme_rust_runtime_init,
};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
