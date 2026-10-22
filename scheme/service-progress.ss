;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Service hints never release native resources or schedule an Actor from Rust.
(def gerbil-rs-progress-context (make-parameter #f))
(defstruct progress-ledger (values lock))
(defstruct service-progress (ledger class remaining seen finished total))
(def (make-gerbil-rs-progress-ledger) (make-progress-ledger (vector 0 0) (make-mutex)))
(def (progress-locked ledger thunk)
  (dynamic-wind (lambda () (mutex-lock! (progress-ledger-lock ledger))) thunk
    (lambda () (mutex-unlock! (progress-ledger-lock ledger)))))
(def (make-gerbil-rs-progress ledger short? units)
  (unless (and (integer? units) (<= 1 units 4294967295))
    (error "invalid declared progress units"))
  (let ((class (if short? 1 0)) (totals (progress-ledger-values ledger)))
    (progress-locked ledger (lambda () (vector-set! totals class (+ (vector-ref totals class) units))))
    (make-service-progress ledger class units (make-table test: eqv?) #f units)))
(def (gerbil-rs-progress-unit! index)
  (let (progress (gerbil-rs-progress-context))
    (when progress
      (progress-locked (service-progress-ledger progress)
        (lambda ()
          (unless (service-progress-finished progress)
            (unless (and (integer? index) (<= 0 index) (< index (service-progress-total progress)))
              (error "invalid progress identity"))
            ;; Stable identities make continuation reentry idempotent. No user
            ;; callback runs under this Scheme-owned ledger lock.
            (unless (table-ref (service-progress-seen progress) index #f)
              (table-set! (service-progress-seen progress) index #t)
              (set! (service-progress-remaining progress) (1- (service-progress-remaining progress)))
              (let ((totals (progress-ledger-values (service-progress-ledger progress)))
                    (class (service-progress-class progress)))
                (vector-set! totals class (1- (vector-ref totals class)))))))))))
(def (gerbil-rs-progress-finish! progress)
  (when progress
    (progress-locked (service-progress-ledger progress)
      (lambda ()
        (unless (service-progress-finished progress)
          (let ((totals (progress-ledger-values (service-progress-ledger progress)))
                (class (service-progress-class progress)))
            (vector-set! totals class (- (vector-ref totals class) (service-progress-remaining progress))))
          (set! (service-progress-remaining progress) 0)
          (set! (service-progress-finished progress) #t)
          ;; Retained continuations need the inert terminal marker, not old IDs.
          (set! (service-progress-seen progress) #f))))))
(def (gerbil-rs-progress-receipt ledger ids)
  (progress-locked ledger (lambda ()
    (let (totals (progress-ledger-values ledger))
      (list->s64vector (append (list -6 (vector-ref totals 0) (vector-ref totals 1)) (s64vector->list ids)))))))

(def (gerbil-rs-progress-for-batch count)
  (let (progress (gerbil-rs-progress-context))
    ;; Arbitrary handlers may declare coarser work units. Preserve execution and
    ;; conservative finish-only progress when item identities do not match.
    (and progress (= (service-progress-total progress) count) progress)))
