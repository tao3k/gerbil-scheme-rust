;; Native reads live next to the Scheme owner. C knows no stages or report
;; layout. Readings go into private green-thread records at a source-window
;; boundary. C retains no shared snapshot, stage metadata or Scheme pointer.
(extern mailbox-native-wall mailbox-native-cpu mailbox-native-gc-count
        mailbox-native-gc-wall mailbox-native-gc-cpu)
(begin-foreign
 (namespace ("gerbil-scheme-rust/qualification/actors#"
   mailbox-native-wall mailbox-native-cpu mailbox-native-gc-count
   mailbox-native-gc-wall mailbox-native-gc-cpu))
 (c-declare #<<c-end
#include <stdint.h>
#include <time.h>
static uint64_t mailbox_clock_now(void) {
  struct timespec now;
  if (clock_gettime(CLOCK_UPTIME_RAW, &now)) return UINT64_MAX;
  return (uint64_t)now.tv_sec * 1000000000ULL + (uint64_t)now.tv_nsec;
}
static uint64_t mailbox_cpu_now(void) {
  struct timespec now;
  if (clock_gettime(CLOCK_THREAD_CPUTIME_ID, &now)) return UINT64_MAX;
  return (uint64_t)now.tv_sec * 1000000000ULL + (uint64_t)now.tv_nsec;
}
c-end
 )
 (define mailbox-native-wall (c-lambda () unsigned-int64 "mailbox_clock_now"))
 (define mailbox-native-cpu (c-lambda () unsigned-int64 "mailbox_cpu_now"))
 (define mailbox-native-gc-count (c-lambda () unsigned-int64 "___result = (uint64_t)___VMSTATE->mem.nb_gcs_;"))
 (define mailbox-native-gc-wall (c-lambda () unsigned-int64 "___result = (uint64_t)(___VMSTATE->mem.gc_real_time_ * 1000000000.0);"))
 (define mailbox-native-gc-cpu (c-lambda () unsigned-int64 "___result = (uint64_t)((___VMSTATE->mem.gc_user_time_ + ___VMSTATE->mem.gc_sys_time_) * 1000000000.0);")))

(defstruct mailbox-clock (begin end cpu gc-count gc-wall gc-cpu))
(defstruct mailbox-interval (id begin end window-begin window-end))
(defstruct mailbox-stage (name count total maximum))
(defstruct mailbox-observation (current id begin sample period stages children))

