/* Private test image: real wrapper and real libc select; no SDK or Scheme.
 * The three legacy SDK hooks are intentionally inert, not a simulated VM. */
#include <stdint.h>
#include <time.h>
#include <sys/select.h>
#include "../ffi/owner_wait_scope.h"

static void (*test_enter)(void *);
static void *test_enter_context;
void gerbil_owner_clock_signpost(uint64_t value) { (void)value; }
void gerbil_mailbox_native_wait_enter(void) {
  if (test_enter) test_enter(test_enter_context);
}
void gerbil_mailbox_native_wait_exit(void) {}
void gerbil_mailbox_native_progress(uint64_t previous, uint64_t now) {
  (void)previous; (void)now;
}
void gerbil_owner_wait_bind(uint64_t (*observe)(int, uint64_t)) { (void)observe; }
uint64_t gerbil_owner_scope_gc_count(void) { return 0; }
uint64_t gerbil_owner_scope_gc_wall(void) { return 0; }
uint64_t gerbil_owner_scope_gc_cpu(void) { return 0; }
void gerbil_owner_scope_bind(uint64_t (*control)(const char *,uint64_t,uint64_t),
    uint64_t (*publish)(uint64_t,uint64_t),
    void (*source)(uint64_t,uint64_t,uint64_t,uint64_t,uint64_t,uint64_t)) {
  (void)control; (void)publish; (void)source;
}
#ifndef OWNER_WAIT_HEADER
#define OWNER_WAIT_HEADER "../ffi/owner_wait.h"
#endif
#include OWNER_WAIT_HEADER

#ifdef GERBIL_OWNER_WAIT_SCOPE
int gws_open(struct owner_wait_scope_handle *handle) { return owner_wait_binding_open(handle); }
int gws_progress(struct owner_wait_scope_handle handle, uint64_t previous, uint64_t now) {
  return owner_wait_binding_progress(handle, previous, now);
}
int gws_finish(struct owner_wait_scope_handle handle, int aborting,
    struct owner_wait_scope_snapshot *snapshot) {
  return owner_wait_binding_finish(handle, aborting, snapshot);
}
uint64_t gws_clock(void) { return owner_wait_clock(); }
int gws_select(int count, fd_set *readers, fd_set *writers, fd_set *errors, struct timeval *timeout) {
  return select(count, readers, writers, errors, timeout);
}
int gws_original_distinct(void) {
  pthread_once(&owner_select_once, owner_select_resolve);
  return owner_select_original && owner_select_original != &select;
}
void gws_on_enter(void (*callback)(void *), void *context) {
  test_enter = callback; test_enter_context = context;
}
uint64_t gws_legacy_recording(int recording) {
  return gerbil_owner_wait_observe(0, (uint64_t)recording);
}
int gws_exhaustion_contract(void) {
  /* Fault injection after all scopes/threads have closed. No reset/reuse. */
  __atomic_store_n(&owner_wait_binding_generation, UINT64_MAX, __ATOMIC_RELAXED);
  struct owner_wait_scope_handle unchanged = {123, 456};
  errno = EDOM;
  int status = owner_wait_binding_open(&unchanged);
  return status == EOVERFLOW && errno == EDOM && unchanged.generation == 123 &&
    unchanged.owner == 456 && owner_wait_association.phase == OWNER_WAIT_SCOPE_CLOSED;
}
#endif
