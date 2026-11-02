(import :std/test :package-fixture/answer)
(export passing-test)
(def passing-test
  (test-suite "C ABI package owner"
    (test-case "same process and compiled macro"
      (call-with-output-file (list path: "test-owner.txt" truncate: #t)
        (lambda (port) (write (##os-getpid) port)))
      (let ((runs (if (file-exists? "test-runs.txt")
                      (call-with-input-file "test-runs.txt" read) 0)))
      (call-with-output-file (list path: "test-runs.txt" truncate: #t)
        (lambda (p) (write (+ runs 1) p))))
    (check-equal? (answer) 42)
      (check-equal? (answer-literal) 42))))
