(import :std/test :std/actor (only-in :std/error Timeout?) :package-fixture/answer)
(export passing-test)

;; Top-level evaluation is separately observable from executing the Case.
(let (runs (if (file-exists? "test-imports.txt")
               (call-with-input-file "test-imports.txt" read) 0))
  (call-with-output-file (list path: "test-imports.txt" truncate: #t)
    (lambda (port) (write (+ runs 1) port))))

;; Exercise the installed v0.19 interaction API, not a replacement actor runtime.
(def (check-native-actor-contract)
  (let* ((protocol "/gerbil-scheme-rust/qualification/v0")
         (peer (spawn (lambda ()
                        (<- protocol: protocol
                            ((equal? "ping") (--> 42))))))
         (destination (LocalActor peer)))
    (with-unwind-protect
      (lambda ()
        (check-equal? (->> destination "ping" protocol timeout: 1) 42)
        (thread-join! peer)
        (check-exception (->> destination "ping" protocol timeout: 0.01) Timeout?)
        (check-exception (<- protocol: protocol timeout: 0.01 (_ (void))) Timeout?)
        (displayln "GERBIL-ACTOR-CONTRACT-OK reply=42 dead-peer=timeout receive-timeout=OK"))
      (lambda ()
        (with-catch void (lambda () (thread-terminate! peer)))
        (with-catch void (lambda () (thread-join! peer)))))))

(def passing-test
  (test-suite "C ABI package owner"
    (test-case "same process and compiled macro"
      (call-with-output-file (list path: "test-owner.txt" truncate: #t)
        (lambda (port) (write (##os-getpid) port)))
      (let ((runs (if (file-exists? "test-runs.txt")
                     (call-with-input-file "test-runs.txt" read) 0)))
        (call-with-output-file (list path: "test-runs.txt" truncate: #t)
          (lambda (port) (write (+ runs 1) port))))
      (check-equal? (answer) 42)
      (check-equal? (answer-literal) 42)
      (check-native-actor-contract))))
