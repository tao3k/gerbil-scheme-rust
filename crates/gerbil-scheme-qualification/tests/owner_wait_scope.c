/* Exercise the exact association header, without Scheme, an SDK, or real waits. */
#include "../ffi/owner_wait_scope.h"
#include "owner_wait_recovery.h"
#include <stdio.h>

#define REQUIRE(test) do { if (!(test)) { \
  fprintf(stderr, "OWNER-WAIT-SCOPE-FAILED line %d: %s\n", __LINE__, #test); return 1; \
} } while (0)

static int paired_calls_and_outcomes(void) {
  struct owner_wait_scope s = {0};
  struct owner_wait_scope_ticket ticket;
  struct owner_wait_scope_snapshot snapshot;
  uint64_t generation = 0;
  REQUIRE(owner_wait_scope_open(&s, 7, &generation) == 0);
  REQUIRE(generation == 1);
  REQUIRE(owner_wait_scope_snapshot(&s, generation, 7, &snapshot) == EBUSY);
  /* Initialization calls are not assigned to a complete progress snapshot.gap. */
  REQUIRE(owner_wait_scope_enter(&s, generation, 7,
    (struct owner_wait_scope_cut){10, 1}, OWNER_WAIT_TIMEOUT_ZERO, &ticket) == 0);
  REQUIRE(owner_wait_scope_leave(&s, ticket, 7,
    (struct owner_wait_scope_cut){20, 2}, 0, 0) == 0);
  REQUIRE(owner_wait_scope_progress(&s, generation, 7, 1, 100) == 0);
  for (unsigned i = 0; i < 4; i++) {
    enum owner_wait_timeout_kind kind = i == 0 ? OWNER_WAIT_TIMEOUT_ZERO :
      i == 1 ? OWNER_WAIT_TIMEOUT_FINITE : OWNER_WAIT_TIMEOUT_INFINITE;
    REQUIRE(owner_wait_scope_enter(&s, generation, 7,
      (struct owner_wait_scope_cut){110 + i*20, 10 + i*2}, kind, &ticket) == 0);
    int result = i == 0 ? 0 : i == 1 ? 1 : -1;
    int error = i == 2 ? EINTR : i == 3 ? EIO : 0;
    REQUIRE(owner_wait_scope_leave(&s, ticket, 7,
      (struct owner_wait_scope_cut){120 + i*20, 11 + i*2}, result, error) == 0);
  }
  REQUIRE(owner_wait_scope_progress(&s, generation, 7, 100, 200) == 0);
  REQUIRE(owner_wait_scope_progress(&s, generation, 7, 200, 210) == 0);
  REQUIRE(owner_wait_scope_enter(&s, generation, 7,
    (struct owner_wait_scope_cut){211, 20}, OWNER_WAIT_TIMEOUT_ZERO, &ticket) == 0);
  REQUIRE(owner_wait_scope_leave(&s, ticket, 7,
    (struct owner_wait_scope_cut){212, 21}, 0, 0) == 0);
  REQUIRE(owner_wait_scope_close(&s, generation, 7, 0) == 0);
  REQUIRE(owner_wait_scope_snapshot(&s, generation, 7, &snapshot) == 0);
  REQUIRE(snapshot.gap.present && !snapshot.gap.invalid && snapshot.gap.begin == 100 && snapshot.gap.end == 200);
  REQUIRE(snapshot.gap.calls == 4 && snapshot.gap.call_wall == 40 && snapshot.gap.call_cpu == 4);
  REQUIRE(snapshot.gap.zero_timeout == 1 && snapshot.gap.finite_timeout == 1 && snapshot.gap.infinite_timeout == 2);
  REQUIRE(snapshot.gap.ready == 1 && snapshot.gap.timed_out == 1 && snapshot.gap.interrupted == 1 && snapshot.gap.failed == 1);
  REQUIRE(snapshot.unsealed_calls == 1 && owner_wait_scope_complete_gap(&snapshot));
  return 0;
}

