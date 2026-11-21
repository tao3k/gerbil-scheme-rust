;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Application-declared executed work, independent of admission and result units.
;;; One VM owns these fields. Uncontended checkpoints never lock or yield;
;;; Scheme scheduler boundaries, not foreign OS threads, mutate this state.
(defstruct service-work (quantum spent pending ready offered short bulk handoffs maximum fast-plans budget-plans selected cached short-span bulk-span turn-grants turn-yields turn-boundaries receiver executions execution-checkpoint cycle runtime native))
(def (gerbil-rs-work-enabled? budget) (and (service-budget-work budget) #t))
(def (gerbil-rs-work-configure! budget quantum (maximum quantum) (cached? #f))
  (unless (and (integer? quantum) (<= 0 quantum 256)
               (integer? maximum) (or (and (zero? quantum) (zero? maximum))
                                      (and (positive? quantum) (<= quantum maximum 65536)))
               (gerbil-rs-budget-idle? budget)
               (or (zero? quantum) (positive? (service-budget-limit budget))))
    (error "work budget requires idle cooperative service"))
  (set! (service-budget-work budget)
    (and (positive? quantum)
      ;; Service-cycle policy is immutable epoch metadata, not another handler
      ;; execution mode. Both A/B arms use the same registered layout dispatch.
      (let (plan-mode (if (memq cached? '(executing cycle-control ready-cycle progress-cycle runtime-control runtime-joint runtime-event native-cycle)) 'registered cached?))
        (make-service-work quantum 0 0 0 0 0 0 0 maximum 0 0 0 plan-mode maximum maximum 0 0 0 #f
          (and (memq plan-mode '(registered directed)) (make-vector 5 0))
          (and (memq cached? '(executing cycle-control ready-cycle progress-cycle runtime-control runtime-joint runtime-event native-cycle)) #t)
          (and (memq cached? '(cycle-control ready-cycle progress-cycle runtime-control runtime-joint runtime-event native-cycle)) (vector (case cached? ((native-cycle) 6) ((runtime-event) 5) ((runtime-joint) 4) ((runtime-control) 3) ((progress-cycle) 2) ((ready-cycle) 1) (else 0)) 0 0 0 0 0 0 0 0)) #f #f))))
  (let (work (service-budget-work budget))
    (when (and work (memq cached? '(runtime-control runtime-joint runtime-event)))
      (set! (service-work-runtime work)
        (make-service-runtime budget (not (eq? cached? 'runtime-control)) (eq? cached? 'runtime-event)
          (make-vector 10 0) 0 0 0 0 (and (eq? cached? 'runtime-event) (make-table test: eq?)))))
    (when (and work (eq? cached? 'native-cycle))
      (set! (service-work-native work) (make-service-native budget [])))
    (set! (service-budget-hooks budget)
      (if (and work (memq cached? '(runtime-control runtime-joint)))
        (make-budget-hooks budget budget-acquire/mirror! budget-release/mirror!)
        (make-budget-hooks budget budget-acquire/core! budget-release/core!)))))
(def (gerbil-rs-work-event! budget update!)
  ;; Service-lock -> budget-lock is the existing control order. Lease release
  ;; never takes a service lock. Publish the obligation and its cache together.
  (when (service-budget-work budget)
    (budget-locked budget
      (lambda ()
        (let (work (service-budget-work budget))
          (when work (update! work) (gerbil-rs-work-cache-refresh! budget)))))))
(def (work-submitted/locked! work)
  (set! (service-work-pending work) (1+ (service-work-pending work)))
  (when (service-work-executions work)
    (let (counts (service-work-executions work))
      (vector-set! counts 0 (1+ (vector-ref counts 0))))))
(def (gerbil-rs-work-submitted! budget)
  (gerbil-rs-work-event! budget work-submitted/locked!))
(def (gerbil-rs-work-dispatch! budget short? submitted?)
  (let (ledger (gerbil-rs-runtime-state budget))
    (if (and submitted? ledger (service-runtime-events? ledger))
      (let (token #f)
        (gerbil-rs-work-event! budget (lambda (work)
          (work-submitted/locked! work)
          (set! token (runtime-register/locked! budget short?))))
        token)
      (let (token (make-gerbil-rs-runtime-context budget short?))
        (when submitted? (gerbil-rs-work-submitted! budget)) token))))
(def (gerbil-rs-work-started! budget)
  (gerbil-rs-work-event! budget
    (lambda (work)
      (unless (positive? (service-work-pending work)) (error "work dispatch without obligation"))
      (set! (service-work-pending work) (1- (service-work-pending work))))))
(def (gerbil-rs-work-ready! budget)
  (gerbil-rs-work-event! budget
    (lambda (work) (set! (service-work-ready work) (1+ (service-work-ready work))))))
;; A ready short graph owns the next service opportunity. No additional
;; worker grant can consume that opportunity before the host harvests it.
;; This fences grants, not an already-running declared span, roots or permits.
(def (gerbil-rs-work-grantable? budget)
  (let (work (service-budget-work budget))
    (or (not work)
        (case (service-work-cached work)
          ((reclaim) (= (service-work-ready work) (service-work-offered work)))
          ((receiver) (or (not (service-work-receiver work))
                         (= (service-work-ready work) (service-work-offered work))))
          (else #t)))))
(def (call-with-gerbil-rs-work-receiver budget thunk)
  (let ((work (service-budget-work budget)) (thread (current-thread)))
    (if (not (and work (eq? (service-work-cached work) 'receiver))) (thunk)
      (dynamic-wind
        (lambda ()
          (budget-locked budget
            (lambda ()
              (when (and (eq? (service-budget-work budget) work)
                         (eq? (service-work-cached work) 'receiver))
                (when (service-work-receiver work) (error "nested completion receiver"))
                (set! (service-work-receiver work) thread)
                (gerbil-rs-work-cache-refresh! budget)))))
        thunk
        (lambda ()
          (budget-locked budget
            (lambda ()
              (when (and (eq? (service-budget-work budget) work)
                         (eq? (service-work-receiver work) thread))
                ;; Departure without a harvest never acknowledges a graph.
                ;; It revokes only this receiver's execution opportunity.
                (set! (service-work-receiver work) #f)
                (gerbil-rs-work-cache-refresh! budget)
                (budget-grant! budget)))))))))
(def (gerbil-rs-work-drain! budget)
  ;; Native close may drain without another host harvest. Revoke only this
  ;; service fence, leaving class arbitration and every live lease intact.
  (budget-locked budget
    (lambda ()
      (let (work (service-budget-work budget))
        (when (and work (memq (service-work-cached work) '(reclaim receiver)))
          (set! (service-work-receiver work) #f)
          (set! (service-work-cached work) 'specialized)
          (budget-grant! budget))))))
(def (gerbil-rs-work-harvested! budget)
  ;; Settles actionability after transfer, not projection/root/capacity retirement.
  (gerbil-rs-work-event! budget
    (lambda (work)
      (set! (service-work-offered work) (service-work-ready work))
      (when (memq (service-work-cached work) '(reclaim receiver)) (budget-grant! budget)))))
(def (gerbil-rs-work-flush! budget)
  ;; The owning lease commits one accumulated work count on release/suspension,
  ;; before changing the owner. Checkpoints do not update both global totals.
  (let (work (service-budget-work budget))
    (when (and work (positive? (service-work-spent work)))
      (if (service-budget-last budget)
        (set! (service-work-short work) (+ (service-work-short work) (service-work-spent work)))
        (set! (service-work-bulk work) (+ (service-work-bulk work) (service-work-spent work))))
      (set! (service-work-spent work) 0))))
(def (gerbil-rs-work-snapshot budget)
  (let (work (service-budget-work budget))
    (if work
      (s64vector (service-work-quantum work)
        (+ (service-work-short work) (if (service-budget-last budget) (service-work-spent work) 0))
        (+ (service-work-bulk work) (if (service-budget-last budget) 0 (service-work-spent work)))
        (service-work-handoffs work) (service-work-pending work)
        (service-work-ready work) (service-work-offered work))
      (s64vector 0 0 0 0 0 0 0))))
(def (gerbil-rs-work-contended? budget work short?)
  (or (not (budget-empty? (if short? (service-budget-bulk budget) (service-budget-short budget))))
      (and (not short?) (positive? (service-work-pending work)))
      (and (or (not (eq? (service-work-cached work) 'receiver)) (service-work-receiver work))
           (> (service-work-ready work) (service-work-offered work)))))
(def (gerbil-rs-work-handoff! budget work short?)
  (when (budget-locked budget
          (lambda ()
            (and (gerbil-rs-work-contended? budget work short?)
              (begin
                ;; A yield offers scheduling only. In the reclamation policy,
                ;; only service harvest acknowledges the ready generation.
                (unless (memq (service-work-cached work) '(reclaim receiver))
                  (set! (service-work-offered work) (service-work-ready work)))
                (set! (service-work-handoffs work) (1+ (service-work-handoffs work)))
                (gerbil-rs-work-cache-refresh! budget) #t))))
    (call-with-gerbil-rs-service-suspension (lambda () (thread-yield!)) 'runnable)))
(defrules work-charge! ()
  ((_ budget work short? maximum cost more-work?)
   (when (and work (eq? (service-budget-work budget) work))
     (let (amount cost)
       (unless (and (integer? amount) (<= 1 amount maximum)
                    (eq? (service-budget-owner budget) (current-thread))
                    (eq? (service-budget-last budget) short?))
         (error "work checkpoint outside owned bounded span"))
       (set! (service-work-spent work) (+ (service-work-spent work) amount))
       (when (and more-work? (>= (service-work-spent work) (service-work-quantum work)))
         (gerbil-rs-work-handoff! budget work short?))))))
(def (gerbil-rs-service-work-checkpoint)
  ;; Callable strict-span API for arbitrary handlers and retained continuations.
  (let* ((context (gerbil-rs-service-context))
         (budget (and context (car context))) (short? (and context (cdr context)))
         (work (and budget (service-budget-work budget))))
    (and work
      (lambda (cost (more-work? #t))
        (work-charge! budget work short? (service-work-quantum work) cost more-work?)))))
(def (gerbil-rs-work-plan-state total)
  (unless (and (integer? total) (positive? total)) (error "invalid declared handler work"))
  (let* ((context (gerbil-rs-service-context))
         (budget (and context (car context))) (short? (and context (cdr context)))
         (work (and budget (service-budget-work budget))))
    (if (not work) (values total #f #f #f)
      (begin
        (unless (eq? (service-budget-owner budget) (current-thread))
          (error "work plan outside owned lease"))
        (let* ((fast? (and (<= total (service-work-maximum work))
                          (not (gerbil-rs-work-contended? budget work short?))))
               (span (if fast? total (min total (service-work-quantum work)))))
          (if (= span total)
            (set! (service-work-fast-plans work) (1+ (service-work-fast-plans work)))
            (set! (service-work-budget-plans work) (1+ (service-work-budget-plans work))))
          (set! (service-work-selected work) (max (service-work-selected work) span))
          (values span budget work short?))))))
(defrules with-gerbil-rs-service-work-plan ()
  ((_ (span charge!) total body ...)
   (let-values (((span budget work short?) (gerbil-rs-work-plan-state total)))
     ;; A lexical Scheme macro, not a per-record procedure or Rust callback.
     ;; Captured continuations retain this selected, bounded span. Replacement
     ;; policies make their old charge operation inert through the identity fence.
     (let-syntax ((charge! (syntax-rules ()
                            ((_ amount more?)
                             (work-charge! budget work short? span amount more?)))))
       body ...))))
(def (gerbil-rs-work-plan-snapshot budget)
  (let (work (service-budget-work budget))
    (if work
      (s64vector (service-work-quantum work) (service-work-maximum work)
        (service-work-fast-plans work) (service-work-budget-plans work) (service-work-selected work))
      (s64vector 0 0 0 0 0))))

;; Cache authority belongs to Scheme scheduling events, never an OS worker.
;; Limits are class permissions, not selected spans or completed work.
(def (gerbil-rs-work-cache-refresh! budget)
  (let (work (service-budget-work budget))
    (when (and work (service-work-cached work))
      (set! (service-work-short-span work)
        (if (gerbil-rs-work-contended? budget work #t) (service-work-quantum work) (service-work-maximum work)))
      (set! (service-work-bulk-span work)
        (if (gerbil-rs-work-contended? budget work #f) (service-work-quantum work) (service-work-maximum work))))))

(defstruct service-work-layout (cost cache))
(def (make-gerbil-rs-service-work-layout cost)
  ;; Application layout validation is cold; arbitrary positive layouts remain valid.
  (unless (and (integer? cost) (positive? cost)) (error "invalid declared work layout"))
  (make-service-work-layout cost #f))

(def (gerbil-rs-work-lease-account! budget work cost span report?)
  ;; Fold terminal whole-work credit and plan diagnostics into the existing
  ;; release critical section. No handler-side whole-span checkpoint is needed.
  (when (and work (eq? (service-budget-work budget) work))
    (when (positive? cost)
      (if (service-budget-last budget)
        (set! (service-work-short work) (+ (service-work-short work) cost))
        (set! (service-work-bulk work) (+ (service-work-bulk work) cost))))
    (when report?
      (if (= span report?)
        (set! (service-work-fast-plans work) (1+ (service-work-fast-plans work)))
        (set! (service-work-budget-plans work) (1+ (service-work-budget-plans work))))
      (set! (service-work-selected work) (max (service-work-selected work) span)))))

(def (call-with-gerbil-rs-service-work-layout layout thunk)
  ;; The thunk returns the one Scheme value required by a service handler.
  ;; Scheme computation inside it retains all language/runtime capabilities.
  (let* ((total (service-work-layout-cost layout)) (context (gerbil-rs-service-context))
         (budget (and context (car context))) (work (and budget (service-budget-work budget))))
    (gerbil-rs-work-layout-dispatch layout thunk context budget work total)))
(def (gerbil-rs-work-layout-dispatch layout thunk context budget work total)
    (cond
      ((not work) (call-with-gerbil-rs-service-unit (lambda () (thunk total #f))))
      ((not (service-work-cached work))
       (call-with-gerbil-rs-service-unit
         (lambda () (with-gerbil-rs-service-work-plan (span charge!) total
           (thunk span (lambda (cost more?) (charge! cost more?)))))))
      ((and (eq? (service-work-cached work) 'turn)
            (gerbil-rs-turn-current? (gerbil-rs-turn-context) budget work))
       (gerbil-rs-work-turn-unit layout thunk context budget work (gerbil-rs-turn-context)))
      ((eq? (service-work-cached work) 'turn)
       (when (gerbil-rs-turn-context) (gerbil-rs-turn-yield! 'budget))
       (gerbil-rs-work-cycle-unit layout thunk context budget work))
      ((memq (service-work-cached work) '(specialized reclaim receiver registered directed))
       (gerbil-rs-work-specialized-unit layout thunk context budget work))
      ((eq? (service-work-cached work) 'cycle)
       (gerbil-rs-work-cycle-unit layout thunk context budget work))
      ((eq? (service-work-cached work) 'activation)
       (gerbil-rs-work-activation-unit layout thunk context budget work))
      (else
       (let ((span #f) (normal? #f) (report? total) (checkpoint #f))
         (dynamic-wind
           (lambda ()
             (set! normal? #f)
             (let (permission (and (positive? (service-budget-limit budget))
                               (budget-acquire! budget (cdr context))))
               (unless span
                 ;; The grant copies permission under the existing mutex; no
                 ;; stale unlocked read between an obligation and invalidation.
                 ;; Reentry reacquires but preserves its already-selected span.
                 (set! span (min total permission))
                 (when (< span total)
                   (set! checkpoint (lambda (cost more?) (work-charge! budget work (cdr context) span cost more?)))))))
           (lambda ()
             (let (result (thunk span checkpoint)) (set! normal? #t) result))
           (lambda ()
             (when (positive? (service-budget-limit budget))
               (budget-release! budget work (if (and normal? (not checkpoint)) total 0) span report?))
             (set! report? #f)))))))

(def (gerbil-rs-work-cache-snapshot budget)
  (budget-locked budget
    (lambda ()
      (let (work (service-budget-work budget))
        (if (and work (service-work-cached work))
          (s64vector 1 (service-work-short-span work) (service-work-bulk-span work))
          (s64vector 0 0 0))))))

 ;; One per short job; retained continuations keep identity, not a new grant debt.
(defstruct service-execution (budget work phase))
(def gerbil-rs-work-execution-context (make-parameter #f))
(def (make-gerbil-rs-work-execution budget)
  (let (work (service-budget-work budget))
    (and work (memq (service-work-cached work) '(registered directed))
         (make-service-execution budget work 'accepted))))
(def (execution-live? budget ticket)
  (and ticket (eq? (service-execution-budget ticket) budget)
       (eq? (service-execution-work ticket) (service-budget-work budget))))
(def (execution-count! work index)
  (let (counts (service-work-executions work))
    (vector-set! counts index (1+ (vector-ref counts index)))))
(def (execution-settle! work)
  (unless (positive? (service-work-pending work)) (error "execution acknowledgement without debt"))
  (set! (service-work-pending work) (1- (service-work-pending work))))
(def (gerbil-rs-work-execution-runnable! ticket)
  (when ticket
    (gerbil-rs-work-event! (service-execution-budget ticket)
      (lambda (work)
        (when (and (eq? work (service-execution-work ticket))
                   (eq? (service-execution-phase ticket) 'accepted))
          (set! (service-execution-phase ticket) 'runnable)
          (execution-count! work 1))))))
;; Both operations below run under the existing budget mutex. A releaser may
;; assign a peer, but only the receiving thread acknowledges actual execution.
(def (execution-register! budget ticket)
  (when (and (execution-live? budget ticket)
             (eq? (service-execution-phase ticket) 'runnable))
    (set! (service-execution-phase ticket) 'registered)
    (execution-count! (service-execution-work ticket) 2)))
(def (execution-granted! budget ticket)
  (when (and (execution-live? budget ticket)
             (eq? (service-execution-phase ticket) 'registered))
    (unless (eq? (service-budget-owner budget) (current-thread))
      (error "execution acknowledgement before actual grant"))
    (let (work (service-execution-work ticket))
      (set! (service-execution-phase ticket) 'granted)
      (execution-settle! work) (execution-count! work 3)
      (gerbil-rs-work-cache-refresh! budget))))
(def (gerbil-rs-work-execution-finished! ticket)
  (when ticket
    (gerbil-rs-work-event! (service-execution-budget ticket)
      (lambda (work)
        (when (eq? work (service-execution-work ticket))
          (case (service-execution-phase ticket)
            ((accepted runnable)
             ;; Validation/early exception can finish without a grant. Its
             ;; terminal receipt settles debt without inventing execution.
             (execution-settle! work) (execution-count! work 4))
            ((registered) (error "registered execution ended before grant")))
          (set! (service-execution-phase ticket) 'finished))))))
(def (gerbil-rs-work-execution-snapshot budget)
  (budget-locked budget
    (lambda ()
      (let* ((work (service-budget-work budget))
             (counts (and work (service-work-executions work))))
        (if counts
          (list->s64vector (append (list 1 (service-work-pending work)) (vector->list counts)))
          (s64vector 0 0 0 0 0 0 0))))))

(include "service-work-activation.ss")
(include "service-work-turn.ss")

;; The policy epoch cannot change during active work. Only the opt-in service
;; boundary observes execution debt; control plans keep dispatch-only progress.
(def (gerbil-rs-work-execution-pending? budget)
  (let (work (service-budget-work budget))
    (and work (service-work-execution-checkpoint work)
      (budget-locked budget
        (lambda () (positive? (service-work-pending work)))))))

;; Per-boundary protocol counters, shared by both explicit cycle policies.
;; No handler, root, capacity or execution acknowledgement is updated here.
(def (gerbil-rs-work-cycle-state budget)
  (let (work (service-budget-work budget)) (and work (service-work-cycle work))))
(def (gerbil-rs-work-cycle-snapshot budget)
  (let (cycle (gerbil-rs-work-cycle-state budget))
    (if cycle
      (s64vector 1 (if (positive? (vector-ref cycle 0)) 1 0)
        (vector-ref cycle 1) (vector-ref cycle 2) (vector-ref cycle 3)
        (vector-ref cycle 4) (vector-ref cycle 5)
        (if (= (vector-ref cycle 0) 2) 1 0) (vector-ref cycle 7))
      (s64vector 0 0 0 0 0 0 0 0 0))))
;; Actual successful logical work, including the live lease accumulator. This
;; is not a grant count, readiness event, CPU timer or root retirement receipt.
(def (gerbil-rs-work-cycle-executed budget)
  (let (work (service-budget-work budget))
    (+ (service-work-short work) (service-work-bulk work) (service-work-spent work))))
