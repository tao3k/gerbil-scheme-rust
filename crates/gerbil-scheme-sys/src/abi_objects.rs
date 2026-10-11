//! Owner-local strong roots, collection spans and signed-magnitude integers.
use crate::GerbilRootId;
unsafe extern "C" {
    /// # Safety
    /// Live owner and string root; kind 0 interned symbol/1 keyword/2 uninterned
    /// symbol. Transfers one root; spelling alone is never an identity token.
    pub fn gerbil_scheme_rust_identifier_create(string_root: i64, kind: i32) -> i64;
    /// # Safety
    /// Two live roots on the same owner; -1 selects root, otherwise vector slot.
    pub fn gerbil_scheme_rust_object_identical(left: i64, li: i64, right: i64, ri: i64) -> i32;
    /// # Safety
    /// Live owner/root. Index -1 selects root or vector snapshot; otherwise
    /// one slot. Writable two-u64 metadata span. Positive result owns a packet.
    pub fn gerbil_scheme_rust_identifiers_prepare(root: i64, index: i64, metadata: *mut u64)
    -> i64;
    /// # Safety
    /// Owner-local prepared packet only; disjoint writable byte capacity,
    /// count+1 u64 offsets, count u8 tags matching the prepared metadata.
    /// The packet root stays live throughout encoding. Negative result is failure.
    pub fn gerbil_scheme_rust_identifiers_copy(
        root: i64,
        bytes: *mut u8,
        capacity: u64,
        offsets: *mut u64,
        kinds: *mut u8,
        count: u64,
    ) -> i64;
    /// # Safety
    /// Owner-local call; kind 0..8 names u16/u32/u64/f32/f64/i16/i32/i64/i8. Readable native
    /// element bytes, disjoint from Scheme storage. Positive root transfers ownership.
    pub fn gerbil_scheme_rust_numeric_create(kind: i32, bytes: *const u8, len: u64) -> i64;
    /// # Safety
    /// Live owner/root; index -1 selects root, otherwise its snapshot slot.
    pub fn gerbil_scheme_rust_numeric_size(root: i64, index: i64, kind: i32) -> i64;
    /// # Safety
    /// Live owner/root; writable disjoint native element span. Range/type errors
    /// publish nothing. Body pointers never escape the nonallocating leaf.
    pub fn gerbil_scheme_rust_numeric_copy(
        root: i64,
        index: i64,
        kind: i32,
        bytes: *mut u8,
        offset: u64,
        len: u64,
    ) -> i64;
    /// # Safety
    /// Live owner and root; index -1 projects the root, otherwise a vector slot.
    /// Output must be valid and disjoint. Invalid indices publish nothing.
    pub fn gerbil_scheme_rust_object_scalar_indexed(
        root: i64,
        index: i64,
        output: *mut crate::GerbilScalarProjection,
    ) -> i32;
    /// # Safety
    /// Live owner and root, shape 0/1/2. Positive result transfers one snapshot
    /// root retaining the complete shallow element snapshot; zero is failure.
    pub fn gerbil_scheme_rust_collection_snapshot_raw(root: i64, kind: i32, len: u64) -> i64;
    /// # Safety
    /// Live root and owner; -1 selects the root, otherwise its vector slot.
    pub fn gerbil_scheme_rust_bigint_size_indexed(root: i64, index: i64) -> i64;
    /// # Safety
    /// Live root and owner, valid disjoint writable span of exactly len bytes.
    /// -1 selects the root, otherwise its vector slot; errors publish nothing.
    pub fn gerbil_scheme_rust_bigint_copy_indexed(
        root: i64,
        index: i64,
        bytes: *mut u8,
        len: u64,
    ) -> i32;
    /// # Safety
    /// Active runtime owner only. Diagnostic registry count, not heap bytes.
    pub fn gerbil_scheme_rust_root_count_raw() -> i64;
    /// # Safety
    /// Active runtime owner only. Requests the installed SDK's normal collector.
    pub fn gerbil_scheme_rust_force_gc_raw() -> i32;
    /// # Safety
    /// Active owner, live root, writable disjoint scalar projection.
    pub fn gerbil_scheme_rust_object_scalar_raw(
        root: i64,
        output: *mut crate::GerbilScalarProjection,
    ) -> i32;
    /// # Safety
    /// The word must be a live object on the active Scheme owner.
    pub fn gerbil_scheme_rust_object_root_raw(word: usize) -> i64;
    /// # Safety
    /// The root must remain live on its owner; the borrowed word must not escape.
    pub fn gerbil_scheme_rust_object_word_raw(root: i64) -> usize;
    /// # Safety
    /// Disjoint readable span of live roots on the owner; kind 0/1/2.
    pub fn gerbil_scheme_rust_collection_create_raw(
        roots: *const GerbilRootId,
        len: u64,
        kind: i32,
    ) -> i64;
    /// # Safety
    /// Owner-local root, kind 0/1/2; negative result rejects type mismatch.
    pub fn gerbil_scheme_rust_collection_length_raw(root: i64, kind: i32) -> i64;
    /// # Safety
    /// Owner-local root, kind 0/1/2; returned positive root has one new owner.
    pub fn gerbil_scheme_rust_collection_element_raw(root: i64, kind: i32, index: u64) -> i64;
    /// # Safety
    /// Active owner and readable immutable span; the caller exclusively owns
    /// every live token in the span. Nonascending, duplicate or missing tokens
    /// are rejected transactionally, without releasing any token.
    pub fn gerbil_scheme_rust_roots_release_raw(roots: *const GerbilRootId, len: u64) -> i32;
    /// # Safety
    /// Readable canonical little-endian magnitude, sign 0/1; active owner.
    pub fn gerbil_scheme_rust_bigint_create_raw(bytes: *const u8, len: u64, negative: i32) -> i64;
    /// # Safety
    /// Owner-local live root; negative result rejects non-integer values.
    pub fn gerbil_scheme_rust_bigint_size_raw(root: i64) -> i64;
    /// # Safety
    /// Writable span of exactly len bytes, owner-local integer root.
    /// Returns 0/1 sign on success, negative on capacity/type failure.
    pub fn gerbil_scheme_rust_bigint_copy_raw(root: i64, bytes: *mut u8, len: u64) -> i32;
}
