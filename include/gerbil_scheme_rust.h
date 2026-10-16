#ifndef GERBIL_SCHEME_RUST_H
#define GERBIL_SCHEME_RUST_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define GERBIL_SCHEME_RUST_ABI_VERSION 1u

typedef enum GerbilStatus {
  GERBIL_STATUS_OK = 0,
  GERBIL_STATUS_NULL_POINTER = 1,
  GERBIL_STATUS_ABI_MISMATCH = 2,
  GERBIL_STATUS_INVALID_VALUE = 3,
  GERBIL_STATUS_RUNTIME_UNAVAILABLE = 4,
  GERBIL_STATUS_PANIC = 5,
  GERBIL_STATUS_ALREADY_INITIALIZED = 6,
  GERBIL_STATUS_NOT_INITIALIZED = 7,
  GERBIL_STATUS_RUNTIME_FINALIZED = 8
} GerbilStatus;

typedef enum GerbilByteOrder {
  GERBIL_BYTE_ORDER_BIG = 0,
  GERBIL_BYTE_ORDER_LITTLE = 1,
  GERBIL_BYTE_ORDER_NATIVE = 2
} GerbilByteOrder;

typedef struct GerbilBorrowedUtf8 {
  const char *ptr;
  size_t len;
} GerbilBorrowedUtf8;

typedef struct GerbilBorrowedBytevector {
  const uint8_t *ptr;
  size_t len;
} GerbilBorrowedBytevector;

typedef struct GerbilRuntimeOpaque GerbilRuntimeOpaque;
typedef uintptr_t GerbilValueHandle;
typedef uint8_t GerbilBoolean;
typedef intptr_t GerbilFixnum;
typedef uint32_t GerbilChar;
typedef double GerbilFlonum;
GerbilStatus gerbil_scheme_rust_fixnum_from_word(intptr_t value, uintptr_t *out);
/* Owner-local diagnostic: four GC doubles and four OS uint64 counters.
 * Heap capacity and process RSS high-water are not live allocation counts. */
int gerbil_scheme_rust_objects_usage(double *gc, uint64_t *os);
/* Tags: 1 boolean, 2 signed fixnum bits, 3 Unicode scalar, 4 IEEE f64 bits.
 * Input handles must be live owner-local objects; spans must not overlap.
 * Empty spans permit null. Failure leaves every output slot unchanged.
 * The Scheme word zero is fixnum zero, not a null object pointer. */
typedef struct GerbilScalarProjection {
  uint32_t tag;
  uint32_t reserved;
  uint64_t bits;
} GerbilScalarProjection;
GerbilStatus gerbil_scheme_rust_scalar_batch(const uintptr_t *values,
                                            size_t length,
                                            GerbilScalarProjection *output);
typedef int64_t GerbilRootId;
/* Owner-only raw entries. Zero object words are valid; root IDs are positive.
 * Creation transfers a strong root. Collection copy publishes newly owned
 * roots only on success. Release spans contain ascending unique live IDs. */
GerbilRootId gerbil_scheme_rust_object_root_raw(GerbilValueHandle value);
int64_t gerbil_scheme_rust_root_count_raw(void);
int32_t gerbil_scheme_rust_force_gc_raw(void);
int32_t gerbil_scheme_rust_object_scalar_raw(GerbilRootId root, GerbilScalarProjection *output);
/* Index -1 projects the root; otherwise project a retained vector snapshot slot. */
int32_t gerbil_scheme_rust_object_scalar_indexed(GerbilRootId root, int64_t index, GerbilScalarProjection *output);
GerbilRootId gerbil_scheme_rust_collection_snapshot_raw(GerbilRootId root, int32_t kind, uint64_t len);
int64_t gerbil_scheme_rust_bigint_size_indexed(GerbilRootId root, int64_t index);
int32_t gerbil_scheme_rust_bigint_copy_indexed(GerbilRootId root, int64_t index, uint8_t *output, uint64_t len);
GerbilValueHandle gerbil_scheme_rust_object_word_raw(GerbilRootId root);
GerbilRootId gerbil_scheme_rust_collection_create_raw(const GerbilRootId *roots, uint64_t len, int32_t kind);
int64_t gerbil_scheme_rust_collection_length_raw(GerbilRootId root, int32_t kind);
GerbilRootId gerbil_scheme_rust_collection_element_raw(GerbilRootId root, int32_t kind, uint64_t index);
int32_t gerbil_scheme_rust_roots_release_raw(const GerbilRootId *roots, uint64_t len);
GerbilRootId gerbil_scheme_rust_bigint_create_raw(const uint8_t *bytes, uint64_t len, int32_t negative);
int64_t gerbil_scheme_rust_bigint_size_raw(GerbilRootId root);
int32_t gerbil_scheme_rust_bigint_copy_raw(GerbilRootId root, uint8_t *output, uint64_t len);
typedef GerbilStatus (*GerbilI64Callback)(int64_t value, void *context);
typedef GerbilStatus (*GerbilProcedureCallback)(GerbilValueHandle value,
                                                void *context);

