;; Native controls for immutable plan sharing and invocation-local unwind state.
(def (orders-activation-contract (mode 'activation))
  (unless (equal? (orders-lease-plan-contract mode) (s64vector 48 4 196 0))
    (error "activation mode lost lease contract"))
  (let ((budget (make-gerbil-rs-budget)) (layout (make-gerbil-rs-service-work-layout 48))
        (short-layout (make-gerbil-rs-service-work-layout 4)) (parent (current-thread))
        (caught? #f) (metadata #f) (checkpoint #f) (slice-metadata #f))
    (gerbil-rs-budget-configure! budget 1)
    (gerbil-rs-work-configure! budget 4 48 mode)
    (unless (not (gerbil-rs-service-work-layout-metadata layout)) (error "layout cache was not cold"))
    ;; A suspends without leaving its outer dynamic extent; B then completes.
    ;; Pooling their mutable normal/report flags would credit A's exception.
    (let (a (spawn
             (lambda ()
               (with-catch (lambda (_) (set! caught? #t))
                 (lambda () (call-with-gerbil-rs-budget-context budget #f
                   (lambda () (call-with-gerbil-rs-service-work-layout layout
                     (lambda (span cp)
                       (unless (and (= span 48) (not cp)) (error "invalid activation plan"))
                       (set! metadata (gerbil-rs-service-work-layout-metadata layout))
                       (call-with-gerbil-rs-service-suspension
                         (lambda () (thread-send parent 'suspended) (thread-receive)))
                       (error "injected A failure after B completion"))))))))))
      (thread-receive)
      (call-with-gerbil-rs-budget-context budget #f
        (lambda () (call-with-gerbil-rs-service-work-layout layout
          (lambda (span cp)
            (unless (eq? metadata (gerbil-rs-service-work-layout-metadata layout))
              (error "same-policy metadata was rebuilt")) #t))))
      (thread-send a 'resume) (thread-join! a))
    (unless (and caught? (= (s64vector-ref (gerbil-rs-work-snapshot budget) 2) 48))
      (error "invocation state aliases a completed peer"))
    ;; A successful reentry followed by an exceptional reentry must reset normal.
    (call-with-gerbil-rs-budget-context budget #f
      (lambda ()
        (let ((resume #f) (phase 0) (caught #f))
          (with-catch (lambda (_) (set! caught #t))
            (lambda ()
              (call/cc (lambda (escape)
                (call-with-gerbil-rs-service-work-layout layout
                  (lambda (span cp)
                    (call/cc (lambda (k) (set! resume k) (escape #f)))
                    (when (= phase 2) (error "injected exceptional replay")) #t))))
              (case phase
                ((0) (set! phase 1) (resume #t))
                ((1) (set! phase 2) (resume #t)))))
          (unless caught (error "exceptional replay did not unwind")))))
    (unless (= (s64vector-ref (gerbil-rs-work-snapshot budget) 2) 96)
      (error "exceptional replay inherited normal completion"))
    ;; Contended checkpoint procedures are immutable and shared too.
    (gerbil-rs-work-ready! budget)
    (call-with-gerbil-rs-budget-context budget #f
      (lambda ()
        (call-with-gerbil-rs-service-work-layout layout
          (lambda (span cp) (unless (= span 4) (error "missing sliced activation"))
            (set! checkpoint cp) (set! slice-metadata (gerbil-rs-service-work-layout-metadata layout)) (cp 4 #f) #t))
        (call-with-gerbil-rs-service-work-layout layout
          (lambda (span cp) (unless (eq? cp checkpoint) (error "checkpoint closure rebuilt per activation" (list mode span (procedure? cp) (procedure? checkpoint) (eq? slice-metadata (gerbil-rs-service-work-layout-metadata layout)))))
            (cp 4 #f) #t))))
    (gerbil-rs-work-harvested! budget)
    ;; A checkpoint cannot cross into another scheduling class, nor can nested
    ;; parameterization relabel a still-owned lease. Both failures preserve it.
    (call-with-gerbil-rs-budget-context budget #t
      (lambda () (call-with-gerbil-rs-service-work-layout short-layout
        (lambda (span cp)
          (unless (with-catch (lambda (_) #t) (lambda () (checkpoint 4 #f) #f))
            (error "bulk checkpoint charged a short lease"))
          (unless (with-catch (lambda (_) #t)
                    (lambda () (call-with-gerbil-rs-budget-context budget #f
                      (lambda () (call-with-gerbil-rs-service-work-layout layout (lambda (_ __) #f)))) #f))
            (error "nested class change succeeded")) #t))))
    (let (work (gerbil-rs-work-snapshot budget))
      (unless (and (= (s64vector-ref work 1) 4) (= (s64vector-ref work 2) 104))
        (error "checkpoint class rejection changed accounting")))
    ;; Same-class nested invocations share immutable metadata, not their flags.
    (call-with-gerbil-rs-budget-context budget #f
      (lambda () (with-catch (lambda (_) #t)
        (lambda () (call-with-gerbil-rs-service-work-layout layout
          (lambda (span cp)
            (call-with-gerbil-rs-service-work-layout short-layout (lambda (_ __) #t))
            (error "outer failure after nested completion")))))))
    (unless (= (s64vector-ref (gerbil-rs-work-snapshot budget) 2) 108)
      (error "nested invocation normal flags aliased"))
    ;; A new policy publishes a new immutable cache entry. Retained old metadata
    ;; remains unchanged and cannot become the new policy's accounting identity.
    (gerbil-rs-work-configure! budget 4 48 mode)
    (call-with-gerbil-rs-budget-context budget #f
      (lambda () (call-with-gerbil-rs-service-work-layout layout (lambda (_ __) #t))))
    (unless (not (eq? metadata (gerbil-rs-service-work-layout-metadata layout)))
      (error "replacement policy reused old metadata identity"))
    ;; The low-level Scheme budget can be disabled while this unit is suspended.
    ;; Resume must not acquire a zero-limit lease that outer unwind cannot release.
    ;; The Rust endpoint separately rejects configuration while jobs are active.
    (let (a (spawn (lambda () (call-with-gerbil-rs-budget-context budget #f
               (lambda () (call-with-gerbil-rs-service-work-layout layout
                 (lambda (_ __)
                   (call-with-gerbil-rs-service-suspension
                     (lambda () (thread-send parent 'parked-for-disable) (thread-receive))) #t)))))))
      (thread-receive)
      (gerbil-rs-work-configure! budget 0) (gerbil-rs-budget-configure! budget 0)
      (thread-send a 'resume) (thread-join! a))
    (unless (gerbil-rs-budget-idle? budget) (error "activation leaked a lease"))
    (s64vector 48 4 108 1)))

;; Observe the protocol while the outer lease is still active: selection is
;; already reported, nested success is visible, and suspension flushes once.
(def (orders-cycle-contract)
  (let ((result (orders-activation-contract 'cycle)) (budget (make-gerbil-rs-budget))
        (outer (make-gerbil-rs-service-work-layout 48))
        (inner (make-gerbil-rs-service-work-layout 4)) (verified? #f) (caught? #f))
    (gerbil-rs-budget-configure! budget 1)
    (gerbil-rs-work-configure! budget 4 48 'cycle)
    (call-with-gerbil-rs-budget-context budget #f
      (lambda () (with-catch (lambda (_) (set! caught? #t))
        (lambda () (call-with-gerbil-rs-service-work-layout outer
          (lambda (_ __)
            (unless (= (s64vector-ref (gerbil-rs-work-plan-snapshot budget) 2) 1)
              (error "selection receipt deferred until unwind"))
            (call-with-gerbil-rs-service-work-layout inner (lambda (_ __) #t))
            (unless (= (s64vector-ref (gerbil-rs-work-snapshot budget) 2) 4)
              (error "nested successful return not accrued to lease"))
            (call-with-gerbil-rs-service-suspension
              (lambda ()
                (unless (and (gerbil-rs-budget-idle? budget)
                             (= (s64vector-ref (gerbil-rs-work-snapshot budget) 2) 4))
                  (error "suspension did not settle current lease"))))
            (set! verified? #t)
            (error "outer failure after settled nested return")))))))
    (unless (and verified? caught? (gerbil-rs-budget-idle? budget)
                 (= (s64vector-ref (gerbil-rs-work-snapshot budget) 2) 4))
      (error "outer exception lost or duplicated settled nested work"))
    result))

(def (orders-specialized-contract)
  (let ((receipt (orders-activation-contract 'specialized))
        (budget (make-gerbil-rs-budget)) (layout (make-gerbil-rs-service-work-layout 4)))
    (gerbil-rs-budget-configure! budget 1)
    (gerbil-rs-work-configure! budget 4 48 'specialized)
    ;; Validate the public layout before any typed cache access or lease grant.
    ;; A literal vector is rejected statically by Gerbil's signature checker.
    ;; Receive the same forged value dynamically to exercise runtime rejection.
    (thread-send (current-thread) (vector 4 #f))
    (unless (with-catch (lambda (_) #t)
              (lambda () (with-gerbil-rs-service-work-layout (span cp) (thread-receive) #f) #f))
      (error "lexical API accepted a forged layout"))
    (call-with-gerbil-rs-budget-context budget #f
      (lambda ()
        (let ((resume #f) (phase 0) (caught #f))
          (with-catch (lambda (_) (set! caught #t))
            (lambda ()
              (call/cc (lambda (escape)
                (with-gerbil-rs-service-work-layout (span cp) layout
                  (unless (and (= span 4) (not cp)) (error "invalid lexical plan"))
                  (call/cc (lambda (k) (set! resume k) (escape #f)))
                  (when (= phase 2) (error "lexical exceptional replay")) #t)))
              (case phase ((0) (set! phase 1) (resume #t))
                          ((1) (set! phase 2) (resume #t)))))
          (unless caught (error "lexical replay exception missing")))))
    (unless (and (gerbil-rs-budget-idle? budget)
                 (= (s64vector-ref (gerbil-rs-work-snapshot budget) 2) 4)
                 (= (+ (s64vector-ref (gerbil-rs-work-plan-snapshot budget) 2)
                       (s64vector-ref (gerbil-rs-work-plan-snapshot budget) 3)) 1))
      (error "lexical replay duplicated selection or exception credit"))
    receipt))
