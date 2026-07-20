//! Raw C ABI bindings for the Gerbil Scheme runtime bridge.

mod abi;

pub use abi::{
    GerbilBoolean, GerbilBorrowedBytevector, GerbilBorrowedVector, GerbilChar, GerbilFixnum,
    GerbilFlonum, GerbilPair, GerbilProcedureCallback,
};

pub use abi::{
    gerbil_scheme_rust_pair_car, gerbil_scheme_rust_pair_cdr, gerbil_scheme_rust_pair_parts,
    gerbil_scheme_rust_runtime_sentinel_value, gerbil_scheme_rust_value_is_list,
    gerbil_scheme_rust_value_is_null, gerbil_scheme_rust_value_is_pair,
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