static int generations_abort_and_late_exit(void) {
  struct owner_wait_scope s = {0};
  struct owner_wait_scope_ticket old, current;
  struct owner_wait_scope_snapshot snapshot;
  uint64_t first, second, untouched = 99;
  REQUIRE(owner_wait_scope_open(&s, 7, &first) == 0);
  REQUIRE(owner_wait_scope_progress(&s, first, 7, 0, 10) == 0);
  REQUIRE(owner_wait_scope_enter(&s, first, 7,
    (struct owner_wait_scope_cut){11, 1}, OWNER_WAIT_TIMEOUT_FINITE, &old) == 0);
  REQUIRE(owner_wait_scope_close(&s, first, 8, 1) == EPERM);
  REQUIRE(s.pending && s.phase == OWNER_WAIT_SCOPE_ACTIVE);
  REQUIRE(owner_wait_scope_close(&s, first, 7, 0) == EBUSY);
  REQUIRE(owner_wait_scope_close(&s, first, 7, 1) == 0);
  REQUIRE(owner_wait_scope_open(&s, 7, &untouched) == EBUSY && untouched == 99);
  REQUIRE(owner_wait_scope_snapshot(&s, first, 7, &snapshot) == EBUSY);
  REQUIRE(owner_wait_scope_leave(&s, old, 7,
    (struct owner_wait_scope_cut){12, 2}, 1, 0) == ECANCELED);
  REQUIRE(owner_wait_scope_snapshot(&s, first, 7, &snapshot) == 0);
  REQUIRE(snapshot.aborted && snapshot.invalid && snapshot.gap.invalid);
  REQUIRE(!owner_wait_scope_complete_gap(&snapshot));
  REQUIRE(owner_wait_scope_open(&s, 7, &second) == 0 && second > first);
  REQUIRE(owner_wait_scope_progress(&s, second, 7, 10, 20) == 0);
  REQUIRE(owner_wait_scope_enter(&s, second, 7,
    (struct owner_wait_scope_cut){21, 3}, OWNER_WAIT_TIMEOUT_ZERO, &current) == 0);
  REQUIRE(owner_wait_scope_leave(&s, old, 7,
    (struct owner_wait_scope_cut){22, 4}, 1, 0) == EPERM);
  REQUIRE(s.pending && !s.invalid && s.current.calls == 0);
  REQUIRE(owner_wait_scope_progress(&s, first, 7, 20, 30) == EPERM);
  REQUIRE(owner_wait_scope_leave(&s, current, 7,
    (struct owner_wait_scope_cut){23, 4}, 0, 0) == 0);
  REQUIRE(owner_wait_scope_leave(&s, current, 7,
    (struct owner_wait_scope_cut){24, 5}, 0, 0) == EPERM);
  REQUIRE(owner_wait_scope_progress(&s, second, 7, 20, 30) == 0);
  REQUIRE(owner_wait_scope_close(&s, second, 7, 0) == 0);
  REQUIRE(owner_wait_scope_snapshot(&s, second, 7, &snapshot) == 0);
  REQUIRE(snapshot.gap.calls == 1 && snapshot.gap.call_wall == 2 && !snapshot.gap.invalid);
  REQUIRE(!snapshot.aborted && snapshot.generation == second && snapshot.owner == 7);
  return 0;
}

static int invalid_events_cannot_be_erased_by_a_later_good_gap(void) {
  struct owner_wait_scope s = {0};
  struct owner_wait_scope_ticket ticket, unused;
  struct owner_wait_scope_snapshot snapshot;
  uint64_t generation;
  REQUIRE(owner_wait_scope_open(&s, 7, &generation) == 0);
  REQUIRE(owner_wait_scope_progress(&s, generation, 7, 0, 10) == 0);
  REQUIRE(owner_wait_scope_progress(&s, generation, 7, 10, 100) == 0);
  REQUIRE(owner_wait_scope_enter(&s, generation, 7,
    (struct owner_wait_scope_cut){101, 1}, OWNER_WAIT_TIMEOUT_ZERO, &ticket) == 0);
  REQUIRE(owner_wait_scope_enter(&s, generation, 7,
    (struct owner_wait_scope_cut){102, 2}, OWNER_WAIT_TIMEOUT_ZERO, &unused) == EBUSY);
  REQUIRE(owner_wait_scope_progress(&s, generation, 7, 100, 110) == EINVAL);
  REQUIRE(owner_wait_scope_leave(&s, ticket, 7,
    (struct owner_wait_scope_cut){103, 2}, 0, 0) == 0);
  REQUIRE(owner_wait_scope_progress(&s, generation, 7, 100, 120) == 0);
  REQUIRE(owner_wait_scope_close(&s, generation, 7, 0) == 0);
  REQUIRE(owner_wait_scope_snapshot(&s, generation, 7, &snapshot) == 0);
  REQUIRE(snapshot.gap.begin == 10 && snapshot.gap.end == 100 && snapshot.gap.invalid);
  REQUIRE(!owner_wait_scope_complete_gap(&snapshot));
  return 0;
}

