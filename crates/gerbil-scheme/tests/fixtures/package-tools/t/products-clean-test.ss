(import :std/test)
(export products-clean-test)
(def products-clean-test
  (test-suite "Scheme-owned package cleanup"
    (test-case "generated namespace is gone; inputs and other packages remain"
      (check (file-exists? (path-expand "lib/package-fixture" (getenv "GERBIL_PATH"))) => #f)
      (check (file-exists? (path-expand "lib/static/package-fixture__answer.scm"
                           (getenv "GERBIL_PATH"))) => #f)
      (check (file-exists? "answer.ss") => #t)
      (check (file-exists? (path-expand "lib/unrelated/keep" (getenv "GERBIL_PATH"))) => #t))))
