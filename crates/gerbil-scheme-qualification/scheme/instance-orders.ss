;; Application workload: validate and aggregate native order-line records by SKU.
;; Rust supplies [request-id, wait?, sku, quantity, unit-cents, ...].
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
(def (orders-handler worker input)
  (unless (and (s64vector? input) (>= (s64vector-length input) 5)
               (zero? (modulo (- (s64vector-length input) 2) 3)))
    (error "invalid order records"))
  (when (= (s64vector-ref input 1) 1) (thread-sleep! 0.001))
  (let ((totals (make-vector 16 0)) (units 0) (gross 0))
    (let loop ((i 2))
      (when (< i (s64vector-length input))
        (let ((sku (s64vector-ref input i)) (quantity (s64vector-ref input (1+ i)))
              (cents (s64vector-ref input (+ i 2))))
          (unless (and (<= 0 sku 15) (positive? quantity) (<= 0 cents))
            (error "invalid order line"))
          (vector-set! totals sku (+ (vector-ref totals sku) (* quantity cents)))
          (set! units (+ units quantity))
          (set! gross (+ gross (* quantity cents))))
        (loop (+ i 3))))
    ;; rank/worker are application instrumentation, never a Rust scheduler.
    (let ((rank (orders-next-rank!)) (result (make-s64vector 21 0)))
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
      (instance-native-call value))
    (instance-orders-call value)))
(begin-foreign
 (c-define (owner-orders-call root) (int64) int64 "gerbil_owner_orders_call" "extern"
  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
    gerbil-scheme-rust/qualification/actors#instance-orders-entry)))