static int empty_scope_does_not_fabricate_a_zero_wait_gap(void) {
  struct owner_wait_scope s = {0};
  struct owner_wait_scope_snapshot snapshot;
  uint64_t generation;
  REQUIRE(owner_wait_scope_open(&s, 7, &generation) == 0);
  REQUIRE(owner_wait_scope_close(&s, generation, 7, 0) == 0);
  REQUIRE(owner_wait_scope_snapshot(&s, generation, 7, &snapshot) == 0);
  REQUIRE(!snapshot.primed && !snapshot.gap.present && !owner_wait_scope_complete_gap(&snapshot));
  return 0;
}

static int ordering_and_counter_exhaustion_are_not_zero_cost(void) {
  struct owner_wait_scope s = {0};
  struct owner_wait_scope_ticket ticket;
  struct owner_wait_scope_snapshot snapshot;
  uint64_t generation;
  REQUIRE(owner_wait_scope_open(&s, 7, &generation) == 0);
  REQUIRE(owner_wait_scope_progress(&s, generation, 7, 0, 10) == 0);
  REQUIRE(owner_wait_scope_enter(&s, generation, 7,
    (struct owner_wait_scope_cut){11, 2}, OWNER_WAIT_TIMEOUT_ZERO, &ticket) == 0);
  REQUIRE(owner_wait_scope_leave(&s, ticket, 7,
    (struct owner_wait_scope_cut){12, 1}, 0, 0) == EINVAL);
  REQUIRE(s.invalid);
  REQUIRE(owner_wait_scope_close(&s, generation, 7, 0) == 0);
  REQUIRE(owner_wait_scope_open(&s, 7, &generation) == 0);
  REQUIRE(owner_wait_scope_progress(&s, generation, 7, 0, 10) == 0);
  REQUIRE(owner_wait_scope_enter(&s, generation, 7,
    (struct owner_wait_scope_cut){11, 2}, OWNER_WAIT_TIMEOUT_ZERO, &ticket) == 0);
  s.current.calls = UINT64_MAX;
  REQUIRE(owner_wait_scope_leave(&s, ticket, 7,
    (struct owner_wait_scope_cut){12, 3}, 0, 0) == EOVERFLOW);
  REQUIRE(s.current.calls == UINT64_MAX && s.current.call_wall == 0 && s.invalid);
  REQUIRE(owner_wait_scope_progress(&s, generation, 7, 10, 20) == 0);
  REQUIRE(owner_wait_scope_close(&s, generation, 7, 0) == 0);
  REQUIRE(owner_wait_scope_snapshot(&s, generation, 7, &snapshot) == 0 && snapshot.gap.invalid);
  s.generation = UINT64_MAX;
  generation = 123;
  REQUIRE(owner_wait_scope_open(&s, 7, &generation) == EOVERFLOW && generation == 123);
  return 0;
}

static int progress_and_call_timestamps_cannot_reorder(void) {
  struct owner_wait_scope s = {0};
  struct owner_wait_scope_ticket ticket;
  uint64_t generation;
  REQUIRE(owner_wait_scope_open(&s, 7, &generation) == 0);
  REQUIRE(owner_wait_scope_progress(&s, generation, 7, 0, 10) == 0);
  REQUIRE(owner_wait_scope_enter(&s, generation, 7,
    (struct owner_wait_scope_cut){11, 1}, OWNER_WAIT_TIMEOUT_ZERO, &ticket) == 0);
  REQUIRE(owner_wait_scope_leave(&s, ticket, 7,
    (struct owner_wait_scope_cut){20, 2}, 0, 0) == 0);
  REQUIRE(owner_wait_scope_progress(&s, generation, 7, 10, 19) == EINVAL);
  REQUIRE(owner_wait_scope_enter(&s, generation, 7,
    (struct owner_wait_scope_cut){19, 3}, OWNER_WAIT_TIMEOUT_ZERO, &ticket) == EINVAL);
  REQUIRE(owner_wait_scope_progress(&s, generation, 7, 9, 21) == EINVAL);
  REQUIRE(owner_wait_scope_progress(&s, generation, 7, 10, 21) == 0);
  REQUIRE(s.invalid);
  return 0;
}

