;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
(import :std/test (only-in :std/error Error?)
        (only-in "../scheme/actor-batch.ss"
                 make-gerbil-rs-actor-batch gerbil-rs-actor-batch!
                 gerbil-rs-actor-batch-close!))
(export actor-batch-test)

(def actor-batch-test
  (test-suite "Actor coordinator failure ownership"
    (test-case "A drained failure raises on the caller and leaves every worker reusable"
      (let* ((calls 0)
             (service (make-gerbil-rs-actor-batch
                       (lambda (_ value)
                         (set! calls (1+ calls))
                         (if value value (error "intentional handler failure")))
                       4)))
        (dynamic-wind
          void
          (lambda ()
            (thread-send (current-thread) 'caller-message)
            (for-each
             (lambda (quantum)
               (let (before calls)
                 (check (with-catch (lambda (failure) failure)
                          (lambda ()
                            (gerbil-rs-actor-batch!
                             service (vector (s64vector quantum) '#(1 #f 3 4 5)))))
                        ? Error?)
                 (check (- calls before) => 5))
               (check (gerbil-rs-actor-batch!
                       service (vector (s64vector quantum) '#(6 7 8 9 10)))
                      => '#(6 7 8 9 10)))
             '(0 1 256))
            (check (thread-receive 1 'missing) => 'caller-message))
          (lambda () (gerbil-rs-actor-batch-close! service)))
        (check (with-catch (lambda (_) 'closed)
                 (lambda ()
                   (gerbil-rs-actor-batch! service (vector (s64vector 1) '#(1)))))
               => 'closed)))))
