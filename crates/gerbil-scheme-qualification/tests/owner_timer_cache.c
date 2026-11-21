/* Exercise the actual loader and timer headers with deterministic provider
 * failures. This fixture does not claim SDK or VM acceptance. */
#include <assert.h>
#include <dlfcn.h>
#include <errno.h>
#include <os/lock.h>
#include <pthread.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>
static int mode, allocations, frees, opens, resolves, closes, masks;
static int provider;
static void *allocate(size_t bytes) { ++allocations; return mode == 1 ? NULL : malloc(bytes); }
static void release(void *p) { if (p) ++frees; free(p); }
static void *open_provider(const char *path, int flags) {
  assert(strcmp(path, "/usr/lib/libSystem.B.dylib") == 0);
  assert(flags == (RTLD_NOW | RTLD_LOCAL));
  ++opens; return mode == 2 ? NULL : &provider;
}
static int close_provider(void *p) { assert(p == &provider); ++closes; return mode == 6 ? -1 : 0; }
static int system_mask(int how, const sigset_t *set, sigset_t *previous) {
  (void)how; (void)set; (void)previous; ++masks; return 0;
}
static int other_mask(int how, const sigset_t *set, sigset_t *previous) {
  return system_mask(how, set, previous);
}
static void *resolve_provider(void *handle, const char *name) {
  ++resolves; assert(strcmp(name, "pthread_sigmask") == 0);
  if (mode == 3 || mode == 6) return NULL;
  if (mode == 7) return (void *)pthread_sigmask;
  if (mode == 4 && handle == &provider) return (void *)other_mask;
  return (void *)system_mask;
}
#define malloc allocate
#define free release
#define dlopen open_provider
#define dlsym resolve_provider
#include "../../../ffi/instance_loader.h"
#include "../../../ffi/instance_timer.h"
#undef malloc
#undef free
#undef dlopen
#undef dlsym
int main(int argc, char **argv) {
  assert(argc == 2); mode = atoi(argv[1]);
  gerbil_scheme_instance_bind_loader(close_provider);
  assert(pthread_sigmask(SIG_BLOCK, NULL, NULL) == ENOSYS);
  int status = gerbil_timer_init_owner();
  if (mode == 0) {
    assert(status == 0 && opens == 1 && resolves == 2);
    assert(gerbil_scheme_instance_pending_modules() == 1);
    for (int i = 0; i < 1000; ++i) assert(pthread_sigmask(SIG_BLOCK, NULL, NULL) == 0);
    sigset_t set; sigemptyset(&set); sigaddset(&set, SIGALRM);
    assert(pthread_sigmask(SIG_BLOCK, &set, NULL) == 0 && gerbil_timer_masked);
    assert(pthread_sigmask(SIG_UNBLOCK, &set, NULL) == 0 && !gerbil_timer_masked);
    assert(masks == 1002 && resolves == 2 && opens == 1);
    assert(gerbil_timer_status() == 0 && gerbil_timer_cleanup() == 0);
    gerbil_scheme_instance_code_leases(1);
    assert(gerbil_scheme_instance_release_modules() == -1 && closes == 0);
    gerbil_scheme_instance_code_leases(0);
    assert(gerbil_scheme_instance_release_modules() == 0 && closes == 1);
    assert(gerbil_scheme_instance_pending_modules() == 0 && allocations == frees);
  } else if (mode == 1) {
    assert(status == ENOMEM && opens == 0 && resolves == 0 && frees == 0);
  } else if (mode == 2) {
    assert(status == ENOSYS && opens == 1 && resolves == 0 && allocations == frees);
  } else if (mode == 6) {
    assert(status == EIO && closes == 1 && gerbil_system_mask == NULL);
    assert(gerbil_scheme_instance_pending_modules() == 1 && allocations == frees + 1);
    mode = 0;
    assert(gerbil_scheme_instance_release_modules() == 0 && closes == 2 && allocations == frees);
  } else {
    assert((mode == 3 || mode == 4 || mode == 7) && status == ENOSYS);
    assert(closes == 1 && allocations == frees && gerbil_system_mask == NULL);
  }
  assert(gerbil_scheme_instance_pending_modules() == 0);
  if (mode != 0) assert(pthread_sigmask(SIG_BLOCK, NULL, NULL) == ENOSYS);
  puts("timer-cache-case-passed");
  return 0;
}
