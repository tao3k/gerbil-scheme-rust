(import :std/test)
(export cancel-test)
(def cancel-test
  (test-suite "Cancellation drain"
    (test-case "entered"
      (call-with-output-file "entered" (lambda (port) (display "entered" port)))
      (let loop ()
        (unless (file-exists? "release") (thread-sleep! 0.001) (loop)))
      (check #t => #t))))
