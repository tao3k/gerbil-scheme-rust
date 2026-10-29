/* Qualification-only owner-local association state. No SDK dependency,
 * pointers to Scheme, allocation, clock reads, locks, callbacks or IO.
 * The caller supplies existing progress timestamps and wait clock readings.
 * The default image excludes the separate diagnostic binding. */
#ifndef GERBIL_OWNER_WAIT_SCOPE_H
#define GERBIL_OWNER_WAIT_SCOPE_H

#include <errno.h>
#include <stdint.h>
#include <string.h>

enum owner_wait_scope_phase { OWNER_WAIT_SCOPE_IDLE, OWNER_WAIT_SCOPE_ACTIVE,
  OWNER_WAIT_SCOPE_CLOSED, OWNER_WAIT_SCOPE_ABORTED };
struct owner_wait_scope_cut { uint64_t wall, cpu; };
struct owner_wait_scope_ticket { uint64_t generation, sequence; };
struct owner_wait_scope_handle { uint64_t generation, owner; };
struct owner_wait_scope_gap {
  uint64_t begin, end, calls, call_wall, call_cpu;
  uint64_t zero_timeout, finite_timeout, infinite_timeout;
  uint64_t ready, timed_out, interrupted, failed;
  int present, invalid;
};
struct owner_wait_scope_snapshot {
  uint64_t generation, owner, unsealed_calls;
  int primed, invalid, aborted;
  struct owner_wait_scope_gap gap;
};
enum owner_wait_timeout_kind { OWNER_WAIT_TIMEOUT_ZERO,
  OWNER_WAIT_TIMEOUT_FINITE, OWNER_WAIT_TIMEOUT_INFINITE };
struct owner_wait_scope {
  enum owner_wait_scope_phase phase;
  uint64_t generation, owner, sequence, progress_end, last_wall, last_cpu;
  int primed, pending, invalid;
  enum owner_wait_timeout_kind timeout_kind;
  struct owner_wait_scope_cut entered;
  struct owner_wait_scope_gap current, best;
};

/* Observation completeness only, never service success or timing admission. */
static inline int owner_wait_scope_complete_gap(const struct owner_wait_scope_snapshot *s) {
  return s && s->generation && s->owner && s->primed && !s->aborted && !s->invalid &&
    s->gap.present && !s->gap.invalid && s->gap.begin < s->gap.end;
}

