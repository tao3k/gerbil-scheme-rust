;; Application workload: validate and aggregate native order-line records by SKU.
;; Rust supplies [request-id, wait?, sku, quantity, unit-cents, ...].
(extern owner-wait-trace)
(begin-foreign
 (namespace ("gerbil-scheme-rust/qualification/actors#" owner-wait-trace))
 (c-declare #<<c-end
#include <stdint.h>
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
      (orders-unit/rank next-rank worker input span checkpoint))))
(def (orders-unit/rank next-rank worker input span checkpoint)
  (when (= (s64vector-ref input 1) 2)
    (call-with-gerbil-rs-service-suspension orders-latch-wait!))
  (when (= (s64vector-ref input 1) 1)
    (if orders-wait-trace?
      (let (start (owner-wait-trace 1 0))
        (call-with-gerbil-rs-service-suspension (lambda () (thread-sleep! 0.001)))
        (owner-wait-trace 2 start))
      (call-with-gerbil-rs-service-suspension (lambda () (thread-sleep! 0.001)))))
  (let ((totals (make-vector 16 0)) (units 0) (gross 0))
    (let (size (s64vector-length input))
      (let spans ((start 2))
        (when (< start size)
          ;; Scheme selects one declared span before arithmetic: the original
          ;; whole loop without competition, bounded slices with competition.
          (let (end (min size (+ start (* span 3))))
            (let records ((i start))
              (when (< i end)
                (let ((sku (s64vector-ref input i)) (quantity (s64vector-ref input (1+ i)))
                      (cents (s64vector-ref input (+ i 2))))
                  (unless (and (<= 0 sku 15) (positive? quantity) (<= 0 cents))
                    (error "invalid order line"))
                  (vector-set! totals sku (+ (vector-ref totals sku) (* quantity cents)))
                  (set! units (+ units quantity))
                  (set! gross (+ gross (* quantity cents))))
                (records (+ i 3))))
            (when checkpoint (checkpoint (quotient (- end start) 3) (< end size)))
            (spans end)))))
    ;; rank/worker are application instrumentation, never a Rust scheduler.
    (let ((rank (next-rank)) (result (make-s64vector 21 0)))
      (s64vector-set! result 0 (s64vector-ref input 0))
      (s64vector-set! result 1 units)
      (s64vector-set! result 2 gross)
      (s64vector-set! result 3 rank)
      (s64vector-set! result 4 worker)
      (let loop ((sku 0))
        (when (< sku 16)
          (s64vector-set! result (+ sku 5) (vector-ref totals sku))
          (loop (1+ sku))))
      result)))
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
