/* TEST-ONLY record from a rejected observer. Do not restore its runtime hook
 * based on correctness checks alone; that candidate failed p99 and debt gates.
 * Observation only: bounded source/consumption recovery around the actual
 * global debt peak. Caller supplies existing timestamps and sparse CPU cuts.
 * No clock, allocation, callback, lock, wake or capacity mutation here. */
#ifndef GERBIL_OWNER_WAIT_RECOVERY_H
#define GERBIL_OWNER_WAIT_RECOVERY_H
#include <stdint.h>
#include <string.h>
#include <errno.h>
struct owner_scope_clock { uint64_t begin, end, cpu, gc_count, gc_wall, gc_cpu; int valid; };
struct owner_recovery_point {
  uint64_t wall, due, published, consumed, ordinal;
  int valid;
};
struct owner_recovery_marker {
  struct owner_recovery_point point;
  struct owner_scope_clock before, after;
};
struct owner_wait_recovery {
  uint64_t guard, episode, selected_episode;
  int active, invalid;
  struct owner_recovery_point last, stable, start;
  struct owner_scope_clock start_before;
  struct owner_recovery_marker selected_start, peak, recovered;
};
static inline struct owner_scope_clock owner_recovery_before(
    uint64_t wall, struct owner_scope_clock current, struct owner_scope_clock previous) {
  if (current.valid && current.end<=wall) return current;
  if (previous.valid && previous.end<=wall) return previous;
  return (struct owner_scope_clock){0};
}
static inline void owner_recovery_sample(struct owner_wait_recovery *r, struct owner_scope_clock c) {
  if (!c.valid || r->invalid) return;
  if (r->peak.point.valid && !r->peak.after.valid && r->peak.point.wall<=c.begin) r->peak.after=c;
  if (r->recovered.point.valid && !r->recovered.after.valid && r->recovered.point.wall<=c.begin) r->recovered.after=c;
}
static inline int owner_recovery_source(struct owner_wait_recovery *r,
    struct owner_recovery_point point, struct owner_scope_clock current, struct owner_scope_clock previous) {
  if (!r->guard) return 0; /* not configured; never invent a default */
  if (r->invalid) return EINVAL;
  if (!point.valid || point.consumed>point.published || point.published>point.due ||
      (r->last.valid && (point.wall<r->last.wall || point.due<r->last.due ||
       point.published<r->last.published || point.consumed<r->last.consumed || point.ordinal<r->last.ordinal))) {
    r->invalid=1; return EINVAL;
  }
  struct owner_scope_clock before=owner_recovery_before(point.wall,current,previous);
  uint64_t debt=point.due-point.consumed;
  if (debt>r->guard) {
    if (!r->active) {
      if (r->episode==UINT64_MAX) {r->invalid=1;return EOVERFLOW;}
      r->episode++; r->active=1; r->start=r->stable;
      r->start_before=owner_recovery_before(r->stable.wall,current,previous);
    }
    if (!r->peak.point.valid || debt>r->peak.point.due-r->peak.point.consumed) {
      r->selected_episode=r->episode;
      r->selected_start=(struct owner_recovery_marker){.point=r->start,.before=r->start_before};
      r->peak=(struct owner_recovery_marker){.point=point,.before=before};
      r->recovered=(struct owner_recovery_marker){0};
    }
  } else {
    if (r->active && r->selected_episode==r->episode)
      r->recovered=(struct owner_recovery_marker){.point=point,.before=before};
    r->active=0; r->stable=point;
  }
  r->last=point;
  return 0;
}
/* Named cold projection; partial brackets remain explicitly invalid. */
static inline uint64_t owner_recovery_clock_value(const struct owner_scope_clock *c, const char *name) {
  if (!strcmp(name,"valid")) return (uint64_t)c->valid;
#define RECOVERY_CLOCK(member) if (!strcmp(name,#member)) return c->member;
  RECOVERY_CLOCK(begin) RECOVERY_CLOCK(end) RECOVERY_CLOCK(cpu)
  RECOVERY_CLOCK(gc_count) RECOVERY_CLOCK(gc_wall) RECOVERY_CLOCK(gc_cpu)
#undef RECOVERY_CLOCK
  return UINT64_MAX;
}
static inline uint64_t owner_recovery_marker_value(const struct owner_recovery_marker *m,const char *name) {
  if (!strncmp(name,"before-",7)) return owner_recovery_clock_value(&m->before,name+7);
  if (!strncmp(name,"after-",6)) return owner_recovery_clock_value(&m->after,name+6);
  if (!strcmp(name,"valid")) return (uint64_t)m->point.valid;
#define RECOVERY_POINT(member) if (!strcmp(name,#member)) return m->point.member;
  RECOVERY_POINT(wall) RECOVERY_POINT(due) RECOVERY_POINT(published)
  RECOVERY_POINT(consumed) RECOVERY_POINT(ordinal)
#undef RECOVERY_POINT
  return UINT64_MAX;
}
static inline uint64_t owner_recovery_value(const struct owner_wait_recovery *r,const char *name) {
  if (!strcmp(name,"guard")) return r->guard;
  if (!strcmp(name,"invalid")) return (uint64_t)r->invalid;
  if (!strcmp(name,"episode")) return r->selected_episode;
  if (!strncmp(name,"start-",6)) return owner_recovery_marker_value(&r->selected_start,name+6);
  if (!strncmp(name,"peak-",5)) return owner_recovery_marker_value(&r->peak,name+5);
  if (!strncmp(name,"end-",4)) return owner_recovery_marker_value(&r->recovered,name+4);
  return UINT64_MAX;
}
#endif
