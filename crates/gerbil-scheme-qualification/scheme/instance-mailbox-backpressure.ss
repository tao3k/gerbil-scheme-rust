;; Qualification-only native exchange. The source retains ordinal debt, never
;; an unbounded list of payloads. The original timestamps and deadline include
;; that debt. Three existing execution credits own three transfer frames.
(defrules define-mailbox-backpressure ()
 ((_ name observe? watch? park? coalesce?)
 (def (name command)
  (let* ((header (vector-ref command 0)) (rate (s64vector-ref header 1))
         (seconds (s64vector-ref header 2)) (progress (s64vector-ref header 3))
         (state (orders-endpoint (vector (s64vector 15)))))
    (unless (and (= (s64vector-length header) (if (or observe? watch?) 5 4)) (<= 1 rate 500000) (<= 1 seconds 10)
                 (positive? progress) (zero? (s64vector-ref state 0))
                 (zero? (s64vector-ref state 3)) (zero? (s64vector-ref state 7))
                 (not orders-native-access?))
      (error "native backpressure requires idle original handler, at most 500k/s"))
    (when (memv (s64vector-ref header 0) '(56 57 58 59 60)) (actor-arrival-access-contract!))
    (when observe? (mailbox-clock-contract!))
    (let* ((receiver (current-thread)) (workers (make-vector 4 #f))
           ;; [slot, application id, custody, input/result, failure?]
           (frames (vector (vector 0 -1 'free #f #f) (vector 1 -1 'free #f #f)
                           (vector 2 -1 'free #f #f)))
           (histogram (make-u64vector 5000000 0)) (total (* rate seconds))
           (next 0) (completed 0) (active 0) (max-debt 0) (max-outstanding 0)
           (max-lag 0) (last-publish 0) (over-window 0) (start 0) (stopped? #f)
           (watch-before #f) (watch-after #f)
           (edge (and coalesce? (make-mailbox-ready-edge frames receiver (vector #f) 0 0 0))))
     (with-mailbox-observation observe? rate (if observe? (s64vector-ref header 4) 0)
       observer (source construction dispatch consumption idle)
      (def (planned id) (+ start (quotient (* id 1000000000) rate)))
      (def (stop!)
       (unless stopped?
        (vector-for-each (lambda (worker) (when (thread? worker) (thread-send worker 'stop))) workers)
        (vector-for-each (lambda (worker) (when (thread? worker) (thread-join! worker 5))) workers)
        ;; All worker references are gone before clearing unread roots. This is
        ;; the private native caller's mailbox, not a shared application inbox.
        (let drain () (when (thread-receive 0 #f) (drain)))
        (vector-for-each (lambda (frame)
          (vector-set! frame 1 -1) (vector-set! frame 2 'free)
          (vector-set! frame 3 #f) (vector-set! frame 4 #f)) frames)
        (set! stopped? #t)))
      (with-mailbox-observation-failure observe? observer
       (with-mailbox-owner-failure watch?
       (dynamic-wind (lambda () (set! orders-native-access? (memv (s64vector-ref header 0) '(56 57 58 59 60))))
        (lambda ()
          (let create ((index 0))
            (when (< index 4)
             (with-mailbox-observation observe? rate (if observe? (s64vector-ref header 4) 0)
               worker-observer (business verification handoff worker-idle)
              (when observe?
                (set! (mailbox-observation-children observer)
                  (cons (cons (number->string index) worker-observer)
                    (mailbox-observation-children observer))))
              (vector-set! workers index
                (spawn (lambda ()
                  (when observe? (mailbox-observation-start! worker-observer worker-idle))
                  (let work ()
                    (let (frame (thread-receive))
                      (if (eq? frame 'stop)
                       (when observe? (mailbox-observe! worker-observer worker-idle 0))
                       (begin
                        (let (id (vector-ref frame 1))
                          (unless (eq? (vector-ref frame 2) 'queued) (error "native frame not queued"))
                          (vector-set! frame 2 'running)
                          (when observe? (mailbox-observe! worker-observer business id))
                          (with-catch
                            (lambda (failure) (vector-set! frame 4 #t) (vector-set! frame 3 failure))
                            (lambda ()
                              (let (value (orders-service-handler index (vector-ref frame 3)))
                                (when observe? (mailbox-observe! worker-observer verification id))
                                ;; Complete the SAME independent oracle inside
                                ;; this existing Actor turn, not a new scheduler.
                                (actor-arrival-verify! id value)
                                (vector-set! frame 3 value))))
                          (when observe? (mailbox-observe! worker-observer handoff id))
                          (vector-set! frame 2 'ready)
                          ;; Immutable id protects frame reuse against a stale
                          ;; response even after its mutable slot is recycled.
                          (if coalesce?
                            (mailbox-publish-ready! edge id)
                            (thread-send receiver (cons id frame))))
                        (when observe? (mailbox-observe! worker-observer worker-idle 0))
                        (work))))))))
              )
              (create (1+ index))))
          (when watch?
            (set! watch-before (mailbox-read-clock))
            (unless (zero? (mailbox-owner-watch-start! progress
                              (quotient (* (s64vector-ref header 4) 1000000000) rate)))
              (error "native owner observation unavailable")))
          (set! start (owner-wait-trace 1 0))
          (when observe? (mailbox-observation-start! observer source))
          (let drive ()
            (when observe? (mailbox-observe! observer source next))
            (let* ((now (owner-wait-trace 1 0))
                   (due (min total (1+ (quotient (* (- now start) rate) 1000000000))))
                   (debt (- due next)) (outstanding (- due completed)))
              (set! max-debt (max max-debt debt))
              (set! max-outstanding (max max-outstanding outstanding))
              (when (> outstanding 128) (set! over-window (1+ over-window)))
              (when (and (positive? debt) (>= (- now (planned next)) 5000000000))
                (error "native source debt exceeded five seconds"))
              ;; Source eligibility is arithmetic, retaining every original
              ;; arrival, not a loop allocating or rejecting catch-up messages.
              (let supply ((slot 0))
                (when (and (< slot 3) (< next due))
                  (let (frame (vector-ref frames slot))
                    (when (eq? (vector-ref frame 2) 'free)
                      (when observe? (mailbox-observe! observer construction next))
                      (let ((id next) (input (actor-arrival-input next)))
                        (vector-set! frame 1 id) (vector-set! frame 2 'queued)
                        (vector-set! frame 3 input) (vector-set! frame 4 #f)
                        (thread-send (vector-ref workers (modulo id 4)) frame)
                        (set! next (1+ next)) (set! active (1+ active))
                        (set! last-publish (- (owner-wait-trace 1 0) start))
                        (set! max-lag (max max-lag (- (+ start last-publish) (planned id))))
                        (when observe? (mailbox-observe! observer source next)))))
                  (supply (1+ slot)))))
            (if (positive? active)
              ;; Blocking mailbox receive gives the green scheduler the actual
              ;; dependency; no yield/poll layer or timer is added.
              (let* ((message (begin
                       (when observe? (mailbox-observe! observer dispatch next))
                       (if coalesce? (mailbox-take-ready edge) (thread-receive 5 #f)))))
                (unless (or coalesce? message) (error "native completion exceeded five seconds"))
                ;; A wake may describe results already taken by the fast path.
                ;; It returns no credit, fabricates no progress, and never waits
                ;; for a whole batch. Recheck the service cycle immediately.
                (when message
                (when observe? (mailbox-observe! observer consumption (car message)))
                (let* ((frame (if coalesce? message (cdr message)))
                       (id (if coalesce? (vector-ref frame 1) (car message)))
                       (slot (vector-ref frame 0)))
                  (unless (and (<= 0 slot 2) (eq? frame (vector-ref frames slot))
                               (= id (vector-ref frame 1)) (eq? (vector-ref frame 2) 'ready))
                    (error "stale or duplicate native completion frame"))
                  (when (vector-ref frame 4) (raise (vector-ref frame 3)))
                  (let* ((now (owner-wait-trace 1 0)) (latency (- now (planned id)))
                         (bucket (quotient latency 1000)))
                    (unless (< latency 5000000000) (error "native request exceeded five seconds"))
                    (u64vector-set! histogram bucket (1+ (u64vector-ref histogram bucket))))
                  (unless (zero? (owner-wait-trace 11 progress)) (error "native verified progress deadline"))
                  ;; Only verified consumption returns custody and capacity.
                  (vector-set! frame 3 #f) (vector-set! frame 4 #f)
                  (vector-set! frame 1 -1) (vector-set! frame 2 'free)
                  (set! active (1- active)) (set! completed (1+ completed)))))
              (begin
                (when observe? (mailbox-observe! observer idle next))
                (if park?
                  ;; No admitted work can be delayed: all active credits are
                  ;; free. Park this green thread only until the ORIGINAL next
                  ;; source timestamp; overdue work is supplied immediately.
                  (when (< next total)
                    (let (remaining (- (planned next) (owner-wait-trace 1 0)))
                      (when (positive? remaining)
                        (thread-sleep! (exact->inexact (/ remaining 1000000000))))))
                  (thread-yield!))))
            (when (< completed total) (drive)))
          (let (elapsed (- (owner-wait-trace 1 0) start))
            (when watch?
              (unless (zero? (mailbox-owner-watch-stop!)) (error "native owner observer cleanup failed"))
              (set! watch-after (mailbox-read-clock)))
            (when observe? (mailbox-observe! observer source next))
            (unless (and (= next total) (= completed total) (zero? active))
              (error "native backpressure accounting"))
            (when coalesce?
              ;; Join any worker between frame publication and signaling before
              ;; projecting epoch counters. Cleanup drains the last optional wake.
              (stop!))
            (let (receipt (s64vector total completed 0 elapsed (quotient (* (1- total) 1000000000) rate)
              (actor-arrival-percentile histogram completed 99 100)
              (actor-arrival-percentile histogram completed 999 1000) max-lag
              max-debt max-outstanding last-publish over-window))
              (if observe?
                (begin
                  (stop!)
                  (unless (and (= (mailbox-observation-count observer "business") completed)
                               (= (mailbox-observation-count observer "verification") completed)
                               (= (mailbox-observation-count observer "construction") completed)
                               (= (mailbox-observation-count observer "consumption") completed))
                    (error "green-thread observation count mismatch"))
                  (vector receipt (mailbox-observation-report observer)))
                (if watch?
                  (vector receipt (mailbox-report "owner" (mailbox-owner-watch-report)
                    "trial_begin" (mailbox-clock-report watch-before)
                    "trial_end" (mailbox-clock-report watch-after)))
                  (if coalesce?
                    (vector receipt (mailbox-ready-receipt edge))
                    receipt))))))
        (lambda ()
          (with-catch (lambda (failure) (set! orders-native-access? #f) (raise failure))
            (lambda ()
              (when watch?
                (unless (zero? (mailbox-owner-watch-stop!)) (error "native owner observer unwind failed")))
              (stop!) (set! orders-native-access? #f)))))))))))))
(define-mailbox-backpressure orders-mailbox-backpressure #f #f #f #f)
(define-mailbox-backpressure orders-mailbox-backpressure-observed #t #f #f #f)
(define-mailbox-backpressure orders-mailbox-backpressure-watched #f #t #f #f)
(define-mailbox-backpressure orders-mailbox-backpressure-parked #f #f #t #f)
(define-mailbox-backpressure orders-mailbox-backpressure-coalesced #f #f #f #t)
