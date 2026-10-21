;; Same real order computation, with independent per-lane Actor rank state.
(def orders-latch-lock (make-mutex))
(def orders-latch-event (make-condition-variable))
(def orders-latch-open #f)
(def orders-latch-blocked 0)
(def (orders-latch-wait!)
  (mutex-lock! orders-latch-lock)
  (set! orders-latch-blocked (1+ orders-latch-blocked))
  (let loop ()
    (unless orders-latch-open
      (mutex-unlock! orders-latch-lock orders-latch-event)
      (mutex-lock! orders-latch-lock)
      (loop)))
  (set! orders-latch-blocked (1- orders-latch-blocked))
  (mutex-unlock! orders-latch-lock))
(def (orders-latch-release!)
  (mutex-lock! orders-latch-lock)
  (set! orders-latch-open #t)
  (condition-variable-broadcast! orders-latch-event)
  (mutex-unlock! orders-latch-lock))
(def orders-lanes #())
(def (make-orders-lane)
  (let* ((rank (vector 0)) (lock (make-mutex))
         (next (lambda ()
                 (dynamic-wind (lambda () (mutex-lock! lock))
                   (lambda () (vector-set! rank 0 (1+ (vector-ref rank 0))) (vector-ref rank 0))
                   (lambda () (mutex-unlock! lock)))))
         (actors (make-gerbil-rs-actor-batch (lambda (worker input)
                    (when (= (s64vector-ref input 1) 2) (orders-latch-wait!))
                    (orders-handler/rank next worker input)) 4)))
    (vector actors rank)))
(def (orders-service-handler lane input)
  (if (s64vector? input)
    (if (= (s64vector-ref input 0) 11) (raise #f) (instance-native-call input))
  ;; Explicit application tag/version [77,1] selects one native order/result layout. It is
  ;; not inferred from input length and does not enter the Actor batch barrier.
  (if (and (vector? input) (= (vector-length input) 2)
           (equal? (vector-ref input 0) (s64vector 77 1)))
    (let (order (vector-ref input 1))
      (unless (and (s64vector? order) (<= (s64vector-length order) 14))
        (error "short order exceeds four-line contract"))
      (orders-handler/rank (lambda () 1) 0 order))
    (let (state (vector-ref orders-lanes lane))
      (vector-set! (vector-ref state 1) 0 0)
      (gerbil-rs-actor-batch! (vector-ref state 0) input)))))
(def (orders-lanes-open!)
  (set! orders-lanes (make-vector 4 #f))
  (let loop ((lane 0))
    (when (< lane 4) (vector-set! orders-lanes lane (make-orders-lane)) (loop (1+ lane)))))
(def (orders-lanes-close!)
  (vector-for-each (lambda (lane) (when lane (gerbil-rs-actor-batch-close! (vector-ref lane 0)))) orders-lanes)
  (set! orders-lanes #()))
(def orders-endpoint
  (make-gerbil-rs-service-endpoint orders-service-handler 4 orders-lanes-open!
    orders-latch-release! orders-lanes-close!))
(def (orders-service-command command) (orders-endpoint command))
(def (orders-service-reference command)
  (unless (= (s64vector-ref (vector-ref command 0) 0) 6) (error "invalid synchronous reference"))
  (orders-service-handler 0 (vector-ref command 1)))
;; One persistent worker exposes deterministic queue-order acceptance separately
;; from the four-VM throughput workload. It uses the identical service protocol.
(def single-started [])
(def single-accepted 0)
(def single-ready-max 0)
(def single-endpoint
  (make-gerbil-rs-service-endpoint
    (lambda (_ input)
      (set! single-started (cons (s64vector-ref input 0) single-started))
      (when (= (s64vector-ref input 0) 90) (orders-latch-wait!))
      (when (= (s64vector-ref input 0) 91) (raise #f))
      input)
    1 (lambda () (set! orders-latch-open #f)) orders-latch-release! void))
(def (single-command command)
  (let ((result (single-endpoint command)) (op (s64vector-ref (vector-ref command 0) 0)))
    (when (= op 1) (set! single-accepted (1+ single-accepted)))
    (when (= op 3) (set! single-ready-max (max single-ready-max (s64vector-length result))))
    result))
(def (orders-service-observe header)
  (case (s64vector-ref header 0)
    ((8) (orders-latch-release!) (s64vector 0))
    ((9) (set! orders-latch-open #f) (s64vector 0))
    ((12) (list->s64vector (append (list single-accepted single-ready-max) (reverse single-started))))
    ((10) (thread-yield!) (##gc) (s64vector orders-latch-blocked))
    (else (error "invalid service observation"))))
(begin-foreign
 (c-define (owner-service-call root) (int64) int64 "gerbil_owner_service_call" "extern"
  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
    gerbil-scheme-rust/qualification/actors#orders-service-command)))

(begin-foreign
 (c-define (owner-service-fail root) (int64) int64 "gerbil_owner_service_fail" "extern"
  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
    (lambda (command)
      (let ((result (gerbil-scheme-rust/qualification/actors#orders-service-command command)))
        (when (= (s64vector-ref (vector-ref command 0) 0) 0)
          (error "injected Scheme service initialization failure"))
        result)))))

(begin-foreign
 (c-define (owner-service-reference root) (int64) int64 "gerbil_owner_service_reference" "extern"
  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
    gerbil-scheme-rust/qualification/actors#orders-service-reference))
 (c-define (owner-service-observe root) (int64) int64 "gerbil_owner_service_observe" "extern"
  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
    gerbil-scheme-rust/qualification/actors#orders-service-observe)))

(begin-foreign
 (c-define (owner-service-single root) (int64) int64 "gerbil_owner_service_single" "extern"
  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
    gerbil-scheme-rust/qualification/actors#single-command)))
