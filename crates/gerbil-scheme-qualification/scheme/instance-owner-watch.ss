;; Native facts only. No Scheme phase catalog, callbacks or GC-managed pointers.
;; The sole borrowed address is the existing aligned atomic completion word.
(extern mailbox-owner-watch-start! mailbox-owner-watch-stop!)
(begin-foreign
 (namespace ("gerbil-scheme-rust/qualification/actors#"
   mailbox-owner-watch-start! mailbox-owner-watch-stop!))
 (c-declare #<<c-end
#ifdef __APPLE__
#define GERBIL_MAILBOX_OWNER_READING(value) value
#include <mach/mach.h>
#include <mach/mach_time.h>
#include <pthread.h>
#include <pthread/qos.h>
#include <stdint.h>
#include <string.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
struct mailbox_owner_gap {
  uint64_t begin, end, first_sample, last_sample, samples;
  uint64_t runnable, waiting, uninterruptible, stopped, other;
  uint64_t select_samples, select_waiting, first_cpu, last_cpu, largest_spacing;
};
struct mailbox_owner_clock {
  uint64_t begin, end, cpu, gc_count, gc_wall, gc_cpu;
};
struct mailbox_owner_completion_gap {
  uint64_t begin, end;
  struct mailbox_owner_clock before, after;
};
static struct {
  pthread_t observer, owner;
  mach_port_t port;
  const uint64_t *progress;
  uint64_t period, ticks, started, query_errors, qos, largest_poll_spacing;
  uint64_t assigned_qos, qos_query_status;
  int active, stop;
  uint64_t select_epoch;
  uint64_t witness_begin;
  struct mailbox_owner_clock current_clock, previous_clock;
  struct mailbox_owner_completion_gap witnessed;
  struct mailbox_owner_gap best;
} mailbox_owner_watch;
static _Thread_local int mailbox_owner_watch_owner;

static void mailbox_owner_read_clock(struct mailbox_owner_clock *sample) {
  sample->begin = mailbox_clock_now();
  sample->cpu = mailbox_cpu_now();
  sample->gc_count = (uint64_t)___VMSTATE->mem.nb_gcs_;
  sample->gc_wall = (uint64_t)(___VMSTATE->mem.gc_real_time_ * 1000000000.0);
  sample->gc_cpu = (uint64_t)((___VMSTATE->mem.gc_user_time_ + ___VMSTATE->mem.gc_sys_time_) * 1000000000.0);
  sample->end = mailbox_clock_now();
}
/* Extend the EXISTING verified-completion transaction. Only a fast predicate
 * runs per completion; self CPU/GC reads run once per original source window or
 * at the end of a witnessed stall. No progress notification is manufactured. */
void gerbil_mailbox_native_progress(uint64_t previous, uint64_t now) {
  if (!mailbox_owner_watch_owner) return;
  int witness = previous >= mailbox_owner_watch.started &&
    previous == __atomic_load_n(&mailbox_owner_watch.witness_begin, __ATOMIC_ACQUIRE);
  if (witness || now - mailbox_owner_watch.current_clock.end >= mailbox_owner_watch.period) {
    struct mailbox_owner_clock after;
    mailbox_owner_read_clock(&after);
    if (witness) {
      mailbox_owner_watch.witnessed.begin = previous; mailbox_owner_watch.witnessed.end = now;
      mailbox_owner_watch.witnessed.before = mailbox_owner_watch.current_clock.end <= previous
        ? mailbox_owner_watch.current_clock : mailbox_owner_watch.previous_clock;
      mailbox_owner_watch.witnessed.after = after;
    }
    mailbox_owner_watch.previous_clock = mailbox_owner_watch.current_clock;
    mailbox_owner_watch.current_clock = after;
  }
}

/* Called only around this image's existing select wrapper. The epoch makes
 * sampled entry/exit races explicit. These functions do not alter arguments,
 * deadlines, readiness or errno visible to the actual caller. */
void gerbil_mailbox_native_wait_enter(void) {
  if (mailbox_owner_watch_owner)
    __atomic_add_fetch(&mailbox_owner_watch.select_epoch, 1, __ATOMIC_RELEASE);
}
void gerbil_mailbox_native_wait_exit(void) {
  if (mailbox_owner_watch_owner)
    __atomic_add_fetch(&mailbox_owner_watch.select_epoch, 1, __ATOMIC_RELEASE);
}
static void mailbox_owner_keep_gap(struct mailbox_owner_gap *gap, uint64_t end) {
  (void)end;
  if (!gap->samples) return;
  /* This is a witnessed lower bound, not an invented first-return timestamp.
   * A busy owner may complete many messages between two observer polls. */
  gap->end = gap->last_sample;
  if (gap->end - gap->begin > mailbox_owner_watch.best.end - mailbox_owner_watch.best.begin)
    mailbox_owner_watch.best = *gap;
}
static void *mailbox_owner_sample(void *unused) {
  (void)unused;
  uint64_t previous = mailbox_owner_watch.started;
  uint64_t last_poll = previous;
  struct mailbox_owner_gap gap;
  memset(&gap, 0, sizeof(gap)); gap.begin = previous;
  while (!__atomic_load_n(&mailbox_owner_watch.stop, __ATOMIC_ACQUIRE)) {
    uint64_t progress = __atomic_load_n(mailbox_owner_watch.progress, __ATOMIC_ACQUIRE);
    uint64_t now = mailbox_clock_now();
    if (now - last_poll > mailbox_owner_watch.largest_poll_spacing)
      mailbox_owner_watch.largest_poll_spacing = now - last_poll;
    last_poll = now;
    if (progress > previous) {
      mailbox_owner_keep_gap(&gap, progress);
      previous = progress;
      memset(&gap, 0, sizeof(gap)); gap.begin = previous;
    }
    if (now >= previous && now - previous >= mailbox_owner_watch.period) {
      if (now - previous > mailbox_owner_watch.best.end - mailbox_owner_watch.best.begin)
        __atomic_store_n(&mailbox_owner_watch.witness_begin, previous, __ATOMIC_RELEASE);
      thread_basic_info_data_t info;
      mach_msg_type_number_t count = THREAD_BASIC_INFO_COUNT;
      uint64_t before = __atomic_load_n(&mailbox_owner_watch.select_epoch, __ATOMIC_ACQUIRE);
      kern_return_t status = thread_info(mailbox_owner_watch.port, THREAD_BASIC_INFO,
                                        (thread_info_t)&info, &count);
      uint64_t after = __atomic_load_n(&mailbox_owner_watch.select_epoch, __ATOMIC_ACQUIRE);
      uint64_t sampled = mailbox_clock_now();
      uint64_t still = __atomic_load_n(mailbox_owner_watch.progress, __ATOMIC_ACQUIRE);
      if (status != KERN_SUCCESS) mailbox_owner_watch.query_errors++;
      else if (still <= previous) {
        uint64_t cpu = ((uint64_t)info.user_time.seconds + (uint64_t)info.system_time.seconds) * 1000000000ULL
          + ((uint64_t)info.user_time.microseconds + (uint64_t)info.system_time.microseconds) * 1000ULL;
        if (!gap.samples) { gap.first_sample = sampled; gap.first_cpu = cpu; }
        else if (sampled - gap.last_sample > gap.largest_spacing) gap.largest_spacing = sampled - gap.last_sample;
        gap.last_sample = sampled; gap.last_cpu = cpu; gap.samples++;
        switch (info.run_state) {
          case TH_STATE_RUNNING: gap.runnable++; break;
          case TH_STATE_WAITING: gap.waiting++; break;
          case TH_STATE_UNINTERRUPTIBLE: gap.uninterruptible++; break;
          case TH_STATE_STOPPED: gap.stopped++; break;
          default: gap.other++; break;
        }
        if (before == after && (before & 1)) {
          gap.select_samples++;
          if (info.run_state == TH_STATE_WAITING || info.run_state == TH_STATE_UNINTERRUPTIBLE)
            gap.select_waiting++;
        }
      }
    }
    mach_wait_until(mach_absolute_time() + mailbox_owner_watch.ticks);
  }
  uint64_t end = __atomic_load_n(mailbox_owner_watch.progress, __ATOMIC_ACQUIRE);
  if (end >= previous) mailbox_owner_keep_gap(&gap, end);
  return NULL;
}
static int mailbox_owner_watch_start(uint64_t address, uint64_t period) {
  if (mailbox_owner_watch.active) return EBUSY;
  if (!address || address % _Alignof(uint64_t) || !period) return EINVAL;
  memset(&mailbox_owner_watch, 0, sizeof(mailbox_owner_watch));
  mach_timebase_info_data_t timebase;
  if (mach_timebase_info(&timebase) != KERN_SUCCESS) return EIO;
  mailbox_owner_watch.period = period;
  mailbox_owner_watch.ticks = (uint64_t)((long double)period * timebase.denom / timebase.numer);
  if (!mailbox_owner_watch.ticks) return EINVAL;
  mailbox_owner_watch.progress = (const uint64_t *)(uintptr_t)address;
  mailbox_owner_watch.owner = pthread_self();
  mailbox_owner_watch.port = mach_thread_self();
  mailbox_owner_watch.qos = (uint64_t)qos_class_self();
  qos_class_t assigned_qos = QOS_CLASS_UNSPECIFIED;
  int relative_qos = 0;
  mailbox_owner_watch.qos_query_status = (uint64_t)pthread_get_qos_class_np(pthread_self(), &assigned_qos, &relative_qos);
  mailbox_owner_watch.assigned_qos = (uint64_t)assigned_qos;
  mailbox_owner_watch.started = mailbox_clock_now();
  mailbox_owner_read_clock(&mailbox_owner_watch.current_clock);
  mailbox_owner_watch.previous_clock = mailbox_owner_watch.current_clock;
  mailbox_owner_watch_owner = 1;
  int status = pthread_create(&mailbox_owner_watch.observer, NULL, mailbox_owner_sample, NULL);
  if (status) {
    mailbox_owner_watch_owner = 0;
    mach_port_deallocate(mach_task_self(), mailbox_owner_watch.port);
    mailbox_owner_watch.progress = NULL;
    return status;
  }
  mailbox_owner_watch.active = 1;
  return 0;
}
static int mailbox_owner_watch_stop(void) {
  if (!mailbox_owner_watch.active) return 0;
  if (!pthread_equal(pthread_self(), mailbox_owner_watch.owner)) return EPERM;
  __atomic_store_n(&mailbox_owner_watch.stop, 1, __ATOMIC_RELEASE);
  int status = pthread_join(mailbox_owner_watch.observer, NULL);
  if (status) {
    /* Never return a borrowed progress word while a reader may still exist. */
    fprintf(stderr, "NATIVE-OBSERVER-JOIN-FAILED status=%d cleanup=UNVERIFIED\n", status);
    abort();
  }
  mailbox_owner_watch_owner = 0;
  kern_return_t released = mach_port_deallocate(mach_task_self(), mailbox_owner_watch.port);
  mailbox_owner_watch.progress = NULL; mailbox_owner_watch.active = 0;
  return released == KERN_SUCCESS ? 0 : EIO;
}
#else
#include <stdint.h>
#include <errno.h>
/* Kernel observation is Darwin-only. Never turn missing facts into zeros. */
#define GERBIL_MAILBOX_OWNER_READING(value) UINT64_MAX
static int mailbox_owner_watch_start(uint64_t address, uint64_t period) {
  (void)address; (void)period;
  return ENOTSUP;
}
static int mailbox_owner_watch_stop(void) { return ENOTSUP; }
void gerbil_mailbox_native_progress(uint64_t previous, uint64_t now) {
  (void)previous; (void)now;
}
void gerbil_mailbox_native_wait_enter(void) {}
void gerbil_mailbox_native_wait_exit(void) {}
#endif
c-end
 )
 (define mailbox-owner-watch-start! (c-lambda (unsigned-int64 unsigned-int64) int "mailbox_owner_watch_start"))
 (define mailbox-owner-watch-stop! (c-lambda () int "mailbox_owner_watch_stop")))