static inline int owner_wait_scope_identity(const struct owner_wait_scope *s,
    uint64_t generation, uint64_t owner) {
  return generation && owner && s->generation == generation && s->owner == owner;
}
static inline int owner_wait_scope_open(struct owner_wait_scope *s, uint64_t owner,
    uint64_t *generation) {
  if (!owner || !generation) return EINVAL;
  if (s->phase == OWNER_WAIT_SCOPE_ACTIVE || s->pending) return EBUSY;
  if (s->generation == UINT64_MAX) return EOVERFLOW;
  uint64_t next = s->generation + 1;
  memset(s, 0, sizeof(*s));
  s->generation = next; s->owner = owner; s->phase = OWNER_WAIT_SCOPE_ACTIVE;
  *generation = next;
  return 0;
}
static inline int owner_wait_scope_enter(struct owner_wait_scope *s,
    uint64_t generation, uint64_t owner, struct owner_wait_scope_cut cut,
    enum owner_wait_timeout_kind timeout_kind, struct owner_wait_scope_ticket *ticket) {
  if (!owner_wait_scope_identity(s, generation, owner) ||
      s->phase != OWNER_WAIT_SCOPE_ACTIVE) return EPERM;
  if (!ticket || timeout_kind < OWNER_WAIT_TIMEOUT_ZERO ||
      timeout_kind > OWNER_WAIT_TIMEOUT_INFINITE) return EINVAL;
  if (s->pending) { s->invalid = 1; return EBUSY; }
  if (s->sequence == UINT64_MAX) { s->invalid = 1; return EOVERFLOW; }
  if ((s->primed && cut.wall < s->progress_end) || cut.wall < s->last_wall ||
      cut.cpu < s->last_cpu) { s->invalid = 1; return EINVAL; }
  s->sequence++; s->entered = cut; s->timeout_kind = timeout_kind; s->pending = 1;
  *ticket = (struct owner_wait_scope_ticket){ generation, s->sequence };
  return 0;
}
static inline int owner_wait_scope_add(uint64_t *field, uint64_t value) {
  if (value > UINT64_MAX - *field) return EOVERFLOW;
  *field += value;
  return 0;
}
static inline int owner_wait_scope_leave(struct owner_wait_scope *s,
    struct owner_wait_scope_ticket ticket, uint64_t owner,
    struct owner_wait_scope_cut cut, int result, int call_errno) {
  if (!owner_wait_scope_identity(s, ticket.generation, owner) ||
      !s->pending || ticket.sequence != s->sequence) return EPERM;
  /* An aborted call may acknowledge its exit, but never publish new evidence. */
  if (s->phase == OWNER_WAIT_SCOPE_ABORTED) { s->pending = 0; return ECANCELED; }
  if (s->phase != OWNER_WAIT_SCOPE_ACTIVE) return EPERM;
  s->pending = 0;
  if (cut.wall < s->entered.wall || cut.cpu < s->entered.cpu) {
    s->invalid = 1; return EINVAL;
  }
  s->last_wall = cut.wall; s->last_cpu = cut.cpu;
  /* First progress primes a complete interval; initialization is not priced. */
  if (!s->primed) return 0;
  struct owner_wait_scope_gap *g = &s->current;
  uint64_t wall = cut.wall - s->entered.wall, cpu = cut.cpu - s->entered.cpu;
  /* Keep earlier evidence intact on counter exhaustion: compute a local copy. */
  struct owner_wait_scope_gap next = *g;
  int error = owner_wait_scope_add(&next.calls, 1) |
    owner_wait_scope_add(&next.call_wall, wall) | owner_wait_scope_add(&next.call_cpu, cpu);
  uint64_t *timeout = s->timeout_kind == OWNER_WAIT_TIMEOUT_ZERO ? &next.zero_timeout :
    s->timeout_kind == OWNER_WAIT_TIMEOUT_FINITE ? &next.finite_timeout : &next.infinite_timeout;
  error |= owner_wait_scope_add(timeout, 1);
  uint64_t *outcome = result > 0 ? &next.ready : result == 0 ? &next.timed_out :
    call_errno == EINTR ? &next.interrupted : &next.failed;
  error |= owner_wait_scope_add(outcome, 1);
  if (error) { s->invalid = 1; return EOVERFLOW; }
  *g = next;
  return 0;
}
static inline int owner_wait_scope_progress(struct owner_wait_scope *s,
    uint64_t generation, uint64_t owner, uint64_t previous, uint64_t now) {
  if (!owner_wait_scope_identity(s, generation, owner) ||
      s->phase != OWNER_WAIT_SCOPE_ACTIVE) return EPERM;
  if (s->pending || now < previous || now < s->last_wall ||
      (s->primed && previous != s->progress_end)) {
    s->invalid = 1; return EINVAL;
  }
  if (!s->primed) {
    s->primed = 1; s->progress_end = now;
    memset(&s->current, 0, sizeof(s->current));
    return 0;
  }
  if (now == previous) return 0;
  struct owner_wait_scope_gap *g = &s->current;
  g->begin = previous; g->end = now; g->present = 1; g->invalid = s->invalid;
  if (!s->best.present || now - previous > s->best.end - s->best.begin) s->best = *g;
  s->progress_end = now;
  memset(g, 0, sizeof(*g));
  return 0;
}
static inline int owner_wait_scope_close(struct owner_wait_scope *s,
    uint64_t generation, uint64_t owner, int aborting) {
  if (!owner_wait_scope_identity(s, generation, owner)) return EPERM;
  if (s->phase != OWNER_WAIT_SCOPE_ACTIVE) return EALREADY;
  if (s->pending && !aborting) return EBUSY;
  s->phase = aborting ? OWNER_WAIT_SCOPE_ABORTED : OWNER_WAIT_SCOPE_CLOSED;
  return 0;
}
static inline int owner_wait_scope_snapshot(const struct owner_wait_scope *s,
    uint64_t generation, uint64_t owner, struct owner_wait_scope_snapshot *snapshot) {
  if (!snapshot) return EINVAL;
  if (!owner_wait_scope_identity(s, generation, owner)) return EPERM;
  if (s->pending || (s->phase != OWNER_WAIT_SCOPE_CLOSED && s->phase != OWNER_WAIT_SCOPE_ABORTED))
    return EBUSY;
  *snapshot = (struct owner_wait_scope_snapshot){
    .generation = s->generation, .owner = s->owner, .unsealed_calls = s->current.calls,
    .primed = s->primed, .invalid = s->invalid || s->phase == OWNER_WAIT_SCOPE_ABORTED,
    .aborted = s->phase == OWNER_WAIT_SCOPE_ABORTED, .gap = s->best };
  /* A late invalid event must invalidate a previously captured good gap. */
  snapshot->gap.invalid |= snapshot->invalid;
  return 0;
}

#endif
