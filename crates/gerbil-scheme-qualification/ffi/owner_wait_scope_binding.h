/* Explicit diagnostic image only. The ordinary image does not include this
 * header. TLS storage is admitted at open; hooks remain owner-only. */
#ifndef GERBIL_OWNER_WAIT_SCOPE_BINDING_H
#define GERBIL_OWNER_WAIT_SCOPE_BINDING_H
#include "owner_wait_scope.h"
#include <time.h>
#include <sys/time.h>

static _Thread_local struct owner_wait_scope owner_wait_association;
/* Image-local activation identity survives individual thread/TLS retirement.
 * One checked atomic reservation per cold open, never per progress or wait. */
static uint64_t owner_wait_binding_generation;
_Static_assert(__atomic_always_lock_free(sizeof(uint64_t), 0), "lock-free diagnostic generation required");
static inline int owner_wait_binding_next_generation(uint64_t *generation) {
  uint64_t previous = __atomic_load_n(&owner_wait_binding_generation, __ATOMIC_RELAXED);
  for (;;) {
    if (previous == UINT64_MAX) return EOVERFLOW;
    uint64_t next = previous + 1;
    if (__atomic_compare_exchange_n(&owner_wait_binding_generation, &previous, next,
          0, __ATOMIC_RELAXED, __ATOMIC_RELAXED)) { *generation = next; return 0; }
  }
}
struct owner_wait_scope_call {
  struct owner_wait_scope_ticket ticket;
  uint64_t owner;
  int entered;
};
static inline uint64_t owner_wait_binding_owner(void) {
  return (uint64_t)(uintptr_t)&owner_wait_association;
}
static inline int owner_wait_binding_cut(struct owner_wait_scope_cut *cut) {
  struct timespec wall, cpu;
  if (clock_gettime(CLOCK_UPTIME_RAW, &wall) || clock_gettime(CLOCK_THREAD_CPUTIME_ID, &cpu))
    return EIO;
  cut->wall = (uint64_t)wall.tv_sec * 1000000000ULL + (uint64_t)wall.tv_nsec;
  cut->cpu = (uint64_t)cpu.tv_sec * 1000000000ULL + (uint64_t)cpu.tv_nsec;
  return 0;
}
static inline int owner_wait_binding_open(struct owner_wait_scope_handle *handle) {
  if (!handle) return EINVAL;
  int saved = errno;
  struct owner_wait_scope *s = &owner_wait_association;
  uint64_t generation, local_generation, owner = owner_wait_binding_owner();
  int status = (owner_wait_enabled || s->phase == OWNER_WAIT_SCOPE_ACTIVE || s->pending) ? EBUSY :
    owner_wait_binding_next_generation(&generation);
  if (!status) status = owner_wait_scope_open(s, owner, &local_generation);
  if (!status) s->generation = generation;
  if (!status) *handle = (struct owner_wait_scope_handle){ generation, owner };
  errno = saved;
  return status;
}
static inline int owner_wait_binding_progress(struct owner_wait_scope_handle handle,
    uint64_t previous, uint64_t now) {
  int saved = errno;
  int status = owner_wait_scope_progress(&owner_wait_association,
    handle.generation, handle.owner, previous, now);
  errno = saved;
  return status;
}
static inline int owner_wait_binding_finish(struct owner_wait_scope_handle handle,
    int aborting, struct owner_wait_scope_snapshot *snapshot) {
  if (!snapshot) return EINVAL;
  int saved = errno;
  struct owner_wait_scope *s = &owner_wait_association;
  int status;
  if (!owner_wait_scope_identity(s, handle.generation, handle.owner)) status = EPERM;
  else if (s->phase == OWNER_WAIT_SCOPE_ABORTED || s->phase == OWNER_WAIT_SCOPE_CLOSED) status = 0;
  else status = owner_wait_scope_close(s, handle.generation, handle.owner, aborting);
  if (!status) status = owner_wait_scope_snapshot(s, handle.generation, handle.owner, snapshot);
  errno = saved;
  return status;
}
static inline void owner_wait_binding_enter(struct owner_wait_scope_call *call,
    const struct timeval *timeout) {
  *call = (struct owner_wait_scope_call){0};
  struct owner_wait_scope *s = &owner_wait_association;
  if (s->phase != OWNER_WAIT_SCOPE_ACTIVE) return;
  struct owner_wait_scope_cut cut;
  if (owner_wait_binding_cut(&cut)) { s->invalid = 1; return; }
  enum owner_wait_timeout_kind kind = !timeout ? OWNER_WAIT_TIMEOUT_INFINITE :
    !timeout->tv_sec && !timeout->tv_usec ? OWNER_WAIT_TIMEOUT_ZERO : OWNER_WAIT_TIMEOUT_FINITE;
  call->owner = owner_wait_binding_owner();
  call->entered = !owner_wait_scope_enter(s, s->generation, call->owner, cut, kind, &call->ticket);
}
/* POSIX cancellation cleanup owns a stack-local ticket until select returns.
 * It acknowledges retirement without inventing an exit timestamp/outcome. */
static inline void owner_wait_binding_cancel(void *argument) {
  struct owner_wait_scope_call *call = argument;
  if (!call->entered) return;
  int saved = errno;
  struct owner_wait_scope *s = &owner_wait_association;
  (void)owner_wait_scope_close(s, call->ticket.generation, call->owner, 1);
  (void)owner_wait_scope_leave(s, call->ticket, call->owner,
    (struct owner_wait_scope_cut){0}, -1, ECANCELED);
  call->entered = 0;
  errno = saved;
}
static inline void owner_wait_binding_leave(struct owner_wait_scope_call *call,
    int result, int call_errno) {
  if (!call->entered) return;
  struct owner_wait_scope_cut cut;
  if (owner_wait_binding_cut(&cut)) {
    owner_wait_association.invalid = 1;
    owner_wait_binding_cancel(call);
    return;
  }
  (void)owner_wait_scope_leave(&owner_wait_association, call->ticket,
    call->owner, cut, result, call_errno);
  call->entered = 0;
}
#endif