static struct owner_scope_clock recovery_clock(uint64_t wall) {
  return (struct owner_scope_clock){.begin=wall,.end=wall,.cpu=wall,.valid=1};
}
static struct owner_recovery_point recovery_point(uint64_t wall,uint64_t due,uint64_t published,uint64_t consumed) {
  return (struct owner_recovery_point){wall,due,published,consumed,consumed,1};
}
static int actual_peak_recovery_keeps_its_own_episode(void) {
  struct owner_wait_recovery r={.guard=3};
  REQUIRE(owner_recovery_source(&r,recovery_point(10,10,9,9),recovery_clock(8),recovery_clock(6))==0);
  REQUIRE(owner_recovery_source(&r,recovery_point(20,20,12,12),recovery_clock(18),recovery_clock(8))==0);
  owner_recovery_sample(&r,recovery_clock(21));
  REQUIRE(r.peak.after.valid && r.peak.point.wall==20);
  REQUIRE(owner_recovery_source(&r,recovery_point(30,30,13,13),recovery_clock(28),recovery_clock(21))==0);
  REQUIRE(r.selected_start.point.wall==10 && r.selected_start.before.end==8);
  REQUIRE(r.peak.point.wall==30 && !r.peak.after.valid && !r.recovered.point.valid);
  owner_recovery_sample(&r,recovery_clock(31));
  REQUIRE(owner_recovery_source(&r,recovery_point(40,40,40,38),recovery_clock(39),recovery_clock(31))==0);
  REQUIRE(r.recovered.point.wall==40 && !r.recovered.after.valid);
  owner_recovery_sample(&r,recovery_clock(41));
  REQUIRE(r.peak.after.end==31 && r.recovered.after.end==41);
  REQUIRE(owner_recovery_source(&r,recovery_point(50,50,40,40),recovery_clock(49),recovery_clock(41))==0);
  REQUIRE(owner_recovery_source(&r,recovery_point(60,60,60,59),recovery_clock(59),recovery_clock(49))==0);
  REQUIRE(r.peak.point.wall==30 && r.recovered.point.wall==40 && r.selected_episode==1);
  REQUIRE(owner_recovery_value(&r,"peak-due")==30 && owner_recovery_value(&r,"end-after-valid")==1);
  REQUIRE(owner_recovery_source(&r,recovery_point(70,90,60,60),recovery_clock(69),recovery_clock(59))==0);
  REQUIRE(r.peak.point.wall==70 && !r.recovered.point.valid && !r.peak.after.valid);
  REQUIRE(r.selected_start.point.wall==60 && r.selected_episode==3);
  REQUIRE(owner_recovery_source(&r,recovery_point(71,89,61,61),recovery_clock(69),recovery_clock(59))==EINVAL);
  REQUIRE(r.invalid && r.peak.point.wall==70);
  REQUIRE(owner_recovery_source(&r,recovery_point(72,92,62,62),recovery_clock(69),recovery_clock(59))==EINVAL);
  REQUIRE(r.peak.point.wall==70);
  return 0;
}
static int partial_recovery_cannot_invent_a_bracket_or_return(void) {
  struct owner_wait_recovery r={.guard=3};
  REQUIRE(owner_recovery_source(&r,recovery_point(10,10,0,0),recovery_clock(8),recovery_clock(6))==0);
  REQUIRE(!r.selected_start.point.valid && r.peak.point.valid && !r.recovered.point.valid);
  owner_recovery_sample(&r,(struct owner_scope_clock){0});
  REQUIRE(!r.peak.after.valid);
  r=(struct owner_wait_recovery){0};
  REQUIRE(owner_recovery_source(&r,recovery_point(10,10,0,0),recovery_clock(8),recovery_clock(6))==0);
  REQUIRE(!r.peak.point.valid); /* no configured window, no implicit default */
  return 0;
}
int main(void) {
  if (paired_calls_and_outcomes() || generations_abort_and_late_exit() ||
      invalid_events_cannot_be_erased_by_a_later_good_gap() ||
      ordering_and_counter_exhaustion_are_not_zero_cost() ||
      progress_and_call_timestamps_cannot_reorder() ||
      empty_scope_does_not_fabricate_a_zero_wait_gap() ||
      actual_peak_recovery_keeps_its_own_episode() ||
      partial_recovery_cannot_invent_a_bracket_or_return()) return 1;
  puts("OWNER-WAIT-SCOPE-CONTRACT-OK");
  return 0;
}
