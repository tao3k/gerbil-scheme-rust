/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
#define ___VERSION 409007
#include "gambit.h"

#include <stdint.h>

___BEGIN_C_LINKAGE
#ifndef GERBIL_SCHEME_RUST_EXTERNAL_PROGRAM
extern ___mod_or_lnk ___LNK_gerbil__scheme__rust__linker(
    ___global_state_struct *);
#endif
___END_C_LINKAGE

/* 0 = never initialized, 1 = running, 2 = finalized and not restartable. */
static int gerbil_scheme_rust_state = 0;

int64_t gerbil_scheme_rust_identity_i64(int64_t value) { return value; }

int32_t gerbil_scheme_rust_runtime_init_program(
    ___mod_or_lnk (*linker)(___global_state_struct *)) {
  ___setup_params_struct params;

  if (gerbil_scheme_rust_state == 1) {
    return 6;
  }
  if (gerbil_scheme_rust_state == 2) {
    return 8;
  }
  if (linker == 0) {
    return 1;
  }

  ___setup_params_reset(&params);
  params.version = ___VERSION;
  params.linker = linker;
  /* Failed setup is terminal too: never retry partially initialized state. */
  if (___setup(&params) != ___FIX(___NO_ERR)) {
    gerbil_scheme_rust_state = 2;
    return 4;
  }
  gerbil_scheme_rust_state = 1;
  return 0;
}

int32_t gerbil_scheme_rust_runtime_init(void) {
#ifdef GERBIL_SCHEME_RUST_EXTERNAL_PROGRAM
  return 4;
#else
  return gerbil_scheme_rust_runtime_init_program(
      ___LNK_gerbil__scheme__rust__linker);
#endif
}

int32_t gerbil_scheme_rust_runtime_cleanup(void) {
  if (gerbil_scheme_rust_state == 0) {
    return 7;
  }
  if (gerbil_scheme_rust_state == 2) {
    return 8;
  }

  ___cleanup();
  gerbil_scheme_rust_state = 2;
  return 0;
}
