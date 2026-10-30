;; Application workload: validate and aggregate native order-line records by SKU.
;; Rust supplies [request-id, wait?, sku, quantity, unit-cents, ...].
(extern owner-wait-trace)
(begin-foreign
 (namespace ("gerbil-scheme-rust/qualification/actors#" owner-wait-trace))
 (c-declare #<<c-end
#include <stdint.h>
#include <string.h>
#include <math.h>
static uint64_t (*owner_scope_control)(const char *, uint64_t, uint64_t);
static uint64_t (*owner_scope_publish)(uint64_t, uint64_t);
static void (*owner_scope_source)(uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t);
void gerbil_owner_scope_bind(uint64_t (*control)(const char *, uint64_t, uint64_t),
    uint64_t (*publish)(uint64_t, uint64_t),
    void (*source)(uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t)) {
  owner_scope_control=control; owner_scope_publish=publish; owner_scope_source=source;
}
static uint64_t gerbil_owner_scope_control(const char *name, uint64_t generation, uint64_t value) {
  return owner_scope_control ? owner_scope_control(name,generation,value) :
    !strcmp(name,"supported") ? 0 : UINT64_MAX;
}
static uint64_t gerbil_owner_scope_publish(uint64_t generation, uint64_t address) {
  return owner_scope_publish ? owner_scope_publish(generation,address) : UINT64_MAX;
}
static void gerbil_owner_scope_source(uint64_t generation, uint64_t now, uint64_t due,
    uint64_t published, uint64_t consumed, uint64_t active) {
  if (owner_scope_source) owner_scope_source(generation,now,due,published,consumed,active);
}
uint64_t gerbil_owner_scope_gc_count(void) { return (uint64_t)___VMSTATE->mem.nb_gcs_; }
static uint64_t gerbil_owner_scope_gc_ns(double seconds) {
  double value=seconds*1000000000.0;
  return isfinite(value) && value>=0 && value<(double)UINT64_MAX ? (uint64_t)value : UINT64_MAX;
}
uint64_t gerbil_owner_scope_gc_wall(void) { return gerbil_owner_scope_gc_ns(___VMSTATE->mem.gc_real_time_); }
uint64_t gerbil_owner_scope_gc_cpu(void) {
  return gerbil_owner_scope_gc_ns(___VMSTATE->mem.gc_user_time_+___VMSTATE->mem.gc_sys_time_);
}
static uint64_t (*owner_wait_observe)(int, uint64_t);
void gerbil_owner_wait_bind(uint64_t (*observe)(int, uint64_t)) {
  owner_wait_observe = observe;
}
static uint64_t gerbil_owner_wait_trace(int operation, uint64_t value) {
  return owner_wait_observe ? owner_wait_observe(operation, value) : 0;
}
c-end
 )
 (define owner-wait-trace
   (c-lambda (int unsigned-int64) unsigned-int64 "gerbil_owner_wait_trace")))
(extern owner-scope-control owner-scope-publish owner-scope-source)
(begin-foreign
 (namespace ("gerbil-scheme-rust/qualification/actors#" owner-scope-control owner-scope-publish owner-scope-source))
 (define owner-scope-control (c-lambda (char-string unsigned-int64 unsigned-int64) unsigned-int64 "gerbil_owner_scope_control"))
 (define owner-scope-publish (c-lambda (unsigned-int64 unsigned-int64) unsigned-int64 "gerbil_owner_scope_publish"))
 (define owner-scope-source (c-lambda (unsigned-int64 unsigned-int64 unsigned-int64 unsigned-int64 unsigned-int64 unsigned-int64) void "gerbil_owner_scope_source")))
