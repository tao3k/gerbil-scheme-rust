(import :std/test :package-fixture/answer)
(export passing-test)
(def passing-test
  (test-suite "C ABI package owner"
    (test-case "same process and compiled macro"
      (call-with-output-file (list path: "test-owner.txt" truncate: #t)
        (lambda (port) (write (##os-getpid) port)))
      (check-equal? (answer) 42)
      (check-equal? (answer-literal) 42))))
