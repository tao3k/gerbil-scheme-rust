/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
#if defined(__linux__) && !defined(_GNU_SOURCE)
#define _GNU_SOURCE
#endif
#define ___VERSION 409008
#include "gambit.h"
#if defined(GERBIL_SCHEME_RUST_BUILD) || defined(GERBIL_SCHEME_RUST_TOOLCHAIN_HOME)
#include "build-home.h"
#endif

#include <stdint.h>
#include <string.h>
#if defined(__unix__) || defined(__APPLE__)
#include <sys/resource.h>
#endif
#include "../include/gerbil_scheme_rust.h"
#include "host_io.h"
#include "host_environment.h"

___BEGIN_C_LINKAGE
#ifndef GERBIL_SCHEME_RUST_EXTERNAL_PROGRAM
extern ___mod_or_lnk ___LNK_gerbil__scheme__rust__linker(
    ___global_state_struct *);
#endif
___END_C_LINKAGE

/* 0 = never initialized, 1 = running, 2 = finalized and not restartable. */
static int gerbil_scheme_rust_state = 0;
#if defined(GERBIL_SCHEME_RUST_BUILD) && (defined(__unix__) || defined(__APPLE__))
/* Native build/test owns Gerbil child processes for the duration of the call.
 * Save the handler installed by the actual runtime, not a private SDK symbol.
 * Ordinary Rust ownership and its original handler resume after the operation. */
static struct sigaction package_child_action;
static int package_child_action_ready = 0;
static int package_image_live = 0;
#endif

/* Exact failed startup branch, SDK status, host errno and processor count. */
static int64_t setup_diagnostic[4] = {0, 0, 0, 0};
int32_t gerbil_scheme_instance_setup_diagnostic(int64_t *out) {
  if (out == NULL) return -1;
  memcpy(out, setup_diagnostic, sizeof(setup_diagnostic));
  return 0;
}

/* Owner-local qualification snapshot. Heap size is capacity, not live bytes;
 * RSS is the process high-water mark and cannot be attributed to one variant. */
int gerbil_scheme_rust_objects_usage(double *gc, uint64_t *os) {
  if (gerbil_scheme_rust_state != 1 || gc == NULL || os == NULL) return -1;
  ___virtual_machine_state vm = ___VMSTATE;
  gc[0] = vm->mem.nb_gcs_;
  gc[1] = vm->mem.gc_user_time_ + vm->mem.gc_sys_time_;
  gc[2] = vm->mem.gc_real_time_;
  gc[3] = (double)vm->mem.heap_size_ * ___WS;
#if defined(__unix__) || defined(__APPLE__)
  struct rusage usage;
  if (getrusage(RUSAGE_SELF, &usage) != 0) return -1;
  os[0] = ((uint64_t)usage.ru_utime.tv_sec + (uint64_t)usage.ru_stime.tv_sec) * 1000000000ULL
        + ((uint64_t)usage.ru_utime.tv_usec + (uint64_t)usage.ru_stime.tv_usec) * 1000ULL;
#if defined(__APPLE__)
  os[1] = (uint64_t)usage.ru_maxrss;
#else
  os[1] = (uint64_t)usage.ru_maxrss * 1024ULL;
#endif
  os[2] = (uint64_t)usage.ru_nvcsw;
  os[3] = (uint64_t)usage.ru_nivcsw;
  return 0;
#else
  return -1;
#endif
}

GerbilStatus gerbil_scheme_rust_fixnum_from_word(intptr_t number, uintptr_t *out) {
  if (gerbil_scheme_rust_state != 1)
    return gerbil_scheme_rust_state == 2 ? GERBIL_STATUS_RUNTIME_FINALIZED : GERBIL_STATUS_NOT_INITIALIZED;
  if (out == NULL) return GERBIL_STATUS_NULL_POINTER;
  ___SCMOBJ value = ___FIX(number);
  if (___INT(value) != number) return GERBIL_STATUS_INVALID_VALUE;
  *out = (uintptr_t)value;
  return GERBIL_STATUS_OK;
}

/* Official Gambit representation operators; no Scheme allocation or retained
 * body pointer. Validate the complete span before publishing any result. */
