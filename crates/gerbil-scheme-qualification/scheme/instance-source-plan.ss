;; Owner-only exact periodic schedule. A frame keeps its original planned time
;; until verified consumption; Actors never share or mutate the source cursor.
(defstruct mailbox-source-plan (time carry whole fraction rate))

(def (mailbox-source-plan-create rate)
  (make-mailbox-source-plan 0 0 (quotient 1000000000 rate) (modulo 1000000000 rate) rate))

(def (mailbox-source-plan-integral! plan)
  (mailbox-source-plan-time-set! plan
    (+ (mailbox-source-plan-time plan) (mailbox-source-plan-whole plan))))

(def (mailbox-source-plan-fractional! plan)
  (let* ((carry (+ (mailbox-source-plan-carry plan) (mailbox-source-plan-fraction plan)))
         (rate (mailbox-source-plan-rate plan)) (overflow? (>= carry rate)))
    (mailbox-source-plan-time-set! plan
      (+ (mailbox-source-plan-time plan) (mailbox-source-plan-whole plan) (if overflow? 1 0)))
    (mailbox-source-plan-carry-set! plan (if overflow? (- carry rate) carry))))

;; Select once per source lifetime. The qualified 500k/s and 1k/s paths have
;; integral periods and do not check/update fractional bookkeeping per message.
(def (mailbox-source-plan-step plan)
  (if (zero? (mailbox-source-plan-fraction plan))
    mailbox-source-plan-integral! mailbox-source-plan-fractional!))

(def (mailbox-source-plan-contract)
  ;; Compare transition results with the unchanged closed-form oracle, including
  ;; non-divisor rates, carry boundaries and ordinals near the five-million end.
  (for-each (lambda (rate)
    (for-each (lambda (ordinal)
      (let* ((base 1234567890123) (whole (quotient 1000000000 rate))
             (fraction (modulo 1000000000 rate))
             (plan (make-mailbox-source-plan
               (+ base (quotient (* ordinal 1000000000) rate))
               (modulo (* ordinal fraction) rate) whole fraction rate))
             (advance (mailbox-source-plan-step plan)))
        (let step ((remaining 5) (current ordinal))
          (unless (and (= (mailbox-source-plan-time plan)
                         (+ base (quotient (* current 1000000000) rate)))
                       (= (mailbox-source-plan-carry plan)
                         (modulo (* current fraction) rate)))
            (error "native source plan changed original schedule"))
          (when (positive? remaining)
            (advance plan) (step (1- remaining) (1+ current))))))
      '(0 1 2 17 127 128 4999997))) '(1 3 7 1000 499999 500000))
  (s64vector 0))
