(import :std/test :package-fixture/answer)
(export failing-test)
(def failing-test
  (test-suite "Retained failure"
    (test-case "negative control" (check-equal? (answer) 41))))