static uint32_t scalar_tag(___SCMOBJ value) {
  ___FLONUMP_DECL
  if (value == ___FAL || value == ___TRU) return 1;
  if (___FIXNUMP(value)) return 2;
  if (___CHARP(value)) {
    uint64_t code = (uint64_t)___ORD(value);
    return code <= 0x10ffff && !(code >= 0xd800 && code <= 0xdfff) ? 3 : 0;
  }
  if (___FLONUMP(value)) return 4;
  return 0;
}
GerbilStatus gerbil_scheme_rust_scalar_batch(const uintptr_t *values,
                                            size_t length,
                                            GerbilScalarProjection *output) {
  if (gerbil_scheme_rust_state != 1)
    return gerbil_scheme_rust_state == 2 ? GERBIL_STATUS_RUNTIME_FINALIZED
                                        : GERBIL_STATUS_NOT_INITIALIZED;
  if (length == 0) return GERBIL_STATUS_OK;
  if (values == NULL || output == NULL) return GERBIL_STATUS_NULL_POINTER;
  for (size_t i = 0; i < length; ++i)
    if (scalar_tag((___SCMOBJ)values[i]) == 0)
      return GERBIL_STATUS_INVALID_VALUE;
  for (size_t i = 0; i < length; ++i) {
    ___SCMOBJ value = (___SCMOBJ)values[i];
    output[i].tag = scalar_tag(value);
    output[i].reserved = 0;
    if (output[i].tag == 1) output[i].bits = value == ___TRU;
    else if (output[i].tag == 2) output[i].bits = (uint64_t)(int64_t)___INT(value);
    else if (output[i].tag == 3) output[i].bits = (uint64_t)___ORD(value);
    else {
      double number = ___F64UNBOX(value);
      memcpy(&output[i].bits, &number, sizeof(number));
    }
  }
  return GERBIL_STATUS_OK;
}

/* Report the ABI used to compile this bridge, not flags requested by a host.
 * No runtime initialization, allocation, or processor-state access occurs.
 * Multiple VM storage alone does not imply thread-local foreign entry. */
uint32_t gerbil_scheme_rust_vm_capability_flags(void) {
  uint32_t flags = 0;
#ifndef ___SINGLE_VM
  flags |= 1u;
#endif
#ifndef ___SINGLE_THREADED_VMS
  flags |= 2u;
#endif
  return flags;
}

uint32_t gerbil_scheme_rust_vm_max_processors(void) {
  return ___MAX_PROCESSORS;
}

int64_t gerbil_scheme_rust_identity_i64(int64_t value) { return value; }

static int32_t runtime_init_program(
    ___mod_or_lnk (*linker)(___global_state_struct *), int single_processor) {
  ___setup_params_struct params;
  gerbil_host_io host;

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
#if defined(GERBIL_SCHEME_RUST_BUILD) || defined(GERBIL_SCHEME_RUST_TOOLCHAIN_HOME)
  params.gambitdir = package_toolchain_path_0;
  params.gambitdir_map = package_toolchain_map;
#endif
  if (single_processor) params.parallelism_level = 1;
  /*
   * The default entry retains Gambit's policy; the explicit single-processor
   * entry sets one active processor, independently of host worker count or
   * compiled SDK capacity. Scheme owns scheduling within that population.
   */
  /* Failed setup is terminal too: never retry partially initialized state. */
  memset(setup_diagnostic, 0, sizeof(setup_diagnostic));
  if (gerbil_host_io_capture(&host) != 0) {
    setup_diagnostic[0] = 1;
    setup_diagnostic[2] = errno;
    gerbil_scheme_rust_state = 2;
    return 4;
  }
  ___SCMOBJ initialized = ___setup(&params);
  setup_diagnostic[1] = ___FIXNUMP(initialized) ? ___INT(initialized) : (int64_t)initialized;
#if defined(GERBIL_SCHEME_RUST_BUILD) && (defined(__unix__) || defined(__APPLE__))
  if (initialized == ___FIX(___NO_ERR)) {
    package_child_action_ready = sigaction(SIGCHLD, NULL, &package_child_action) == 0;
    package_image_live = linker == ___LNK_gerbil__scheme__rust__linker;
  }
#endif
  int host_restored = gerbil_host_io_restore(&host);
  if (host_restored != 0) setup_diagnostic[2] = errno;
  if (initialized == ___FIX(___NO_ERR)) setup_diagnostic[3] = ___VMSTATE->processor_count;
  setup_diagnostic[0] = initialized != ___FIX(___NO_ERR) ? 2 :
    (host_restored != 0 ? 3 : ((single_processor && setup_diagnostic[3] != 1) ? 4 : 0));
  if (initialized != ___FIX(___NO_ERR) || host_restored != 0 ||
      (single_processor && ___VMSTATE->processor_count != 1)) {
    if (initialized == ___FIX(___NO_ERR)) {
      gerbil_host_sigpipe pipe;
      if (gerbil_host_sigpipe_begin(&pipe) == 0) {
        ___cleanup();
        gerbil_host_signals_restore(&host.signals);
        gerbil_host_sigpipe_end(&pipe);
      }
      gerbil_host_affinity_restore(&host.affinity);
    }
    gerbil_scheme_rust_state = 2;
    return 4;
  }
  gerbil_scheme_rust_state = 1;
  return 0;
}

