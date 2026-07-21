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
int32_t gerbil_scheme_rust_runtime_cleanup(void);
int64_t gerbil_scheme_rust_identity_i64(int64_t value);
int64_t gerbil_scheme_rust_add_i64(int64_t left, int64_t right);
int32_t gerbil_scheme_rust_is_even_i64(int64_t value);
int32_t gerbil_scheme_rust_compare_i64(int64_t left, int64_t right);
GerbilStatus gerbil_scheme_rust_runtime_sentinel_value(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_null(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_pair(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_proper_list(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_improper_list(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_true(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_false(GerbilValueHandle *out);
GerbilStatus gerbil_scheme_rust_fixture_fixnum(GerbilValueHandle *out);
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
GerbilStatus gerbil_scheme_rust_scheme_object_is_boolean(
    GerbilValueHandle value, GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_scheme_object_as_boolean(
    GerbilValueHandle value, GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_scheme_object_is_fixnum(GerbilValueHandle value,
                                                        GerbilBoolean *out);
GerbilStatus gerbil_scheme_rust_scheme_object_as_fixnum(GerbilValueHandle value,
                                                        GerbilFixnum *out);
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
