;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Opt-in cooperative units. Scheme owns all leases, waits and class decisions.
(export gerbil-rs-native-state gerbil-rs-native-has-sources? gerbil-rs-native-runnable? gerbil-rs-native-source-add! gerbil-rs-native-source-remove! gerbil-rs-native-budget-short? gerbil-rs-native-source-short? gerbil-rs-native-snapshot
        gerbil-rs-runtime-state gerbil-rs-runtime-pass? gerbil-rs-runtime-snapshot make-gerbil-rs-runtime-context make-gerbil-rs-runtime-contexts call-with-gerbil-rs-runtime-context
        gerbil-rs-budget-limit make-gerbil-rs-budget gerbil-rs-budget-configure! gerbil-rs-budget-snapshot gerbil-rs-budget-idle?
        make-gerbil-rs-progress-ledger gerbil-rs-progress-receipt gerbil-rs-progress-for-batch
        gerbil-rs-progress-context gerbil-rs-progress-unit! make-gerbil-rs-progress gerbil-rs-progress-finish!
        gerbil-rs-service-context call-with-gerbil-rs-budget-context
        call-with-gerbil-rs-service-unit call-with-gerbil-rs-service-suspension call-with-gerbil-rs-service-turn gerbil-rs-service-turn-snapshot gerbil-rs-turn-snapshot
        gerbil-rs-work-configure! gerbil-rs-work-snapshot gerbil-rs-work-enabled?
        gerbil-rs-work-cycle-executed gerbil-rs-work-cycle-state gerbil-rs-work-cycle-snapshot gerbil-rs-work-execution-pending? gerbil-rs-work-execution-context make-gerbil-rs-work-execution gerbil-rs-work-execution-runnable! gerbil-rs-work-execution-finished! gerbil-rs-work-execution-snapshot
        gerbil-rs-work-submitted! gerbil-rs-work-dispatch! gerbil-rs-work-started! gerbil-rs-work-ready! gerbil-rs-work-harvested! gerbil-rs-work-drain! call-with-gerbil-rs-work-receiver
        gerbil-rs-service-work-checkpoint with-gerbil-rs-service-work-plan
        gerbil-rs-work-plan-snapshot gerbil-rs-work-cache-snapshot gerbil-rs-service-work-layout-metadata make-gerbil-rs-service-work-layout call-with-gerbil-rs-service-work-layout with-gerbil-rs-service-work-layout)

(include "service-progress.ss")