int32_t gerbil_scheme_rust_runtime_init_program(
    ___mod_or_lnk (*linker)(___global_state_struct *)) {
  return runtime_init_program(linker, 0);
}

int32_t gerbil_scheme_rust_runtime_init_program_single_processor(
    ___mod_or_lnk (*linker)(___global_state_struct *)) {
  return runtime_init_program(linker, 1);
}

uint32_t gerbil_scheme_rust_current_vm_processors(void) {
  if (gerbil_scheme_rust_state != 1) return 0;
  return (uint32_t)___VMSTATE->processor_count;
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

  gerbil_host_affinity affinity;
  if (gerbil_host_affinity_capture(&affinity) != 0) return 4;
  gerbil_host_environment environment;
  if (gerbil_host_environment_capture(&environment) != 0) return 4;
  gerbil_host_sigpipe pipe;
  if (gerbil_host_sigpipe_begin(&pipe) != 0) {
    gerbil_host_environment_discard(&environment);
    return 4;
  }
  gerbil_host_signals signals;
  int captured = gerbil_host_signals_capture(&signals);
  if (captured != 0) {
    gerbil_host_sigpipe_end(&pipe);
    gerbil_host_environment_discard(&environment);
    return 4;
  }
  ___cleanup();
  gerbil_host_environment_restore(&environment);
  gerbil_scheme_rust_state = 2;
  int restored = gerbil_host_signals_restore(&signals);
  int pipe_restored = gerbil_host_sigpipe_end(&pipe);
  int affinity_restored = gerbil_host_affinity_restore(&affinity);
  return restored == 0 && pipe_restored == 0 && affinity_restored == 0 ? 0 : 4;
}

#ifdef GERBIL_SCHEME_RUST_BUILD
extern int32_t gerbil_scheme_rust_load_build_scheme(const char *library, void *forward);
static gerbil_host_io *package_io_active;
static int package_forward_output(void) {
#if defined(__unix__) || defined(__APPLE__)
  return package_io_active == NULL ? 0 : gerbil_host_io_forward(package_io_active);
#else
  return 0;
#endif
}
/* Pass the address of typed, process-lived callback storage through Gambit's
 * ordinary pointer conversion, not its unsupported C-to-Scheme function conversion. */