(def orders-wait-trace? #f)
(def (orders-wait-snapshot)
  (let* ((count (owner-wait-trace 3 0)) (result (make-s64vector count)))
    (let loop ((index 0))
      (when (< index count)
        (s64vector-set! result index (owner-wait-trace 4 index))
        (loop (1+ index))))
    result))
(def orders-service #f)
(def orders-control-service #f)
(def orders-control? #f)
(def orders-rank 0)
(def orders-rank-lock (make-mutex))
(def (orders-next-rank!)
  (dynamic-wind
    (lambda () (mutex-lock! orders-rank-lock))
    (lambda () (set! orders-rank (1+ orders-rank)) orders-rank)
    (lambda () (mutex-unlock! orders-rank-lock))))
(def (orders-handler worker input) (orders-handler/rank orders-next-rank! worker input))
(def orders-short-layout (make-gerbil-rs-service-work-layout 4))
(def orders-bulk-layout (make-gerbil-rs-service-work-layout 48))
(def (orders-handler/rank next-rank worker input)
  (unless (and (s64vector? input) (>= (s64vector-length input) 5)
               (zero? (modulo (- (s64vector-length input) 2) 3)))
    (call-with-gerbil-rs-service-work-layout orders-short-layout
      (lambda (_ __) (error "invalid order records"))))
  (let* ((cost (quotient (- (s64vector-length input) 2) 3))
         (layout (case cost ((4) orders-short-layout) ((48) orders-bulk-layout)
                   (else (make-gerbil-rs-service-work-layout cost)))))
    (with-gerbil-rs-service-work-layout (span checkpoint) layout
      (if orders-native-access?
        (orders-unit/native next-rank worker input span checkpoint)
        (orders-unit/rank next-rank worker input span checkpoint)))))
;; All indexes below follow the existing shape/record guards. Keep numeric
;; arithmetic checked and generic; only proven vector access is specialized.
(def orders-native-access? #f)
(defrules orders-input-ref ()
  ((_ value index) (let () (declare (not safe)) (s64vector-ref value index))))
(defrules orders-input-size ()
  ((_ value) (let () (declare (not safe)) (s64vector-length value))))
(defrules orders-total-ref ()
  ((_ value index) (let () (declare (not safe)) (vector-ref value index))))
(defrules orders-total-set! ()
  ((_ value index item) (let () (declare (not safe)) (vector-set! value index item))))
(defrules orders-result-set! ()
  ((_ target index item)
   (let* ((output target) (position index) (value item))
     ;; A fixnum always fits signed64. Bignums and invalid numeric values retain
     ;; the original checked conversion and its range/type exception behavior.
     (if (fixnum? value)
       (let () (declare (not safe)) (s64vector-set! output position value))
       (s64vector-set! output position value)))))
(defrules define-orders-unit ()
  ((_ name read-input input-size read-total write-total! write-result!)
    (def (name next-rank worker input span checkpoint)
      (when (= (read-input input 1) 2)
        (call-with-gerbil-rs-service-suspension orders-latch-wait!))
      (when (= (read-input input 1) 1)
        (if orders-wait-trace?
          (let (start (owner-wait-trace 1 0))
            (call-with-gerbil-rs-service-suspension (lambda () (thread-sleep! 0.001)))
            (owner-wait-trace 2 start))
          (call-with-gerbil-rs-service-suspension (lambda () (thread-sleep! 0.001)))))
      (let ((totals (make-vector 16 0)) (units 0) (gross 0))
        (let (size (input-size input))
          (let spans ((start 2))
            (when (< start size)
              ;; Scheme selects one declared span before arithmetic: the original
              ;; whole loop without competition, bounded slices with competition.
              (let (end (min size (+ start (* span 3))))
                (let records ((i start))
                  (when (< i end)
                    (let ((sku (read-input input i)) (quantity (read-input input (1+ i)))
                          (cents (read-input input (+ i 2))))
                      (unless (and (<= 0 sku 15) (positive? quantity) (<= 0 cents))
                        (error "invalid order line"))
                      (write-total! totals sku (+ (read-total totals sku) (* quantity cents)))
                      (set! units (+ units quantity))
                      (set! gross (+ gross (* quantity cents))))
                    (records (+ i 3))))
                (when checkpoint (checkpoint (quotient (- end start) 3) (< end size)))
                (spans end)))))
        ;; rank/worker are application instrumentation, never a Rust scheduler.
        (let ((rank (next-rank)) (result (make-s64vector 21 0)))
          (write-result! result 0 (read-input input 0))
          (write-result! result 1 units)
          (write-result! result 2 gross)
          (write-result! result 3 rank)
          (write-result! result 4 worker)
          (let loop ((sku 0))
            (when (< sku 16)
              (write-result! result (+ sku 5) (read-total totals sku))
              (loop (1+ sku))))
          result)))))
(define-orders-unit orders-unit/rank s64vector-ref s64vector-length vector-ref vector-set! s64vector-set!)
(define-orders-unit orders-unit/native orders-input-ref orders-input-size orders-total-ref orders-total-set! orders-result-set!)
(def (instance-orders-call envelope)
  (set! orders-rank 0)
  (if orders-control?
    (begin
      (unless orders-control-service
        (set! orders-control-service (make-gerbil-rs-control-actor-batch orders-handler 4)))
      (gerbil-rs-control-actor-batch! orders-control-service envelope))
    (begin
      (unless orders-service (set! orders-service (make-gerbil-rs-actor-batch orders-handler 4)))
      (gerbil-rs-actor-batch! orders-service envelope))))
(def (instance-orders-close!)
  (when orders-service (gerbil-rs-actor-batch-close! orders-service))
  (set! orders-service #f)
  (when orders-control-service (gerbil-rs-control-actor-batch-close! orders-control-service))
  (set! orders-control-service #f)
  (s64vector 0))

;; Separate application export keeps generic native vector roundtrips unchanged.
(def (instance-orders-entry value)
  (if (s64vector? value)
    (if (and (= (s64vector-length value) 2) (= (s64vector-ref value 0) 9))
      (begin (set! orders-control? (zero? (s64vector-ref value 1))) (s64vector 0))
      (case (s64vector-ref value 0)
        ((10) (set! orders-wait-trace? (not (zero? (s64vector-ref value 1))))
              (owner-wait-trace 0 (s64vector-ref value 1)) (s64vector 0))
        ((11) (orders-wait-snapshot))
        ((14) (set! orders-wait-trace? #f) (s64vector (owner-wait-trace 10 0)))
        ((13) (s64vector (owner-wait-trace 9 0) (owner-wait-trace 9 1) (owner-wait-trace 9 2)))
        ((12) (s64vector (owner-wait-trace 5 (s64vector-ref value 1)) (owner-wait-trace 6 0) (owner-wait-trace 7 0)))
        (else (instance-native-call value))))
    (instance-orders-call value)))
(begin-foreign
 (c-define (owner-orders-call root) (int64) int64 "gerbil_owner_orders_call" "extern"
  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
    gerbil-scheme-rust/qualification/actors#instance-orders-entry)))

;; Snapshot qualification mutates only its explicitly supplied parent vector.
(def (snapshot-retarget value)
  (vector-set! value 0 #f)
  (##gc)
  (void))
(begin-foreign
 (c-define (owner-snapshot-retarget root) (int64) int64 "gerbil_owner_snapshot_retarget" "extern"
  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
    gerbil-scheme-rust/qualification/actors#snapshot-retarget)))
