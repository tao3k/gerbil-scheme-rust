/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
#ifndef GERBIL_SCHEME_RUST_H
#define GERBIL_SCHEME_RUST_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define GERBIL_SCHEME_RUST_ABI_VERSION 1u
#define GERBIL_SCHEME_RUST_STATUS_OK 0
#define GERBIL_SCHEME_RUST_STATUS_ALREADY_INITIALIZED 6
#define GERBIL_SCHEME_RUST_STATUS_NOT_INITIALIZED 7
#define GERBIL_SCHEME_RUST_STATUS_RUNTIME_FINALIZED 8

uint32_t gerbil_scheme_rust_abi_version(void);
int64_t gerbil_scheme_rust_add_i64(int64_t left, int64_t right);
int32_t gerbil_scheme_rust_runtime_init(void);
int32_t gerbil_scheme_rust_runtime_cleanup(void);

#ifdef __cplusplus
}
#endif

#endif