typedef struct GerbilPair {
  GerbilValueHandle car;
  GerbilValueHandle cdr;
} GerbilPair;

typedef struct GerbilBorrowedVector {
  const GerbilValueHandle *ptr;
  size_t len;
} GerbilBorrowedVector;

uint32_t gerbil_scheme_rust_abi_version(void);
int32_t gerbil_scheme_rust_runtime_init(void);
struct ___global_state_struct;
union ___mod_or_lnk_union;
typedef union ___mod_or_lnk_union *(*GerbilProgramLinker)(struct ___global_state_struct *);
int32_t gerbil_scheme_rust_runtime_init_program(GerbilProgramLinker linker);
int32_t gerbil_scheme_rust_runtime_init_program_single_processor(GerbilProgramLinker linker);
uint32_t gerbil_scheme_rust_current_vm_processors(void);
int32_t gerbil_scheme_rust_runtime_cleanup(void);
/* Linked ABI capabilities; these do not certify worker or scheduler safety. */
#define GERBIL_VM_CAP_MULTIPLE_VMS 1u
#define GERBIL_VM_CAP_THREAD_LOCAL_ENTRY 2u
uint32_t gerbil_scheme_rust_vm_capability_flags(void);
uint32_t gerbil_scheme_rust_vm_max_processors(void);
int64_t gerbil_scheme_rust_identity_i64(int64_t value);
int64_t gerbil_scheme_rust_add_i64(int64_t left, int64_t right);
int32_t gerbil_scheme_rust_is_even_i64(int64_t value);
int32_t gerbil_scheme_rust_compare_i64(int64_t left, int64_t right);
GerbilStatus gerbil_scheme_rust_runtime_sentinel_value(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_null(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_void(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_pair(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_proper_list(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_improper_list(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_true(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_false(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_fixnum(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_exact_integer_large_positive(
    GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_exact_integer_large_negative(
    GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_char_ascii(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_char_bmp(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_char_non_bmp(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_flonum_finite(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_flonum_nan(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_flonum_pos_inf(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_flonum_neg_inf(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_flonum_neg_zero(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_bytevector(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_value_is_pair(GerbilValueHandle value,
                                              GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_value_is_list(GerbilValueHandle value,
                                              GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_value_is_null(GerbilValueHandle value,
                                              GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_scheme_object_is_pair(GerbilValueHandle value,
                                                      GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_scheme_object_is_list(GerbilValueHandle value,
                                                      GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_scheme_object_is_null(GerbilValueHandle value,
                                                      GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_scheme_object_is_void(GerbilValueHandle value,
                                                      GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_scheme_object_is_bytevector(
    GerbilValueHandle value, GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_scheme_object_is_boolean(
    GerbilValueHandle value, GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_scheme_object_as_boolean(
    GerbilValueHandle value, GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_scheme_object_is_fixnum(GerbilValueHandle value,
                                                        GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_scheme_object_as_fixnum(GerbilValueHandle value,
                                                        GerbilFixnum *out);
GerbilStatus gerbil_scheme_rust_scheme_object_is_exact_integer(
    GerbilValueHandle value, GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_scheme_object_exact_integer_to_i64(
    GerbilValueHandle value, int64_t *out);
GerbilStatus gerbil_scheme_rust_scheme_object_exact_integer_to_u64(
    GerbilValueHandle value, uint64_t *out);
GerbilStatus gerbil_scheme_rust_scheme_object_is_char(GerbilValueHandle value,
                                                      GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_scheme_object_as_char(GerbilValueHandle value,
                                                      GerbilChar *out);
GerbilStatus gerbil_scheme_rust_scheme_object_is_flonum(GerbilValueHandle value,
                                                        GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_scheme_object_as_flonum(GerbilValueHandle value,
                                                        GerbilFlonum *out);
GerbilStatus gerbil_scheme_rust_scheme_object_bytevector_length(
    GerbilValueHandle value, size_t *out);
GerbilStatus gerbil_scheme_rust_scheme_object_bytevector_u8_ref(
    GerbilValueHandle value, size_t index, uint8_t *out);
GerbilStatus gerbil_scheme_rust_scheme_object_bytevector_copy(
    GerbilValueHandle value, uint8_t *out, size_t len);
GerbilStatus gerbil_scheme_rust_bytevector_to_bytestring_root(
    GerbilValueHandle value, int32_t delimiter, GerbilRootId *out);
GerbilStatus gerbil_scheme_rust_bytestring_to_bytevector_root(
    GerbilBorrowedUtf8 value, int32_t delimiter, GerbilRootId *out);
/* Input is borrowed only for the call; the returned root owns a Scheme copy.
 * Null input is valid only at zero length. On error out remains untouched. */
GerbilStatus gerbil_scheme_rust_bytes_to_bytevector_root(
    GerbilBorrowedBytevector value, GerbilRootId *out);
GerbilStatus gerbil_scheme_rust_bytevector_to_uint(
    GerbilValueHandle value, int32_t byte_order, size_t size, uint64_t *out);
GerbilStatus gerbil_scheme_rust_bytevector_to_sint(
    GerbilValueHandle value, int32_t byte_order, size_t size, int64_t *out);
GerbilStatus gerbil_scheme_rust_root_bytevector_to_uint(
    GerbilRootId root, int32_t byte_order, size_t size, uint64_t *out);
GerbilStatus gerbil_scheme_rust_root_bytevector_to_sint(
    GerbilRootId root, int32_t byte_order, size_t size, int64_t *out);
GerbilStatus gerbil_scheme_rust_uint_to_bytevector_root(
    uint64_t value, int32_t byte_order, size_t size, GerbilRootId *out);
GerbilStatus gerbil_scheme_rust_sint_to_bytevector_root(
    int64_t value, int32_t byte_order, size_t size, GerbilRootId *out);
GerbilStatus gerbil_scheme_rust_i64_to_exact_integer_root(
    int64_t value, GerbilRootId *out);
GerbilStatus gerbil_scheme_rust_u64_to_exact_integer_root(
    uint64_t value, GerbilRootId *out);
GerbilStatus gerbil_scheme_rust_root_exact_integer_to_i64(
    GerbilRootId root, int64_t *out);
GerbilStatus gerbil_scheme_rust_root_exact_integer_to_u64(
    GerbilRootId root, uint64_t *out);
GerbilStatus gerbil_scheme_rust_root_string_to_utf8(GerbilRootId root, GerbilRootId *out);
/* Caller-owned writable span; failure may write a prefix but leaves written untouched. */
GerbilStatus gerbil_scheme_rust_root_string_encode_into(GerbilRootId root, uint8_t *out, size_t capacity, size_t *written);
GerbilStatus gerbil_scheme_rust_root_utf8_to_string(GerbilRootId root, GerbilRootId *out);
GerbilStatus gerbil_scheme_rust_root_string_length(GerbilRootId root,
                                                   size_t *out);
GerbilStatus gerbil_scheme_rust_root_string_char_ref(GerbilRootId root,
                                                     size_t index,
                                                     GerbilChar *out);
GerbilStatus gerbil_scheme_rust_root_bytevector_length(GerbilRootId root,
                                                       size_t *out);
GerbilStatus gerbil_scheme_rust_root_bytevector_u8_ref(GerbilRootId root,
                                                       size_t index,
                                                       uint8_t *out);
GerbilStatus gerbil_scheme_rust_root_bytevector_copy(GerbilRootId root,
                                                     uint8_t *out,
                                                     size_t len);
GerbilStatus gerbil_scheme_rust_root_release(GerbilRootId root);
GerbilStatus gerbil_scheme_rust_pair_car(GerbilValueHandle value,
                                         GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_pair_cdr(GerbilValueHandle value,
                                         GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_pair_parts(GerbilValueHandle value,
                                           GerbilPair *out);

#ifdef __cplusplus
}
#endif

#endif
