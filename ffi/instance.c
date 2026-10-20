/* Fixed-owner Gerbil native runtime lifecycle; shared by library consumers. */
#define GERBIL_SCHEME_RUST_EXTERNAL_PROGRAM
#include "runtime.c"
#include "instance.h"
#include "instance_loader.h"
#include <time.h>
#if defined(__APPLE__)
/* Darwin private pthread SPI, available since macOS 10.12. This backend
 * already requires Darwin. Definitions are image-local under INSTANCE_EXPORTS:
 * the statically linked SDK binds here; host libc callers remain unchanged.
 * The SDK normalizer temporarily chdirs under an image-local mutex, which
 * cannot protect process cwd from another VM's independent normalizer. */
extern int pthread_chdir_np(const char *);
extern int pthread_fchdir_np(int);
int chdir(const char *path) { return pthread_chdir_np(path); }
int fchdir(int descriptor) {
  /* -1 clears Darwin thread cwd, but remains EBADF in the POSIX SDK API. */
  if (descriptor < 0) { errno = EBADF; return -1; }
  return pthread_fchdir_np(descriptor);
}
#endif
#ifndef GERBIL_INSTANCE_LINKER
#error "The canonical program build must specify GERBIL_INSTANCE_LINKER"
#endif
extern ___mod_or_lnk GERBIL_INSTANCE_LINKER(___global_state_struct *);
#ifndef GERBIL_OWNER_FAILURE_STAGE
#define GERBIL_OWNER_FAILURE_STAGE 0
#endif
static uint32_t setup_phase = 0;
uint32_t gerbil_scheme_instance_abi(void) { return 4; }
uint32_t gerbil_scheme_instance_setup_phase(void) { return setup_phase; }
int32_t gerbil_scheme_instance_setup(uint32_t owner) {
  /* Qualification faults are compiled into separate images. No environment,
   * source-name policy or shared SDK configuration controls production setup. */
  if (GERBIL_OWNER_FAILURE_STAGE == 1 && owner == 1) return 61;
#if defined(__APPLE__)
  if (pthread_chdir_np(".") != 0) {
    setup_diagnostic[0] = 5;
    setup_diagnostic[2] = errno;
    return 4;
  }
#endif
  setup_phase = 1;
  int32_t status = gerbil_scheme_rust_runtime_init_program_single_processor(GERBIL_INSTANCE_LINKER);
  if (status != 0) {
#if defined(__APPLE__)
    pthread_fchdir_np(-1);
#endif
    return status;
  }
  setup_phase = 2;
  if (GERBIL_OWNER_FAILURE_STAGE == 2 && owner == 1) return 62;
  return 0;
}
uintptr_t gerbil_scheme_instance_state(void) { return (uintptr_t)___GSTATE; }
uint64_t gerbil_scheme_instance_gc_count(void) { return (uint64_t)___VMSTATE->mem.nb_gcs_; }
/* Optional ABI-v3 diagnostics: cumulative collector seconds and owner CPU.
 * The paired SDK owns these counters; reading them does not enter Scheme. */
int32_t gerbil_scheme_instance_diagnostics(double *out) {
  if (gerbil_scheme_rust_state != 1 || out == NULL) return -1;
  struct timespec cpu;
  if (clock_gettime(CLOCK_THREAD_CPUTIME_ID, &cpu) != 0) return -2;
  out[0] = ___VMSTATE->mem.gc_user_time_;
  out[1] = ___VMSTATE->mem.gc_sys_time_;
  out[2] = ___VMSTATE->mem.gc_real_time_;
  out[3] = (double)cpu.tv_sec + (double)cpu.tv_nsec * 1e-9;
  out[4] = (double)gerbil_scheme_rust_root_count_raw();
  out[5] = ___VMSTATE->mem.nb_gcs_;
  return 0;
}
int32_t gerbil_scheme_instance_usage(double *out) {
  if (gerbil_scheme_rust_state != 1 || out == NULL) return -1;
  out[0] = ___VMSTATE->mem.nb_gcs_;
  out[1] = (double)___VMSTATE->mem.heap_size_ * ___WS;
  out[2] = ___VMSTATE->mem.latest_gc_live_;
  out[3] = (double)gerbil_scheme_rust_root_count_raw();
  return 0;
}
/* Quiescent, fixed-owner terminal transfer of native I/O scratch buffers.
 * The paired SDK's cleanup_io_pstate closes the self-pipe but leaves these
 * allocations in its static pstate. Detach before SDK cleanup so any SDK that
 * also frees null fields cannot double-free, then use its paired allocator. */
static uint64_t retired_io_bytes = 0;
uint64_t gerbil_scheme_instance_io_retired_bytes(void) { return retired_io_bytes; }
int32_t gerbil_scheme_instance_cleanup(void) {
  if (gerbil_scheme_rust_state != 1 || ___VMSTATE->processor_count != 1)
    return -1;
  ___fdset *set = &___PSTATE->os.fdset;
  void *owned[] = { set->readfds, set->writefds, set->exceptfds };
  uint64_t bytes = ((uint64_t)set->size + 7) / 8;
  set->readfds = set->writefds = set->exceptfds = NULL;
  set->size = 0;
  __atomic_store_n(&gerbil_loader_retiring, 1, __ATOMIC_SEQ_CST);
  int32_t status = gerbil_scheme_rust_runtime_cleanup();
  __atomic_store_n(&gerbil_loader_retiring, 0, __ATOMIC_SEQ_CST);
  os_unfair_lock_lock(&gerbil_module_lock);
  int capture_failed = gerbil_module_capture_failures != 0;
  os_unfair_lock_unlock(&gerbil_module_lock);
  if (capture_failed && status == 0) status = -3;
  for (unsigned i=0; i<3; ++i) {
    if (owned[i]) {
      ___FREE_MEM(owned[i]);
      retired_io_bytes += bytes;
    }
  }
#if defined(__APPLE__)
  /* The dedicated owner is about to retire; do not leave a private directory
   * reference to the module namespace after its resources have been removed. */
  if (pthread_fchdir_np(-1) != 0 && status == 0) return -2;
#endif
  return status;
}