(def (mailbox-read-clock)
  ;; No TLS snapshot slot: another green thread may run between primitive calls.
  ;; Each reading remains local; explicit brackets expose that acquisition span.
  (let* ((begin (mailbox-native-wall)) (cpu (mailbox-native-cpu))
         (count (mailbox-native-gc-count)) (wall (mailbox-native-gc-wall))
         (gc-cpu (mailbox-native-gc-cpu)) (end (mailbox-native-wall)))
    (when (or (= begin #xffffffffffffffff) (= cpu #xffffffffffffffff)
              (= end #xffffffffffffffff)) (error "native observation clock unavailable"))
    (make-mailbox-clock begin end cpu count wall gc-cpu)))

;; The catalog appears once, as lexical stages in the caller. No numeric phase
;; identifiers, fixed stage count, serialized record width or C column indexes.
(defrules with-mailbox-observation ()
  ((_ enabled? rate window observer (stage ...) body ...)
   (let* ((stage (and enabled? (make-mailbox-stage (symbol->string 'stage) 0 0
                               (make-mailbox-interval 0 0 0 #f #f)))) ...
          (observer (and enabled? (make-mailbox-observation #f 0 0 #f
                       (quotient (* window 1000000000) rate) (list stage ...) []))))
     body ...)))

(defrules with-mailbox-observation-failure ()
  ((_ #f observer body ...) (begin body ...))
  ((_ #t observer body ...)
   (with-catch
     (lambda (failure)
       ;; The protocol has already unwound/joined before diagnostics are
       ;; projected. Preserve failure evidence; never forge a success receipt.
       (vector #f (mailbox-report "terminal_error"
         (with-output-to-string (lambda () (display-exception failure)))
         "stages" (mailbox-observation-report observer))))
     (lambda () body ...))))

(def (mailbox-observation-start! observer first)
  (let (sample (mailbox-read-clock))
    (set! (mailbox-observation-sample observer) sample)
    (set! (mailbox-observation-current observer) first)
    (set! (mailbox-observation-begin observer) (owner-wait-trace 1 0))))

(def (mailbox-observe! observer following id)
  (let* ((now (owner-wait-trace 1 0)) (before (mailbox-observation-sample observer))
         ;; Sampling cadence is the existing source window's service duration,
         ;; not another configurable scheduler/latency threshold.
         (after (if (>= (- now (mailbox-clock-end before)) (mailbox-observation-period observer))
                  (mailbox-read-clock) before))
         (stage (mailbox-observation-current observer))
         (begin (mailbox-observation-begin observer)) (duration (- now begin)))
    (set! (mailbox-stage-count stage) (1+ (mailbox-stage-count stage)))
    (set! (mailbox-stage-total stage) (+ (mailbox-stage-total stage) duration))
    (let (maximum (mailbox-stage-maximum stage))
      (when (> duration (- (mailbox-interval-end maximum) (mailbox-interval-begin maximum)))
        (set! (mailbox-interval-id maximum) (mailbox-observation-id observer))
        (set! (mailbox-interval-begin maximum) begin) (set! (mailbox-interval-end maximum) now)
        (set! (mailbox-interval-window-begin maximum) before)
        (set! (mailbox-interval-window-end maximum) after)))
    (set! (mailbox-observation-sample observer) after)
    (set! (mailbox-observation-current observer) following)
    (set! (mailbox-observation-id observer) id)
    ;; A new CPU sample must finish before the following business interval
    ;; starts, so the two raw snapshots bracket a long interval. Sampling
    ;; overhead itself is excluded from phase intervals, never from request p99.
    (set! (mailbox-observation-begin observer) (max now (mailbox-clock-end after)))))

;; Projection is after the experiment clock. Actual native tables, strings and
;; scalars use the existing object ABI; no JSON enters or crosses the VM here.
(def (mailbox-report . fields)
  (let (result (make-table test: equal?))
    (let loop ((rest fields))
      (unless (null? rest)
        (table-set! result (car rest) (cadr rest)) (loop (cddr rest)))) result))
(def (mailbox-clock-report sample)
  (mailbox-report "begin_wall_ns" (mailbox-clock-begin sample)
    "end_wall_ns" (mailbox-clock-end sample) "thread_cpu_ns" (mailbox-clock-cpu sample)
    "gc_count" (mailbox-clock-gc-count sample) "gc_wall_ns" (mailbox-clock-gc-wall sample)
    "gc_cpu_ns" (mailbox-clock-gc-cpu sample)))
(def (mailbox-stage-report observer)
  (let (result (make-table test: equal?))
    (for-each (lambda (stage)
      (let* ((span (and (positive? (mailbox-stage-count stage)) (mailbox-stage-maximum stage)))
             (maximum (and span (mailbox-report "application_id" (mailbox-interval-id span)
               "begin_wall_ns" (mailbox-interval-begin span) "end_wall_ns" (mailbox-interval-end span)
               "window_begin" (mailbox-clock-report (mailbox-interval-window-begin span))
               "window_end" (mailbox-clock-report (mailbox-interval-window-end span))))))
        (table-set! result (mailbox-stage-name stage)
          (mailbox-report "interval_count" (mailbox-stage-count stage)
            "total_wall_ns" (mailbox-stage-total stage) "maximum" maximum))))
      (mailbox-observation-stages observer)) result))

(def (mailbox-clock-contract!)
  (let (before (mailbox-read-clock))
    (##gc)
    (let (after (mailbox-read-clock))
      (unless (and (<= (mailbox-clock-begin before) (mailbox-clock-end before)
                      (mailbox-clock-begin after) (mailbox-clock-end after))
                   (positive? (mailbox-clock-cpu before))
                   (<= (mailbox-clock-cpu before) (mailbox-clock-cpu after))
                   (< (mailbox-clock-gc-count before) (mailbox-clock-gc-count after))
                   (<= (mailbox-clock-gc-wall before) (mailbox-clock-gc-wall after))
                   (<= (mailbox-clock-gc-cpu before) (mailbox-clock-gc-cpu after)))
        (error "native clock/GC observation contract")))))

(def (mailbox-observation-report observer)
  (let (children (make-table test: equal?))
    (for-each (lambda (entry)
      (table-set! children (car entry) (mailbox-observation-report (cdr entry))))
      (mailbox-observation-children observer))
    (mailbox-report "stages" (mailbox-stage-report observer) "children" children)))

(def (mailbox-observation-count observer name)
  (let ((own 0) (children 0))
    (for-each (lambda (stage)
      (when (string=? name (mailbox-stage-name stage)) (set! own (+ own (mailbox-stage-count stage)))))
      (mailbox-observation-stages observer))
    (for-each (lambda (entry)
      (set! children (+ children (mailbox-observation-count (cdr entry) name))))
      (mailbox-observation-children observer))
    (+ own children)))