(defstruct budget-queue (front back))
;; A contended acquisition owns its notification until the receiving Scheme
;; thread consumes the grant. Direct acquisitions allocate no waiter or event.
(defstruct budget-waiter (thread event))
(defstruct service-budget (limit owner depth short bulk last streak short-debt bulk-debt stats lock event work permission hooks))
(def gerbil-rs-service-context (make-parameter #f))
(defrules make-budget-hooks ()
  ((_ budget acquire release)
   (vector
     (lambda () (when (positive? (service-budget-limit budget)) (acquire budget #f)))
     (lambda () (when (positive? (service-budget-limit budget)) (acquire budget #t)))
     (lambda () (when (positive? (service-budget-limit budget)) (release budget #f 0 0 #f)))
     (lambda (work cost span report?) (release budget work cost span report?)))))
(defrules runtime-unobserved! () ((_ budget phase) #!void))
(defrules runtime-mirror-enter! ()
  ((_ budget phase)
   (when (not (eq? (service-budget-owner budget) (current-thread))) (runtime-current! budget phase))))
(def (make-gerbil-rs-budget)
  (let (budget (make-service-budget 0 #f 0 (make-budget-queue [] []) (make-budget-queue [] [])
                 #f 0 0 0 (make-vector 6 0) (make-mutex) (make-condition-variable) #f #f #f))
    (set! (service-budget-hooks budget) (make-budget-hooks budget budget-acquire/core! budget-release/core!))
    budget))
(def (budget-empty? queue) (and (null? (budget-queue-front queue)) (null? (budget-queue-back queue))))
(def (budget-push! queue value) (set! (budget-queue-back queue) (cons value (budget-queue-back queue))))
(def (budget-pop! queue)
  (when (null? (budget-queue-front queue))
    (set! (budget-queue-front queue) (reverse (budget-queue-back queue)))
    (set! (budget-queue-back queue) []))
  (let (head (car (budget-queue-front queue)))
    (set! (budget-queue-front queue) (cdr (budget-queue-front queue))) head))
(def (budget-locked budget thunk)
  (dynamic-wind (lambda () (mutex-lock! (service-budget-lock budget))) thunk
    (lambda () (mutex-unlock! (service-budget-lock budget)))))
(def (gerbil-rs-budget-configure! budget limit)
  (unless (and (integer? limit) (<= 0 limit 256)
               (or (not (service-budget-work budget)) (positive? limit))) (error "invalid cooperative unit budget"))
  (budget-locked budget
    (lambda ()
      (unless (and (not (service-budget-owner budget))
                   (budget-empty? (service-budget-short budget)) (budget-empty? (service-budget-bulk budget)))
        (error "cooperative budget is active"))
      (set! (service-budget-limit budget) limit)
      (set! (service-budget-last budget) #f) (set! (service-budget-streak budget) limit)
      (set! (service-budget-short-debt budget) 0) (set! (service-budget-bulk-debt budget) 0)
      (set! (service-budget-stats budget) (make-vector 6 0)))))
(def (budget-count! budget index)
  (let (stats (service-budget-stats budget)) (vector-set! stats index (1+ (vector-ref stats index)))))
(def (budget-record! budget thread chosen short? bulk?)
  (let ((last (service-budget-last budget)) (stats (service-budget-stats budget)))
    (set! (service-budget-owner budget) thread)
    (set! (service-budget-depth budget) 1)
    (set! (service-budget-streak budget)
      (if (eq? chosen last) (min (service-budget-limit budget) (1+ (service-budget-streak budget))) 1))
    (set! (service-budget-last budget) chosen)
    (budget-count! budget (if chosen 0 1))
    (when (and short? bulk?) (budget-count! budget 5))
    (set! (service-budget-short-debt budget)
      (if (and (not chosen) short?) (1+ (service-budget-short-debt budget)) 0))
    (set! (service-budget-bulk-debt budget)
      (if (and chosen bulk?) (1+ (service-budget-bulk-debt budget)) 0))
    (vector-set! stats 3 (max (vector-ref stats 3) (service-budget-short-debt budget)))
    (vector-set! stats 4 (max (vector-ref stats 4) (service-budget-bulk-debt budget)))))
(def (budget-grant! budget)
  (when (and (not (service-budget-owner budget)) (gerbil-rs-work-grantable? budget))
    (let ((short? (not (budget-empty? (service-budget-short budget))))
          (bulk? (not (budget-empty? (service-budget-bulk budget)))))
      (when (or short? bulk?)
        (let* ((last (service-budget-last budget))
               (chosen (cond ((not bulk?) #t) ((not short?) #f)
                         ((>= (service-budget-streak budget) (service-budget-limit budget)) (not last))
                         (else last)))
               (waiter (budget-pop! (if chosen (service-budget-short budget) (service-budget-bulk budget))))
               (thread (if (budget-waiter? waiter) (budget-waiter-thread waiter) waiter)))
          (gerbil-rs-work-cache-refresh! budget)
          (budget-record! budget thread chosen short? bulk?)
          ;; Record ownership under the same mutex used by park/recheck. A
          ;; shared signal could wake the wrong waiter and strand the owner.
          (if (budget-waiter? waiter)
            (condition-variable-signal! (budget-waiter-event waiter))
            (condition-variable-broadcast! (service-budget-event budget))))))))
;; One arbitration implementation, specialized once when selecting the policy.
;; The event path has no Runtime call on uncontended acquisition or release.
(defrules define-budget-acquire ()
  ((_ (name budget short? extra ...) enter-event received-event)
(def (name budget short? extra ...)
  (let* ((lock (service-budget-lock budget)) (thread (current-thread))
         (work (service-budget-work budget))
         (execution (and short? work (memq (service-work-cached work) '(registered directed))
                         (gerbil-rs-work-execution-context))))
    (mutex-lock! lock)
    enter-event
    (when execution (execution-register! budget execution))
    (cond
      ((eq? (service-budget-owner budget) thread)
       (if (eq? (service-budget-last budget) short?)
         (set! (service-budget-depth budget) (1+ (service-budget-depth budget)))
         (begin (mutex-unlock! lock) (error "nested cooperative class change"))))
      ((and (not (service-budget-owner budget)) (gerbil-rs-work-grantable? budget)
            (budget-empty? (service-budget-short budget)) (budget-empty? (service-budget-bulk budget)))
       ;; No waiter exists: avoid queue allocation and a condition broadcast.
       ;; Selection/counters remain protected by the same Scheme mutex.
       (budget-record! budget thread short? short? (not short?)))
      (else
       (let* ((directed? (and work (eq? (service-work-cached work) 'directed)))
              (event (if directed? (make-condition-variable) (service-budget-event budget)))
              (waiter (if directed? (make-budget-waiter thread event) thread)))
         (budget-push! (if short? (service-budget-short budget) (service-budget-bulk budget)) waiter)
         (gerbil-rs-work-cache-refresh! budget)
         (budget-grant! budget)
         (unless (eq? (service-budget-owner budget) thread)
           (runtime-blocking! budget 1)
           (let wait ()
             (unless (eq? (service-budget-owner budget) thread)
               (mutex-unlock! lock event) (mutex-lock! lock) (wait)))
           (runtime-blocking! budget 0)))))
    received-event
    (when execution (execution-granted! budget execution))
    (let* ((work (service-budget-work budget))
           (permission (and work (service-work-cached work)
                         (if short? (service-work-short-span work) (service-work-bulk-span work)))))
      ;; The receiving Scheme thread updates its own turn state only after it
      ;; actually owns the grant. A releaser selecting another thread must not
      ;; publish into the releaser's parameter context.
      (when (and work (eq? (service-work-cached work) 'turn))
        (gerbil-rs-turn-granted! budget))
      ;; The copied grant belongs to the current owner, not to a continuation.
      ;; A specialized body reads it once after its before hook returns.
      (set! (service-budget-permission budget) permission)
      (mutex-unlock! lock) permission)))))
(define-budget-acquire (budget-acquire/core! budget short?)
  (runtime-unobserved! budget 1) (runtime-unobserved! budget 4))
(define-budget-acquire (budget-acquire/mirror! budget short?)
  (runtime-mirror-enter! budget 1) (runtime-current! budget 4))
;; Resume publication and restored nesting belong to the receiving transaction.
;; This specialization adds no test or callback to the ordinary handler path.
(define-budget-acquire (budget-resume! budget short? previous depth)
  (runtime-current! budget previous)
  (begin (runtime-current! budget previous) (set! (service-budget-depth budget) depth)))
(def (budget-acquire! budget short?)
  ((vector-ref (service-budget-hooks budget) (if short? 1 0))))
(defrules release-budget/locked! ()
  ((_ budget work cost span report? transition)
   (begin
     (unless (eq? (service-budget-owner budget) (current-thread)) (error "foreign cooperative unit release"))
     (when work (gerbil-rs-work-lease-account! budget work cost span report?))
     (if (> (service-budget-depth budget) 1)
       (set! (service-budget-depth budget) (1- (service-budget-depth budget)))
       (begin (gerbil-rs-work-flush! budget)
         (set! (service-budget-depth budget) 0)
         (set! (service-budget-owner budget) #f)
         (set! (service-budget-permission budget) #f)
         transition (budget-grant! budget))))))
(defrules define-budget-release ()
  ((_ name released!)
   (def (name budget work cost span report?)
     (budget-locked budget
       (lambda () (release-budget/locked! budget work cost span report? (released! budget 0)))))))
(def (budget-suspend! budget phase)
  (budget-locked budget (lambda ()
    (unless (eq? (service-budget-owner budget) (current-thread)) (error "foreign cooperative suspension"))
    (set! (service-budget-depth budget) 1)
    (budget-count! budget 2)
    (release-budget/locked! budget #f 0 0 #f (runtime-current! budget phase)))))
(define-budget-release budget-release/core! runtime-unobserved!)
(define-budget-release budget-release/mirror! runtime-current!)
(def (budget-release! budget (work #f) (cost 0) (span 0) (report? #f))
  ((vector-ref (service-budget-hooks budget) 3) work cost span report?))
(def (call-with-gerbil-rs-budget-context budget short? thunk)
  (if (zero? (service-budget-limit budget)) (thunk)
    (parameterize ((gerbil-rs-service-context (cons budget short?))) (thunk))))
(def (call-with-gerbil-rs-service-unit thunk)
  (let (context (gerbil-rs-service-context))
    (if (or (not context) (eq? (service-budget-owner (car context)) (current-thread))) (thunk)
      ;; Reentering a retained continuation reacquires on the same Scheme thread.
      ;; The dynamic extent releases on exceptions and continuation exits too.
      (dynamic-wind
        (lambda () (when (positive? (service-budget-limit (car context)))
          (budget-acquire! (car context) (cdr context)))) thunk
        (lambda () (when (positive? (service-budget-limit (car context)))
          (budget-release! (car context))))))))
(def (call-with-gerbil-rs-service-suspension thunk (reason 'external))
  (let* ((context (gerbil-rs-service-context)) (budget (and context (car context)))
         (ledger (and budget (gerbil-rs-runtime-state budget))))
    (if (and ledger (not (service-runtime-events? ledger)))
      (call-with-service-suspension/mirrored thunk reason)
      (call-with-service-suspension/fused thunk reason))))
(def (call-with-service-suspension/fused thunk reason)
  (let* ((context (gerbil-rs-service-context)) (budget (and context (car context)))
         (held (and budget (eq? (service-budget-owner budget) (current-thread))))
         (tracked (and budget (gerbil-rs-runtime-state budget) (gerbil-rs-runtime-context))))
    (if (not (or held tracked)) (thunk)
      (let ((depth 0) (previous 0) (unchanged? #f))
        (dynamic-wind
          (lambda ()
            (when tracked (set! previous (service-runtime-context-phase tracked)))
            (set! depth (if (eq? (service-budget-owner budget) (current-thread))
                          (service-budget-depth budget) 0))
            (set! unchanged? (and (zero? depth) (eq? reason 'runnable) (zero? previous)))
            (cond
              ((positive? depth) (budget-suspend! budget (case reason ((runnable) 0) ((barrier) 3) (else 2))))
              ((not unchanged?) (budget-locked budget (lambda ()
                (runtime-current! budget (case reason ((runnable) 0) ((barrier) 3) (else 2))))))))
          thunk
          (lambda ()
            (cond
              ((and (positive? depth) (positive? (service-budget-limit budget)))
               (budget-resume! budget (cdr context) previous depth))
              ((not unchanged?) (budget-locked budget (lambda () (runtime-current! budget previous)))))))))))
(def (call-with-service-suspension/mirrored thunk reason)
  (let* ((context (gerbil-rs-service-context)) (budget (and context (car context)))
         (held (and budget (eq? (service-budget-owner budget) (current-thread))))
         (tracked (and budget (gerbil-rs-runtime-state budget) (gerbil-rs-runtime-context))))
    (if (not (or held tracked)) (thunk)
      (let ((depth 0) (previous 0))
        (dynamic-wind
          (lambda ()
            (when tracked (set! previous (service-runtime-context-phase tracked)))
            (set! depth (if (eq? (service-budget-owner budget) (current-thread))
                          (service-budget-depth budget) 0))
            (when (positive? depth)
              (set! (service-budget-depth budget) 1)
              (budget-count! budget 2) (budget-release! budget))
            (budget-locked budget (lambda ()
              (runtime-current! budget (case reason ((runnable) 0) ((barrier) 3) (else 2))))))
          thunk
          (lambda ()
            (budget-locked budget (lambda () (runtime-current! budget previous)))
            (when (and (positive? depth) (positive? (service-budget-limit budget)))
              (budget-acquire! budget (cdr context))
              (set! (service-budget-depth budget) depth))))))))
(def (gerbil-rs-budget-snapshot budget)
  (budget-locked budget
    (lambda () (list->s64vector (append (vector->list (service-budget-stats budget))
      (list (+ (length (budget-queue-front (service-budget-short budget))) (length (budget-queue-back (service-budget-short budget))))
            (+ (length (budget-queue-front (service-budget-bulk budget))) (length (budget-queue-back (service-budget-bulk budget))))
            (if (service-budget-owner budget) 1 0)))))))

(def (gerbil-rs-budget-idle? budget)
  (budget-locked budget (lambda ()
    (and (not (service-budget-owner budget))
         (or (not (service-budget-work budget)) (zero? (service-work-pending (service-budget-work budget))))
         (budget-empty? (service-budget-short budget)) (budget-empty? (service-budget-bulk budget))))))

(def (gerbil-rs-budget-limit budget)
  (budget-locked budget (lambda () (service-budget-limit budget))))

(include "service-work.ss")

(include "service-runtime.ss")

(include "service-native.ss")
