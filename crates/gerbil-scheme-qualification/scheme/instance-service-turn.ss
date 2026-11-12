;; Native ownership controls for the actual Scheme Actor turn API.
(def (orders-turn-opportunity fail?)
  (let ((budget (make-gerbil-rs-budget)) (parent (current-thread)) (events [])
        (layout (make-gerbil-rs-service-work-layout 48))
        (short-layout (make-gerbil-rs-service-work-layout 4)))
    (gerbil-rs-budget-configure! budget 1) (gerbil-rs-work-configure! budget 4 48 'turn)
    (let (bulk (spawn (lambda () (call-with-gerbil-rs-budget-context budget #f
                 (lambda () (call-with-gerbil-rs-service-turn
                   (lambda (yield!)
                     ;; No Actor quantum/yield callback is invoked here. The
                     ;; next item must create an opportunity from its work bound.
                     (with-catch (lambda (_) #t)
                       (lambda () (call-with-gerbil-rs-service-work-layout layout
                         (lambda (span cp)
                           (set! events (cons 1 events))
                           (thread-send parent 'selected) (thread-receive)
                           (when fail? (error "injected failed turn item")) #t))))
                     (call-with-gerbil-rs-service-work-layout layout
                       (lambda (_ cp)
                         (set! events (cons 3 events))
                         (when cp (cp 4 #f)) #t)))))))))
      (thread-receive)
      (gerbil-rs-work-submitted! budget)
      (let (short (spawn (lambda ()
                     (gerbil-rs-work-started! budget)
                     (call-with-gerbil-rs-budget-context budget #t
                       (lambda () (call-with-gerbil-rs-service-work-layout short-layout
                         (lambda (_ __) (set! events (cons 2 events)) #t)))))))
        (let wait ()
          (unless (= (s64vector-ref (gerbil-rs-budget-snapshot budget) 6) 1) (thread-yield!) (wait)))
        (thread-send bulk 'continue) (thread-join! bulk) (thread-join! short)))
    (unless (and (equal? (reverse events) '(1 2 3)) (gerbil-rs-budget-idle? budget))
      (error "turn fused items across short execution opportunity" fail?))))
(def (orders-turn-reservation)
  (let ((budget (make-gerbil-rs-budget)) (layout (make-gerbil-rs-service-work-layout 48))
        (small (make-gerbil-rs-service-work-layout 4)) (verified? #f) (caught? #f))
    (gerbil-rs-budget-configure! budget 1) (gerbil-rs-work-configure! budget 4 48 'turn)
    (call-with-gerbil-rs-budget-context budget #f
      (lambda () (call-with-gerbil-rs-service-turn
        (lambda (yield!)
          (with-catch (lambda (_) (set! caught? #t))
            (lambda () (call-with-gerbil-rs-service-work-layout layout
              (lambda (_ __)
                (unless (equal? (gerbil-rs-service-turn-snapshot) (s64vector 48 48 48 2))
                  (error "turn did not reserve the active declaration"))
                (call-with-gerbil-rs-service-suspension thread-yield!)
                (unless (equal? (gerbil-rs-service-turn-snapshot) (s64vector 48 48 48 2))
                  (error "suspension refunded an unfinished declaration"))
                (set! verified? #t) (error "injected unfinished return")))))
          (unless (and verified? caught?
                       (equal? (gerbil-rs-service-turn-snapshot) (s64vector 48 0 48 1))
                       (= (s64vector-ref (gerbil-rs-work-snapshot budget) 2) 0))
            (error "exception refunded reservation or credited unfinished work"))
          (call-with-gerbil-rs-service-work-layout small (lambda (_ __) #t))
          (unless (equal? (gerbil-rs-service-turn-snapshot) (s64vector 4 0 48 1))
            (error "next item did not obtain a fresh execution segment"))))))
    (unless (and (gerbil-rs-budget-idle? budget)
                 (= (s64vector-ref (gerbil-rs-work-snapshot budget) 2) 4))
      (error "turn reservation control leaked or duplicated work"))))
(def (orders-turn-replay)
  (let ((budget (make-gerbil-rs-budget)) (layout (make-gerbil-rs-service-work-layout 48))
        (resume #f) (phase 0) (caught? #f))
    (gerbil-rs-budget-configure! budget 1) (gerbil-rs-work-configure! budget 4 48 'turn)
    (call-with-gerbil-rs-budget-context budget #f
      (lambda ()
        (with-catch (lambda (_) (set! caught? #t))
          (lambda ()
            (call/cc (lambda (escape)
              (call-with-gerbil-rs-service-turn
                (lambda (yield!) (call-with-gerbil-rs-service-work-layout layout
                  (lambda (_ __)
                    (call/cc (lambda (k) (set! resume k) (escape #f)))
                    (when (= phase 2) (error "injected exceptional turn replay")) #t))))))
            (case phase ((0) (set! phase 1) (resume #t))
                        ((1) (set! phase 2) (resume #t)))))))
    (unless (and caught? (= phase 2) (gerbil-rs-budget-idle? budget)
                 (= (s64vector-ref (gerbil-rs-work-snapshot budget) 2) 48)
                 (= (s64vector-ref (gerbil-rs-work-plan-snapshot budget) 2) 1))
      (error "turn replay lost independent lease/work/report ownership"))))
(def (orders-turn-contract)
  (let (result (orders-activation-contract 'turn))
    (orders-turn-opportunity #f) (orders-turn-opportunity #t)
    (orders-turn-reservation) (orders-turn-peer-grant) (orders-turn-replay)
    (orders-turn-generic-contract)
    result))

(def (orders-turn-peer-grant)
  (let ((budget (make-gerbil-rs-budget)) (parent (current-thread))
        (layout (make-gerbil-rs-service-work-layout 48))
        (small (make-gerbil-rs-service-work-layout 4)))
    (gerbil-rs-budget-configure! budget 1) (gerbil-rs-work-configure! budget 4 48 'turn)
    (let (a (spawn (lambda () (call-with-gerbil-rs-budget-context budget #f
               (lambda () (call-with-gerbil-rs-service-turn
                 (lambda (yield!) (call-with-gerbil-rs-service-work-layout layout
                   (lambda (_ __)
                     (call-with-gerbil-rs-service-suspension
                       (lambda () (thread-send parent 'parked) (thread-receive)))
                     (unless (equal? (gerbil-rs-service-turn-snapshot) (s64vector 48 48 4 2))
                       (error "recipient did not publish its own smaller grant")) #t)))))))))
      (thread-receive)
      (call-with-gerbil-rs-budget-context budget #t
        (lambda () (call-with-gerbil-rs-service-turn
          (lambda (yield!) (call-with-gerbil-rs-service-work-layout small
            (lambda (_ __)
              (gerbil-rs-work-ready! budget)
              (thread-send a 'resume)
              (let wait ()
                (unless (= (s64vector-ref (gerbil-rs-budget-snapshot budget) 7) 1)
                  (thread-yield!) (wait))) #t))))))
      (thread-join! a))
    (gerbil-rs-work-harvested! budget)
    (unless (and (gerbil-rs-budget-idle? budget)
                 (= (s64vector-ref (gerbil-rs-work-snapshot budget) 1) 4)
                 (= (s64vector-ref (gerbil-rs-work-snapshot budget) 2) 48))
      (error "peer grant lost independent turn or work ownership"))))

(def (orders-turn-generic-contract)
  (let ((budget (make-gerbil-rs-budget)) (caught 0))
    (gerbil-rs-budget-configure! budget 1) (gerbil-rs-work-configure! budget 4 48 'turn)
    (let (actors (make-gerbil-rs-actor-batch
                   (lambda (_ input)
                     (unless (equal? (gerbil-rs-service-turn-snapshot) (s64vector 0 0 0 0))
                       (error "generic handler inherited a turn execution contract")) input) 1))
      (call-with-gerbil-rs-budget-context budget #f
        (lambda ()
          (unless (equal? (gerbil-rs-actor-batch! actors
                            (vector (s64vector 0) (vector (s64vector 42))))
                          (vector (s64vector 42)))
            (error "generic Actor execution changed"))))
      (gerbil-rs-actor-batch-close! actors))
    (call-with-gerbil-rs-budget-context budget #f
      (lambda () (call-with-gerbil-rs-service-turn
        (lambda (yield!)
          (for-each
            (lambda (bad)
              (with-catch (lambda (_) (set! caught (1+ caught)))
                (lambda () (orders-handler/rank (lambda () 1) 0 bad))))
            (list #f (s64vector 1 0 2)))
          (unless (and (= caught 2)
                       (equal? (gerbil-rs-service-turn-snapshot) (s64vector 8 0 48 1))
                       (= (s64vector-ref (gerbil-rs-work-snapshot budget) 2) 0))
            (error "validation exception escaped declared execution reservation"))))))
    (unless (gerbil-rs-budget-idle? budget) (error "generic/invalid contract left a lease"))))
