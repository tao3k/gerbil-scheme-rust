/* Explicit diagnostic only. No Scheme roots, observer thread or event buffer.
 * Only the already-owner-checked heartbeat mask path calls this hook.
 * This measures a native acquisition CALL, including its scheduling delay;
 * neither a pure blocked duration nor complete coverage is inferred. */
#ifndef GERBIL_OWNER_TIMER_INTERVAL_H
#define GERBIL_OWNER_TIMER_INTERVAL_H
#include "owner_wait_scope_binding.h"
struct owner_timer_call {
  uint64_t generation, owner, begin, end;
};
static _Thread_local struct {
  uint64_t generation, calls;
  int invalid;
  struct owner_timer_call last, longest;
} owner_timer_intervals;
static void owner_timer_interval_reset(uint64_t generation) {
  memset(&owner_timer_intervals,0,sizeof(owner_timer_intervals));
  owner_timer_intervals.generation=generation;
}
/* Keep diagnostic locals out of the original mask wrapper frame. */
__attribute__((noinline)) static int gerbil_timer_observed_lock(pthread_mutex_t *lock) {
  struct owner_wait_scope *scope=&owner_wait_association;
  if (scope->phase!=OWNER_WAIT_SCOPE_ACTIVE || !scope->primed ||
      scope->generation!=owner_timer_intervals.generation)
    return pthread_mutex_lock(lock);
  uint64_t generation=scope->generation, owner=scope->owner;
  int incoming=errno;
  struct timespec before,after;
  int first=clock_gettime(CLOCK_UPTIME_RAW,&before);
  errno=incoming;
  int status=pthread_mutex_lock(lock);
  int returned_errno=errno;
  int second=clock_gettime(CLOCK_UPTIME_RAW,&after);
  if (first || second || status || owner_timer_intervals.calls==UINT64_MAX) {
    owner_timer_intervals.invalid=1;
  } else {
    uint64_t begin=(uint64_t)before.tv_sec*1000000000ULL+(uint64_t)before.tv_nsec;
    uint64_t end=(uint64_t)after.tv_sec*1000000000ULL+(uint64_t)after.tv_nsec;
    if (end<begin) owner_timer_intervals.invalid=1;
    else {
      struct owner_timer_call call={generation,owner,begin,end};
      owner_timer_intervals.calls++;
      owner_timer_intervals.last=call;
      if (!owner_timer_intervals.longest.generation ||
          end-begin>owner_timer_intervals.longest.end-owner_timer_intervals.longest.begin)
        owner_timer_intervals.longest=call;
    }
  }
  errno=returned_errno;
  return status;
}
/* Called only by a cold query after the existing scope has closed.
 * A retained call is evidence only when wholly inside THAT gap. This deliberately
 * rejects partial/foreign/stale intervals, and never claims full coverage. */
static int owner_timer_interval_match(uint64_t generation,uint64_t owner,
    uint64_t begin,uint64_t end,struct owner_timer_call *out) {
  memset(out,0,sizeof(*out));
  if (!generation || !owner || begin>=end || owner_timer_intervals.invalid ||
      owner_timer_intervals.generation!=generation) return 0;
  const struct owner_timer_call *candidates[]={
    &owner_timer_intervals.last,&owner_timer_intervals.longest};
  for (size_t i=0;i<sizeof(candidates)/sizeof(candidates[0]);++i) {
    const struct owner_timer_call *call=candidates[i];
    if (call->generation==generation && call->owner==owner &&
        begin<=call->begin && call->begin<call->end && call->end<=end &&
        (!out->generation || call->end-call->begin>out->end-out->begin)) *out=*call;
  }
  return out->generation!=0;
}
#endif
