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
  GERBIL_STATUS_INVALID_UTF8 = 2,
  GERBIL_STATUS_TYPE_ERROR = 3,
  GERBIL_STATUS_OVERFLOW = 4,
  GERBIL_STATUS_PANIC = 5,
  GERBIL_STATUS_ALREADY_INITIALIZED = 6,
  GERBIL_STATUS_NOT_INITIALIZED = 7,
  GERBIL_STATUS_RUNTIME_FINALIZED = 8
} GerbilStatus;

typedef struct GerbilBorrowedUtf8 {
  const char *ptr;
  size_t len;
} GerbilBorrowedUtf8;

typedef struct GerbilRuntimeOpaque GerbilRuntimeOpaque;
typedef uintptr_t GerbilValueHandle;
typedef GerbilStatus (*GerbilI64Callback)(int64_t value, void *context);

uint32_t gerbil_scheme_rust_abi_version(void);
GerbilStatus gerbil_scheme_rust_runtime_init(void);
GerbilStatus gerbil_scheme_rust_runtime_cleanup(void);
GerbilStatus gerbil_scheme_rust_add_i64(int64_t lhs, int64_t rhs, int64_t *result);

#ifdef __cplusplus
}
#endif

#endif
