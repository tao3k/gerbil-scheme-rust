;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; A Scheme Actor turn owns the outer lease. Item winds borrow its depth;
;;; retained/delimited continuations can reacquire without an enclosing turn.
(defstruct service-turn-state (budget work owner active used permission))
;; The private parameter is only #f or this constructor's record. Typed reads
;; follow that module-owned invariant; no caller can supply a token as authority.
(def gerbil-rs-turn-context (make-parameter #f))
(def (gerbil-rs-turn-current? state budget work)
  (and state (eq? (&service-turn-state-budget state) budget)
       (eq? (&service-turn-state-work state) work)
       (eq? (&service-turn-state-owner state) (current-thread))))
(def (gerbil-rs-turn-granted! budget)
  ;; Called by the receiving owner under its acquire mutex. Parked peers
  ;; have distinct states; the releaser never updates another thread's context.
  (let* ((state (gerbil-rs-turn-context)) (work (service-budget-work budget)))
    (when (gerbil-rs-turn-current? state budget work)
      (set! (service-turn-state-permission state)
        (if (service-budget-last budget) (service-work-short-span work) (service-work-bulk-span work)))
      ;; Internal suspension does not exit item winds. Their outstanding spans
      ;; conservatively survive the new grant, including an exceptional suffix.
      (set! (service-turn-state-used state) (&service-turn-state-active state)))))
(def (gerbil-rs-turn-yield! (reason 'actor))
  (let* ((context (gerbil-rs-service-context)) (budget (and context (car context)))
         (work (and budget (service-budget-work budget))))
    (when (and work (eq? (service-work-cached work) 'turn)
               (eq? (service-budget-owner budget) (current-thread)))
      (if (eq? reason 'actor)
        (set! (service-work-turn-yields work) (1+ (service-work-turn-yields work)))
        (set! (service-work-turn-boundaries work) (1+ (service-work-turn-boundaries work))))))
  (call-with-gerbil-rs-service-suspension thread-yield! 'runnable))
(def (call-with-gerbil-rs-service-turn thunk)
  ;; Select the plan and yield procedure once per turn, not at every Actor item.
  ;; The callback receives a nullary yield operation; its quantum is unchanged.
  (let* ((context (gerbil-rs-service-context)) (budget (and context (car context)))
         (work (and budget (service-budget-work budget))))
    (if (and work (eq? (service-work-cached work) 'turn))
      (let (state (make-service-turn-state budget work (current-thread) 0 0 0))
        (parameterize ((gerbil-rs-turn-context state))
          (dynamic-wind
            (lambda ()
              (when (positive? (service-budget-limit budget))
                (let (held? (eq? (service-budget-owner budget) (current-thread)))
                  (budget-acquire! budget (cdr context))
                  (when (and (not held?) (eq? (service-budget-work budget) work))
                    (set! (service-work-turn-grants work) (1+ (service-work-turn-grants work)))))))
            (lambda () (thunk gerbil-rs-turn-yield!))
            (lambda () (when (positive? (service-budget-limit budget)) (budget-release! budget))))))
      (thunk thread-yield!))))
(def (gerbil-rs-turn-enter! budget short? state)
  (if (eq? (service-budget-owner budget) (current-thread))
    (begin
      (unless (eq? (service-budget-last budget) short?) (error "nested cooperative class change"))
      (set! (service-budget-depth budget) (1+ (service-budget-depth budget)))
      (&service-turn-state-permission state))
    (budget-acquire! budget short?)))
(def (gerbil-rs-turn-exit! budget)
  ;; Depth is mutated only by the owning Scheme thread. Queued peers cannot
  ;; grant themselves while that owner is present. Outer release still locks.
  (unless (eq? (service-budget-owner budget) (current-thread)) (error "foreign turn unit release"))
  (if (> (service-budget-depth budget) 1)
    (set! (service-budget-depth budget) (1- (service-budget-depth budget)))
    (budget-release! budget)))
(def (gerbil-rs-work-turn-unit layout thunk context budget work state)
  (let (activation (make-service-cycle-activation #f))
    (dynamic-wind
      (lambda ()
        (let* ((permission (and (positive? (service-budget-limit budget))
                             (gerbil-rs-turn-enter! budget (cdr context) state)))
               (prior (service-cycle-activation-plan activation))
               (span (if prior (&service-work-plan-span prior)
                       (min (service-work-layout-cost layout) permission))))
          (if (gerbil-rs-turn-current? state budget work)
            (when (and (eq? (service-budget-work budget) work)
                       (positive? (max (&service-turn-state-used state) (service-work-spent work)))
                       (> (+ (max (&service-turn-state-used state) (service-work-spent work)) span)
                          (&service-turn-state-permission state)))
              ;; One opportunity, never a retry loop for an old larger bound.
              (gerbil-rs-turn-yield! 'budget))
            ;; A cross-thread continuation preserves execution but cannot borrow
            ;; the original thread's reservation state. It gets an opportunity
            ;; on each entry instead of fusing an unbounded batch.
            (gerbil-rs-turn-yield! 'budget))
          (unless prior
            (let (plan (gerbil-rs-work-layout-token layout budget work (cdr context)
                         (if (gerbil-rs-turn-current? state budget work)
                           (&service-turn-state-permission state) permission)))
              (set! (service-cycle-activation-plan activation) plan)
              (gerbil-rs-work-lease-account! budget work 0
                (&service-work-plan-span plan) (&service-work-plan-cost plan))))
          (when (gerbil-rs-turn-current? state budget work)
            (let (selected (&service-work-plan-span (service-cycle-activation-plan activation)))
              (set! (service-turn-state-active state) (+ (&service-turn-state-active state) selected))
              (set! (service-turn-state-used state) (+ (&service-turn-state-used state) selected))))))
      (lambda ()
        (let* ((plan (service-cycle-activation-plan activation))
               (result (thunk (&service-work-plan-span plan) (&service-work-plan-checkpoint plan))))
          (when (and (eq? (service-budget-work budget) work) (not (&service-work-plan-checkpoint plan)))
            (set! (service-work-spent work) (+ (service-work-spent work) (&service-work-plan-cost plan))))
          result))
      (lambda ()
        (when (gerbil-rs-turn-current? state budget work)
          (set! (service-turn-state-active state)
            (- (&service-turn-state-active state) (&service-work-plan-span (service-cycle-activation-plan activation)))))
        (when (positive? (service-budget-limit budget)) (gerbil-rs-turn-exit! budget))))))

(def (gerbil-rs-service-turn-snapshot)
  ;; Diagnostic counts only, not a lease/capacity/root capability.
  (let (state (gerbil-rs-turn-context))
    (if state
      (s64vector (&service-turn-state-used state) (&service-turn-state-active state)
        (&service-turn-state-permission state) (service-budget-depth (&service-turn-state-budget state)))
      (s64vector 0 0 0 0))))

(def (gerbil-rs-turn-snapshot budget)
  (budget-locked budget
    (lambda ()
      (let (work (service-budget-work budget))
        (if (and work (eq? (service-work-cached work) 'turn))
          (s64vector 1 (service-work-turn-grants work) (service-work-turn-yields work)
            (service-work-turn-boundaries work))
          (s64vector 0 0 0 0))))))