;; These are native kernel facts, read only after pthread_join. No serialized
;; record width or stage identifiers cross the existing native object ABI.
(defrules define-mailbox-owner-reading ()
  ((_ name expression)
   (begin (extern name)
     (begin-foreign
       (namespace ("gerbil-scheme-rust/qualification/actors#" name))
       (define name (c-lambda () unsigned-int64 expression))))))
(define-mailbox-owner-reading mailbox-owner-gap-begin "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.best.begin);")
(define-mailbox-owner-reading mailbox-owner-gap-end "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.best.end);")
(define-mailbox-owner-reading mailbox-owner-first-sample "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.best.first_sample);")
(define-mailbox-owner-reading mailbox-owner-last-sample "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.best.last_sample);")
(define-mailbox-owner-reading mailbox-owner-samples "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.best.samples);")
(define-mailbox-owner-reading mailbox-owner-runnable "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.best.runnable);")
(define-mailbox-owner-reading mailbox-owner-waiting "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.best.waiting);")
(define-mailbox-owner-reading mailbox-owner-uninterruptible "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.best.uninterruptible);")
(define-mailbox-owner-reading mailbox-owner-stopped "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.best.stopped);")
(define-mailbox-owner-reading mailbox-owner-other "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.best.other);")
(define-mailbox-owner-reading mailbox-owner-select-samples "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.best.select_samples);")
(define-mailbox-owner-reading mailbox-owner-select-waiting "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.best.select_waiting);")
(define-mailbox-owner-reading mailbox-owner-first-cpu "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.best.first_cpu);")
(define-mailbox-owner-reading mailbox-owner-last-cpu "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.best.last_cpu);")
(define-mailbox-owner-reading mailbox-owner-sample-spacing "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.best.largest_spacing);")
(define-mailbox-owner-reading mailbox-owner-query-errors "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.query_errors);")
(define-mailbox-owner-reading mailbox-owner-qos "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.qos);")
(define-mailbox-owner-reading mailbox-owner-assigned-qos "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.assigned_qos);")
(define-mailbox-owner-reading mailbox-owner-qos-status "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.qos_query_status);")
(define-mailbox-owner-reading mailbox-owner-poll-spacing "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.largest_poll_spacing);")
(define-mailbox-owner-reading mailbox-owner-active "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.active);")
(define-mailbox-owner-reading mailbox-owner-witness-begin "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.witnessed.begin);")
(define-mailbox-owner-reading mailbox-owner-witness-end "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.witnessed.end);")
(define-mailbox-owner-reading mailbox-owner-before-begin "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.witnessed.before.begin);")
(define-mailbox-owner-reading mailbox-owner-before-end "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.witnessed.before.end);")
(define-mailbox-owner-reading mailbox-owner-before-cpu "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.witnessed.before.cpu);")
(define-mailbox-owner-reading mailbox-owner-before-gcs "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.witnessed.before.gc_count);")
(define-mailbox-owner-reading mailbox-owner-before-gc-wall "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.witnessed.before.gc_wall);")
(define-mailbox-owner-reading mailbox-owner-before-gc-cpu "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.witnessed.before.gc_cpu);")
(define-mailbox-owner-reading mailbox-owner-after-begin "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.witnessed.after.begin);")
(define-mailbox-owner-reading mailbox-owner-after-end "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.witnessed.after.end);")
(define-mailbox-owner-reading mailbox-owner-after-cpu "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.witnessed.after.cpu);")
(define-mailbox-owner-reading mailbox-owner-after-gcs "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.witnessed.after.gc_count);")
(define-mailbox-owner-reading mailbox-owner-after-gc-wall "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.witnessed.after.gc_wall);")
(define-mailbox-owner-reading mailbox-owner-after-gc-cpu "___result = GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.witnessed.after.gc_cpu);")
(def (mailbox-owner-watch-report)
  (unless (zero? (mailbox-owner-active)) (error "native observer report before join"))
  (mailbox-report
    "gap_begin_wall_ns" (mailbox-owner-gap-begin) "gap_end_lower_bound_wall_ns" (mailbox-owner-gap-end)
    "first_sample_wall_ns" (mailbox-owner-first-sample) "last_sample_wall_ns" (mailbox-owner-last-sample)
    "samples" (mailbox-owner-samples) "runnable_samples" (mailbox-owner-runnable)
    "waiting_samples" (mailbox-owner-waiting) "uninterruptible_samples" (mailbox-owner-uninterruptible)
    "stopped_samples" (mailbox-owner-stopped) "other_samples" (mailbox-owner-other)
    "select_samples" (mailbox-owner-select-samples) "select_waiting_samples" (mailbox-owner-select-waiting)
    "first_thread_cpu_ns" (mailbox-owner-first-cpu) "last_thread_cpu_ns" (mailbox-owner-last-cpu)
    "maximum_sample_spacing_ns" (mailbox-owner-sample-spacing) "query_errors" (mailbox-owner-query-errors)
    "owner_qos_class" (mailbox-owner-qos) "owner_assigned_qos_class" (mailbox-owner-assigned-qos)
    "owner_qos_query_status" (mailbox-owner-qos-status) "maximum_poll_spacing_ns" (mailbox-owner-poll-spacing)
    "completion_gap_begin_wall_ns" (mailbox-owner-witness-begin)
    "completion_gap_end_wall_ns" (mailbox-owner-witness-end)
    "same_interval" (and (positive? (mailbox-owner-gap-begin))
                         (= (mailbox-owner-gap-begin) (mailbox-owner-witness-begin)))
    "self_before" (mailbox-report "begin_wall_ns" (mailbox-owner-before-begin)
      "end_wall_ns" (mailbox-owner-before-end) "thread_cpu_ns" (mailbox-owner-before-cpu)
      "gc_count" (mailbox-owner-before-gcs) "gc_wall_ns" (mailbox-owner-before-gc-wall)
      "gc_cpu_ns" (mailbox-owner-before-gc-cpu))
    "self_after" (mailbox-report "begin_wall_ns" (mailbox-owner-after-begin)
      "end_wall_ns" (mailbox-owner-after-end) "thread_cpu_ns" (mailbox-owner-after-cpu)
      "gc_count" (mailbox-owner-after-gcs) "gc_wall_ns" (mailbox-owner-after-gc-wall)
      "gc_cpu_ns" (mailbox-owner-after-gc-cpu))))

(defrules with-mailbox-owner-failure ()
  ((_ #f body ...) (begin body ...))
  ((_ #t body ...)
   (with-catch (lambda (failure)
     (vector #f (mailbox-report "terminal_error"
       (with-output-to-string (lambda () (display-exception failure)))
       "owner" (mailbox-owner-watch-report))))
     (lambda () body ...))))
