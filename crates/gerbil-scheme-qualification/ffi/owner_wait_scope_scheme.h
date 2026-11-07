/* Image-local typed bridge for the selected Scheme diagnostic, never default. */
#ifndef GERBIL_OWNER_WAIT_SCOPE_SCHEME_H
#define GERBIL_OWNER_WAIT_SCOPE_SCHEME_H
extern uint64_t gerbil_owner_scope_gc_count(void);
extern uint64_t gerbil_owner_scope_gc_wall(void);
extern uint64_t gerbil_owner_scope_gc_cpu(void);
struct owner_scope_clock { uint64_t begin, end, cpu, gc_count, gc_wall, gc_cpu; int valid; };
static _Thread_local struct {
  uint64_t period, ordinal, best_ordinal, source_epoch;
  int error, clock_valid;
  struct owner_wait_scope_snapshot snapshot;
  struct owner_scope_clock current, previous, before, after;
  struct { uint64_t wall,due,published,consumed,active,progress_ordinal,outstanding; } peak;
} owner_scope_scheme;
static struct owner_scope_clock owner_scope_clock_read(void) {
  struct owner_scope_clock c={0}; struct timespec wall,cpu;
  if (clock_gettime(CLOCK_UPTIME_RAW,&wall)) return c;
  c.begin=(uint64_t)wall.tv_sec*1000000000ULL+(uint64_t)wall.tv_nsec;
  if (clock_gettime(CLOCK_THREAD_CPUTIME_ID,&cpu)) return c;
  c.cpu=(uint64_t)cpu.tv_sec*1000000000ULL+(uint64_t)cpu.tv_nsec;
  c.gc_count=gerbil_owner_scope_gc_count(); c.gc_wall=gerbil_owner_scope_gc_wall();
  c.gc_cpu=gerbil_owner_scope_gc_cpu();
  if (clock_gettime(CLOCK_UPTIME_RAW,&wall)) return c;
  c.end=(uint64_t)wall.tv_sec*1000000000ULL+(uint64_t)wall.tv_nsec;
  c.valid=c.begin<=c.end && c.gc_wall!=UINT64_MAX && c.gc_cpu!=UINT64_MAX;
  return c;
}
static int owner_scope_generation(uint64_t generation) {
  return owner_wait_scope_identity(&owner_wait_association,generation,owner_wait_binding_owner()) &&
    owner_wait_association.phase==OWNER_WAIT_SCOPE_ACTIVE;
}
static uint64_t owner_scope_scheme_publish(uint64_t generation,uint64_t address) {
  /* Keep the original clock, five-second gate and release-store heartbeat.
   * No added clock read unless the original observation window elapsed. */
  uint64_t *word=(uint64_t *)(uintptr_t)address;
  uint64_t now=owner_wait_clock(), last=__atomic_load_n(word,__ATOMIC_ACQUIRE);
  if (now-last>=5000000000ULL) return UINT64_MAX;
  __atomic_store_n(word,now,__ATOMIC_RELEASE);
  gerbil_mailbox_native_progress(last,now);
  if (!owner_scope_generation(generation)) return 0; /* closed/stale observation is inert */
  struct owner_wait_scope *s=&owner_wait_association;
  uint64_t observed_previous=owner_scope_scheme.source_epoch && !owner_scope_scheme.ordinal ?
    owner_scope_scheme.source_epoch : last;
  int largest=s->primed && now>observed_previous &&
    (!s->best.present || now-observed_previous>s->best.end-s->best.begin);
  struct owner_scope_clock before=owner_scope_scheme.current.end<=observed_previous ?
    owner_scope_scheme.current : owner_scope_scheme.previous;
  struct owner_scope_clock after={0};
  int sample=now>=owner_scope_scheme.current.end &&
    now-owner_scope_scheme.current.end>=owner_scope_scheme.period;
  if (sample) after=owner_scope_clock_read();
  int status=owner_wait_binding_progress((struct owner_wait_scope_handle){generation,owner_wait_binding_owner()},observed_previous,now);
  if (status) { owner_scope_scheme.error=status; s->invalid=1; return 0; }
  owner_scope_scheme.ordinal++;
  if (largest) {
    owner_scope_scheme.best_ordinal=owner_scope_scheme.ordinal;
    owner_scope_scheme.before=before; owner_scope_scheme.after=after;
    owner_scope_scheme.clock_valid=sample && before.valid && after.valid &&
      before.end<=observed_previous && now<=after.begin && before.cpu<=after.cpu &&
      before.gc_count<=after.gc_count && before.gc_wall<=after.gc_wall && before.gc_cpu<=after.gc_cpu;
  }
  if (sample && after.valid) {
    owner_scope_scheme.previous=owner_scope_scheme.current; owner_scope_scheme.current=after;
  }
  return 0;
}
static void owner_scope_scheme_source(uint64_t generation,uint64_t now,uint64_t due,
    uint64_t published,uint64_t consumed,uint64_t active) {
  if (!owner_scope_generation(generation)) return;
  if (consumed>published || published>due || published-consumed!=active) {
    owner_wait_association.invalid=1; owner_scope_scheme.error=EINVAL; return;
  }
  if (due-consumed>owner_scope_scheme.peak.outstanding) {
    owner_scope_scheme.peak.wall=now; owner_scope_scheme.peak.due=due;
    owner_scope_scheme.peak.published=published; owner_scope_scheme.peak.consumed=consumed;
    owner_scope_scheme.peak.active=active; owner_scope_scheme.peak.outstanding=due-consumed;
    owner_scope_scheme.peak.progress_ordinal=owner_scope_scheme.ordinal;
  }
}
static uint64_t owner_scope_scheme_control(const char *name,uint64_t generation,uint64_t value) {
  if (!strcmp(name,"supported")) return 1;
  if (!strcmp(name,"open")) {
    if (!value) return 0;
    struct owner_wait_scope_handle handle;
    int status=owner_wait_binding_open(&handle);
    if (status) return 0; /* failed admission owns no close */
    memset(&owner_scope_scheme,0,sizeof(owner_scope_scheme));
    owner_scope_scheme.period=value;
    owner_scope_scheme.current=owner_scope_scheme.previous=owner_scope_clock_read();
    if (!owner_scope_scheme.current.valid) owner_wait_association.invalid=1;
    return handle.generation;
  }
  if (!strcmp(name,"active")) return (uint64_t)owner_scope_generation(generation);
  if (!strcmp(name,"live-invalid")) return owner_scope_generation(generation) ?
    (uint64_t)owner_wait_association.invalid : UINT64_MAX;
  if (!strcmp(name,"live-progress-count")) return owner_scope_generation(generation) ?
    owner_scope_scheme.ordinal : UINT64_MAX;
  if (!strcmp(name,"epoch")) {
    if (!owner_scope_generation(generation) || owner_wait_association.primed || !value) return EINVAL;
    int status=owner_wait_binding_progress(
      (struct owner_wait_scope_handle){generation,owner_wait_binding_owner()},value,value);
    if (!status) owner_scope_scheme.source_epoch=value;
    return (uint64_t)status;
  }
  if (!strcmp(name,"close") || !strcmp(name,"abort")) {
    return (uint64_t)owner_wait_binding_finish(
      (struct owner_wait_scope_handle){generation,owner_wait_binding_owner()},
      !strcmp(name,"abort"),&owner_scope_scheme.snapshot);
  }
  const struct owner_wait_scope_snapshot *s=&owner_scope_scheme.snapshot;
  if (s->generation!=generation || s->owner!=owner_wait_binding_owner()) return UINT64_MAX;
  if (!strcmp(name,"generation")) return s->generation;
  if (!strcmp(name,"source-epoch")) return owner_scope_scheme.source_epoch;
  if (!strcmp(name,"complete")) return (uint64_t)owner_wait_scope_complete_gap(s);
  if (!strcmp(name,"aborted")) return (uint64_t)s->aborted;
  if (!strcmp(name,"invalid")) return (uint64_t)s->invalid;
  if (!strcmp(name,"error")) return (uint64_t)owner_scope_scheme.error;
  if (!strcmp(name,"unsealed-calls")) return s->unsealed_calls;
  if (!strcmp(name,"gap-begin")) return s->gap.begin;
  if (!strcmp(name,"gap-end")) return s->gap.end;
  if (!strcmp(name,"calls")) return s->gap.calls;
  if (!strcmp(name,"call-wall")) return s->gap.call_wall;
  if (!strcmp(name,"call-cpu")) return s->gap.call_cpu;
  if (!strcmp(name,"zero-timeout")) return s->gap.zero_timeout;
  if (!strcmp(name,"finite-timeout")) return s->gap.finite_timeout;
  if (!strcmp(name,"infinite-timeout")) return s->gap.infinite_timeout;
  if (!strcmp(name,"ready")) return s->gap.ready;
  if (!strcmp(name,"timed-out")) return s->gap.timed_out;
  if (!strcmp(name,"interrupted")) return s->gap.interrupted;
  if (!strcmp(name,"failed")) return s->gap.failed;
  if (!strcmp(name,"clock-valid")) return (uint64_t)owner_scope_scheme.clock_valid;
  if (!strcmp(name,"best-progress-ordinal")) return owner_scope_scheme.best_ordinal;
  if (!strcmp(name,"peak-wall")) return owner_scope_scheme.peak.wall;
  if (!strcmp(name,"peak-due")) return owner_scope_scheme.peak.due;
  if (!strcmp(name,"peak-published")) return owner_scope_scheme.peak.published;
  if (!strcmp(name,"peak-consumed")) return owner_scope_scheme.peak.consumed;
  if (!strcmp(name,"peak-active")) return owner_scope_scheme.peak.active;
  if (!strcmp(name,"peak-outstanding")) return owner_scope_scheme.peak.outstanding;
  if (!strcmp(name,"peak-progress-ordinal")) return owner_scope_scheme.peak.progress_ordinal;
#define SCOPE_CLOCK_FIELD(label,member) \
  if (!strcmp(name,"before-" label)) return owner_scope_scheme.before.member; \
  if (!strcmp(name,"after-" label)) return owner_scope_scheme.after.member;
  SCOPE_CLOCK_FIELD("begin",begin) SCOPE_CLOCK_FIELD("end",end) SCOPE_CLOCK_FIELD("cpu",cpu)
  SCOPE_CLOCK_FIELD("gc-count",gc_count) SCOPE_CLOCK_FIELD("gc-wall",gc_wall) SCOPE_CLOCK_FIELD("gc-cpu",gc_cpu)
#undef SCOPE_CLOCK_FIELD
  return UINT64_MAX;
}
#endif
