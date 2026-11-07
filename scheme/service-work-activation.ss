;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Read-only descriptors may be shared. Mutable invocation state is never pooled.
(defstruct service-work-plan (work cost span checkpoint))
(defstruct service-layout-cache (work plans))
(defstruct service-work-activation (plan normal? report?))

(def (gerbil-rs-work-build-plan budget work short? cost span)
  (make-service-work-plan work cost span
    (and (< span cost)
      (lambda (amount more?) (work-charge! budget work short? span amount more?)))))

(def (gerbil-rs-work-layout-entry layout budget work)
  ;; Public entry already checked layout type. Only this module publishes cache
  ;; entries, so repeated descriptor/layout type checks are not needed on hits.
  (let* ((cached (&service-work-layout-cache layout))
         (entry
          (if (and cached (eq? (&service-layout-cache-work cached) work)) cached
            (let* ((cost (service-work-layout-cost layout))
                   (q (min cost (service-work-quantum work)))
                   (m (min cost (service-work-maximum work)))
                   (bulk-whole (gerbil-rs-work-build-plan budget work #f cost m))
                   (bulk-slice (if (= q m) bulk-whole (gerbil-rs-work-build-plan budget work #f cost q)))
                   (short-whole (if (= m cost) bulk-whole (gerbil-rs-work-build-plan budget work #t cost m)))
                   (short-slice (if (= q m) short-whole
                                  (if (= q cost) bulk-slice (gerbil-rs-work-build-plan budget work #t cost q))))
                   (next (make-service-layout-cache work (vector bulk-whole bulk-slice short-whole short-slice))))
              ;; Publish one fully formed entry. Interrupted cold construction
              ;; can duplicate an entry, but cannot publish mixed policy fields.
              ;; One latest entry bounds layout retention; old continuations keep
              ;; their own immutable token, independently of this cache slot.
              (set! (service-work-layout-cache layout) next) next))))
    entry))

(def (gerbil-rs-work-layout-token layout budget work short? permission)
  (let (entry (gerbil-rs-work-layout-entry layout budget work))
    (vector-ref (&service-layout-cache-plans entry)
      (+ (if short? 2 0) (if (= permission (&service-work-maximum work)) 0 1)))))

(def (gerbil-rs-service-work-layout-metadata layout)
  ;; Opaque diagnostic identity only; not a lease, root or capacity authority.
  (service-work-layout-cache layout))

(def (gerbil-rs-work-activation-unit layout thunk context budget work)
  ;; Only the private token builder can initialize this plan field. Its nominal
  ;; type is proved there; use generated typed accessors here, not repeated
  ;; generic structure validation on each callback return.
  (let (activation (make-service-work-activation #f #f #t))
    (dynamic-wind
      (lambda ()
        (set! (service-work-activation-normal? activation) #f)
        (let (permission (and (positive? (service-budget-limit budget))
                          (budget-acquire! budget (cdr context))))
          (unless (service-work-activation-plan activation)
            (set! (service-work-activation-plan activation)
              (gerbil-rs-work-layout-token layout budget work (cdr context) permission)))))
      (lambda ()
        (let* ((plan (service-work-activation-plan activation))
               (result (thunk (&service-work-plan-span plan) (&service-work-plan-checkpoint plan))))
          (set! (service-work-activation-normal? activation) #t) result))
      (lambda ()
        (when (positive? (service-budget-limit budget))
          (let (plan (service-work-activation-plan activation))
            (budget-release! budget (&service-work-plan-work plan)
              (if (and (service-work-activation-normal? activation) (not (&service-work-plan-checkpoint plan)))
                (&service-work-plan-cost plan) 0)
              (&service-work-plan-span plan)
              (and (service-work-activation-report? activation) (&service-work-plan-cost plan)))))
        (set! (service-work-activation-report? activation) #f)))))

;; A successful Scheme return publishes executed work to the current lease.
;; Only outer lease release/suspension flushes that accumulator to class totals.
;; No normal/report flags survive in an invocation, and no completion root or
;; admission capacity can be retired by this logical-work receipt.
(defstruct service-cycle-activation (plan))
(def (gerbil-rs-work-cycle-unit layout thunk context budget work)
  (let (activation (make-service-cycle-activation #f))
    (dynamic-wind
      (lambda ()
        (let (permission (and (positive? (service-budget-limit budget))
                          (budget-acquire! budget (cdr context))))
          (unless (service-cycle-activation-plan activation)
            (let (plan (gerbil-rs-work-layout-token layout budget work (cdr context) permission))
              (set! (service-cycle-activation-plan activation) plan)
              ;; Selection is reported once, at selection, never at unwind.
              (when (eq? (service-budget-work budget) work)
                (gerbil-rs-work-lease-account! budget work 0
                  (&service-work-plan-span plan) (&service-work-plan-cost plan)))))))
      (lambda ()
        (let* ((plan (service-cycle-activation-plan activation))
               (result (thunk (&service-work-plan-span plan) (&service-work-plan-checkpoint plan))))
          ;; A stale epoch is inert. A successful whole return is one logical
          ;; declaration, even on multishot replay; exceptional exits reach no
          ;; credit site. Slices already published their actual checkpoints.
          (when (and (eq? (service-budget-work budget) work)
                     (not (&service-work-plan-checkpoint plan)))
            ;; This credit site is private, reached only after our before hook
            ;; acquired this context's lease. Suspension restores that same
            ;; owner/class before returning. Public saved checkpoints still
            ;; validate owner/class; release retains its ownership check.
            (set! (service-work-spent work)
              (+ (service-work-spent work) (&service-work-plan-cost plan))))
          result))
      (lambda ()
        (when (positive? (service-budget-limit budget))
          (budget-release! budget))))))

;; The body itself is the continuation's invocation record. The shared before
;; hook reacquires on every reentry; selecting/reporting precedes user code only
;; on initial entry. A captured suffix naturally retains its immutable plan.
;; No normal flag, activation record or shared mutable invocation slot exists.
(defrules gerbil-rs-with-specialized-work ()
  ((_ (span checkpoint) layout context budget work body ...)
  (let* ((short? (cdr context)) (hooks (&service-budget-hooks budget)))
    (dynamic-wind
      (vector-ref hooks (if short? 1 0))
      (lambda ()
        (let* ((permission (service-budget-permission budget))
               ;; Cold metadata is constructed inside the already-owned unit.
               ;; No pre-lease duplicate builder or extra construction lease.
               (entry (gerbil-rs-work-layout-entry layout budget work))
               (plan (vector-ref (&service-layout-cache-plans entry)
                       (+ (if short? 2 0) (if (= permission (&service-work-maximum work)) 0 1)))))
          (when (eq? (service-budget-work budget) work)
            (gerbil-rs-work-lease-account! budget work 0
              (&service-work-plan-span plan) (&service-work-plan-cost plan)))
          (let* ((span (&service-work-plan-span plan))
                 (checkpoint (&service-work-plan-checkpoint plan))
                 (result (begin body ...)))
            (when (and (eq? (service-budget-work budget) work) (not (&service-work-plan-checkpoint plan)))
              (set! (service-work-spent work) (+ (service-work-spent work) (&service-work-plan-cost plan))))
            result)))
      (vector-ref hooks 2))))
)

(def (gerbil-rs-work-specialized-unit layout thunk context budget work)
  (gerbil-rs-with-specialized-work (span checkpoint) layout context budget work
    (thunk span checkpoint)))

;; Fixed-shape lexical API: caller Scheme code becomes the wind body directly.
;; The generic function fallback keeps the same semantics for other policies.
(defrules with-gerbil-rs-service-work-layout ()
  ((_ (span checkpoint) layout body ...)
   (let* ((value layout) (total (service-work-layout-cost value))
          (context (gerbil-rs-service-context))
          (budget (and context (car context))) (work (and budget (service-budget-work budget))))
     (if (and work (memq (service-work-cached work) '(specialized reclaim receiver registered directed)))
       (gerbil-rs-with-specialized-work (span checkpoint) value context budget work body ...)
       (gerbil-rs-work-layout-dispatch value (lambda (span checkpoint) body ...) context budget work total)))))
