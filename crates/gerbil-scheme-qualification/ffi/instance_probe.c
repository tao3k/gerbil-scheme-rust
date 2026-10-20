/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
#define ___VERSION 409008
#include "gambit.h"
#include <stdint.h>

extern ___mod_or_lnk ___LNK_gerbil__actor__qualification(___global_state_struct *);
extern int64_t gerbil_instance_qualification_batch(uint32_t, uint32_t, uint32_t, void *);
extern uint32_t gerbil_instance_qualification_marker(void);
extern int64_t gerbil_instance_qualification_lifecycle(uint32_t, uint32_t);
static int initialized = 0;

int32_t gerbil_instance_probe_init(void) {
  ___setup_params_struct params;
  if (initialized != 0) return -1;
  initialized = -1;
  ___setup_params_reset(&params);
  params.version = ___VERSION;
  params.parallelism_level = 1;
  params.linker = ___LNK_gerbil__actor__qualification;
  if (___setup(&params) != ___FIX(___NO_ERR)) return -2;
  if (___VMSTATE->processor_count != 1) { ___cleanup(); return -3; }
  initialized = 1;
  return 0;
}

int32_t gerbil_instance_probe_cleanup(void) {
  if (initialized != 1) return -1;
  ___cleanup();
  initialized = -1;
  return 0;
}

uintptr_t gerbil_instance_probe_state(void) { return (uintptr_t)___GSTATE; }

int64_t gerbil_instance_probe_batch(uint32_t marker, uint32_t jobs,
                                   uint32_t rounds, void *context) {
  if (initialized != 1) return -1;
  return gerbil_instance_qualification_batch(marker, jobs, rounds, context);
}

uint32_t gerbil_instance_probe_marker(void) {
  if (initialized != 1) return 0;
  return gerbil_instance_qualification_marker();
}

int64_t gerbil_instance_probe_lifecycle(uint32_t operation, uint32_t owner) {
  if (operation == 99) {
    return initialized == -1 && gerbil_instance_probe_init() == -1 &&
      gerbil_instance_probe_cleanup() == -1 &&
      gerbil_instance_probe_batch(owner, 1, 1, 0) == -1 &&
      gerbil_instance_probe_marker() == 0 ? 0 : -1;
  }
  if (initialized != 1) return -1;
  if (operation == 0) return (int64_t)___VMSTATE->processor_count;
  if (operation == 98) return gerbil_instance_probe_init();
  return gerbil_instance_qualification_lifecycle(operation, owner);
}
