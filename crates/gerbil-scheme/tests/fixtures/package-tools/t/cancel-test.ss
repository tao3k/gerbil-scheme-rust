(import :std/test)
(extern namespace: gerbil-scheme-rust/scheme/runtime gerbil-rs-flush-build-output!)
(export cancel-test)
(def cancel-test
  (test-suite "Cancellation drain"
    (test-case "entered"
      (call-with-output-file "entered" (lambda (port) (display "entered" port)))
      (displayln "PACKAGE-CANCEL-READY=" (path-expand "output-observed" (current-directory)))
      (gerbil-rs-flush-build-output!)
      (let loop ()
        (unless (file-exists? "release") (thread-sleep! 0.001) (loop)))
      (check #t => #t))))
