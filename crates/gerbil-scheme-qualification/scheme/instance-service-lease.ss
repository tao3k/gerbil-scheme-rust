(def (orders-lease-plan-contract (mode #t))
  (let ((budget (make-gerbil-rs-budget)) (layout (make-gerbil-rs-service-work-layout 48))
        (short-layout (make-gerbil-rs-service-work-layout 4)) (parent (current-thread))
        (short #f) (events []))
    (def (cache! short-span bulk-span)
      (unless (equal? (gerbil-rs-work-cache-snapshot budget) (s64vector 1 short-span bulk-span))
        (error "stale Scheme lease cache")))
    (for-each
      (lambda (bad)
        (unless (with-catch (lambda (_) #t)
                  (lambda () (make-gerbil-rs-service-work-layout bad) #f))
          (error "invalid cold work layout accepted")))
      '(0 -1 1.5 bad))
    (gerbil-rs-budget-configure! budget 1)
    (gerbil-rs-work-configure! budget 4 48 mode)
    (cache! 48 48)
    (gerbil-rs-work-submitted! budget) (cache! 48 4)
    (gerbil-rs-work-started! budget) (cache! 48 48)
    (gerbil-rs-work-ready! budget) (cache! 4 4)
    (gerbil-rs-work-harvested! budget) (cache! 48 48)
    (let (bulk (spawn (lambda ()
                (call-with-gerbil-rs-budget-context budget #f
                  (lambda ()
                    (call-with-gerbil-rs-service-work-layout layout
                      (lambda (span checkpoint)
                        (unless (and (= span 48) (not checkpoint)) (error "missing whole lease plan"))
                        (thread-send parent 'selected) (thread-receive)
                        ;; Queuing invalidated future permissions, not this span.
                        (cache! 48 4)
                        (unless (= span 48) (error "late arrival rewrote active span"))
                        (set! events (cons 1 events)) #t))
                    (thread-join! short)
                    (set! events (cons 3 events))
                    (call-with-gerbil-rs-service-work-layout layout
                      (lambda (span checkpoint)
                        (unless (and (= span 4) checkpoint) (error "ready generation missed cached slice"))
                        (checkpoint 4 #f) #t))
                    (gerbil-rs-work-harvested! budget) (cache! 48 48)
                    (call-with-gerbil-rs-service-work-layout layout
                      (lambda (span checkpoint)
                        (unless (and (= span 48) (not checkpoint)) (error "stale harvested permission")) #t)))))))
      (thread-receive)
      (gerbil-rs-work-submitted! budget)
      (set! short (spawn (lambda ()
                     (gerbil-rs-work-started! budget)
                     (call-with-gerbil-rs-budget-context budget #t
                       (lambda () (call-with-gerbil-rs-service-work-layout short-layout
                         (lambda (span checkpoint)
                           (unless (= span 4) (error "invalid short lease span"))
                           ;; It may use a whole four-record body even if its
                           ;; class permission was contended; there is no remainder.
                           (set! events (cons 2 events)) #t))))
                     (gerbil-rs-work-ready! budget))))
      (let wait ()
        (unless (= (s64vector-ref (gerbil-rs-budget-snapshot budget) 6) 1) (thread-yield!) (wait)))
      (thread-send bulk 'continue) (thread-join! bulk) (thread-join! short))
    (unless (equal? (reverse events) '(1 2 3)) (error "cached plan denied short execution"))
    (unless (= (s64vector-ref (gerbil-rs-work-snapshot budget) 2) 100) (error "whole release accounting mismatch"))
    (call-with-gerbil-rs-budget-context budget #f
      (lambda ()
        (let ((resume #f) (returns 0))
          (call/cc (lambda (escape)
            (call-with-gerbil-rs-service-work-layout layout
              (lambda (span checkpoint)
                (unless (and (= span 48) (not checkpoint)) (error "invalid reentered lease plan"))
                (call/cc (lambda (k) (set! resume k) (escape #f))) #t))))
          (unless (= returns 2) (set! returns (1+ returns)) (resume #t)))))
    (unless (= (s64vector-ref (gerbil-rs-work-snapshot budget) 2) 196) (error "reentry release accounting mismatch"))
    ;; An exception must release its lease without crediting an unfinished body.
    (call-with-gerbil-rs-budget-context budget #f
      (lambda () (with-catch (lambda (_) #t)
        (lambda () (call-with-gerbil-rs-service-work-layout layout (lambda (_ __) (error "injected lease failure")))))))
    (unless (= (s64vector-ref (gerbil-rs-work-snapshot budget) 2) 196) (error "failed body credited whole work"))
    (call-with-gerbil-rs-budget-context budget #f
      (lambda ()
        (let ((resume #f) (phase 0))
          (call/cc (lambda (escape)
            (call-with-gerbil-rs-service-work-layout layout
              (lambda (span checkpoint)
                (call/cc (lambda (k) (set! resume k) (escape #f)))
                (unless (= span 48) (error "reentry silently changed selected bound")) #t))))
          (case phase
            ((0) (set! phase 1) (gerbil-rs-work-configure! budget 0)
                 (gerbil-rs-budget-configure! budget 0) (resume #t))
            ((1) (set! phase 2) (gerbil-rs-budget-configure! budget 1)
                 (gerbil-rs-work-configure! budget 4 4 mode) (resume #t))
            ((2) (unless (equal? (gerbil-rs-work-snapshot budget) (s64vector 4 0 0 0 0 0 0))
                   (error "old lease credited replacement policy")))))))
    (unless (gerbil-rs-budget-idle? budget) (error "lease reentry retained ownership"))
    (s64vector 48 4 196 0)))

(include "instance-service-activation.ss")