static int (*package_forward_callback)(void) = package_forward_output;
extern int32_t gerbil_scheme_rust_package_build_scheme(const char *directory);
extern int32_t gerbil_scheme_rust_package_clean_scheme(const char *directory);
extern int32_t gerbil_scheme_rust_package_test_scheme(const char *directory, const char *tests);
extern int32_t gerbil_scheme_rust_package_stage_scheme(const char *, const char *, int64_t);
extern int32_t gerbil_scheme_rust_package_build_spec_scheme(const char *, const char *);
extern int32_t gerbil_scheme_rust_package_load_closure_scheme(int64_t, const char *);
extern int32_t gerbil_scheme_rust_package_paths_scheme(const char *prefix, const char *libraries);
extern int32_t gerbil_scheme_rust_package_main_scheme(const char *directory, const void *arguments, unsigned int count);
static GerbilStatus package_enter(gerbil_host_io *host) {
  if (package_io_active != NULL) return GERBIL_STATUS_RUNTIME_UNAVAILABLE;
  if (gerbil_scheme_rust_state != 1)
    return gerbil_scheme_rust_state == 2 ? GERBIL_STATUS_RUNTIME_FINALIZED : GERBIL_STATUS_NOT_INITIALIZED;
  if (!package_image_live) return GERBIL_STATUS_RUNTIME_UNAVAILABLE;
  if (gerbil_host_io_capture(host) != 0) return GERBIL_STATUS_RUNTIME_UNAVAILABLE;
#if defined(__unix__) || defined(__APPLE__)
  if (!package_child_action_ready || sigaction(SIGCHLD, &package_child_action, NULL) != 0) {
    gerbil_host_io_restore(host);
    return GERBIL_STATUS_RUNTIME_UNAVAILABLE;
  }
#endif
  package_io_active = host;
  if (gerbil_scheme_rust_load_build_scheme(package_library_path, &package_forward_callback) != 0) {
    package_io_active = NULL;
    gerbil_host_io_restore(host);
    return GERBIL_STATUS_RUNTIME_UNAVAILABLE;
  }
  return GERBIL_STATUS_OK;
}
static GerbilStatus package_leave(gerbil_host_io *host, int32_t code, int32_t *result) {
  package_io_active = NULL;
  if (gerbil_host_io_restore(host) != 0) return GERBIL_STATUS_RUNTIME_UNAVAILABLE;
  *result = code;
  return GERBIL_STATUS_OK;
}
GerbilStatus gerbil_scheme_rust_package_build_cstr(const char *directory, int32_t *result) {
  if (directory == NULL || result == NULL) return GERBIL_STATUS_NULL_POINTER;
  gerbil_host_io host;
  GerbilStatus status = package_enter(&host);
  if (status != GERBIL_STATUS_OK) return status;
  return package_leave(&host, gerbil_scheme_rust_package_build_scheme(directory), result);
}
GerbilStatus gerbil_scheme_rust_package_clean_cstr(const char *directory, int32_t *result) {
  if (directory == NULL || result == NULL) return GERBIL_STATUS_NULL_POINTER;
  gerbil_host_io host;
  GerbilStatus status = package_enter(&host);
  if (status != GERBIL_STATUS_OK) return status;
  return package_leave(&host, gerbil_scheme_rust_package_clean_scheme(directory), result);
}
GerbilStatus gerbil_scheme_rust_package_test_cstr(const char *directory, const char *tests, int32_t *result) {
  if (directory == NULL || tests == NULL || result == NULL) return GERBIL_STATUS_NULL_POINTER;
  gerbil_host_io host;
  GerbilStatus status = package_enter(&host);
  if (status != GERBIL_STATUS_OK) return status;
  return package_leave(&host, gerbil_scheme_rust_package_test_scheme(directory, tests), result);
}
GerbilStatus gerbil_scheme_rust_package_stage_cstr(const char *source, const char *output, GerbilRootId imports, int32_t *result) {
  if (source == NULL || output == NULL || result == NULL) return GERBIL_STATUS_NULL_POINTER;
  gerbil_host_io host;
  GerbilStatus status = package_enter(&host);
  if (status != GERBIL_STATUS_OK) return status;
  return package_leave(&host, gerbil_scheme_rust_package_stage_scheme(source, output, imports), result);
}
GerbilStatus gerbil_scheme_rust_package_build_spec_cstr(const char *directory, const char *specification, int32_t *result) {
  if (directory == NULL || specification == NULL || result == NULL) return GERBIL_STATUS_NULL_POINTER;
  gerbil_host_io host;
  GerbilStatus status = package_enter(&host);
  if (status != GERBIL_STATUS_OK) return status;
  return package_leave(&host, gerbil_scheme_rust_package_build_spec_scheme(directory, specification), result);
}
GerbilStatus gerbil_scheme_rust_package_load_closure_cstr(GerbilRootId imports, const char *output, int32_t *result) {
  if (output == NULL || result == NULL) return GERBIL_STATUS_NULL_POINTER;
  gerbil_host_io host;
  GerbilStatus status = package_enter(&host);
  if (status != GERBIL_STATUS_OK) return status;
  return package_leave(&host, gerbil_scheme_rust_package_load_closure_scheme(imports, output), result);
}
GerbilStatus gerbil_scheme_rust_package_paths_cstr(const char *prefix, const char *libraries, int32_t *result) {
  if (prefix == NULL || libraries == NULL || result == NULL) return GERBIL_STATUS_NULL_POINTER;
  gerbil_host_io host;
  GerbilStatus status = package_enter(&host);
  if (status != GERBIL_STATUS_OK) return status;
  return package_leave(&host, gerbil_scheme_rust_package_paths_scheme(prefix, libraries), result);
}
GerbilStatus gerbil_scheme_rust_package_main_cstr(const char *directory, const void *arguments, unsigned int count, int32_t *result) {
  if (directory == NULL || (count != 0 && arguments == NULL) || result == NULL) return GERBIL_STATUS_NULL_POINTER;
  gerbil_host_io host;
  GerbilStatus status = package_enter(&host);
  if (status != GERBIL_STATUS_OK) return status;
  return package_leave(&host, gerbil_scheme_rust_package_main_scheme(directory, arguments, count), result);
}
#endif
