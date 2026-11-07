(import :std/test)
(export products-built-test)
(def products-built-test
  (test-suite "Scheme-owned build products"
    (test-case "compiled module and macro objects exist"
      (for-each
        (lambda (artifact)
          (def path (path-expand artifact
                      (path-expand "lib/package-fixture/" (getenv "GERBIL_PATH"))))
          (check (file-exists? path) => #t)
          (check (> (file-info-size (file-info path)) 0) => #t))
        '("answer.ssi" "answer.o1" "answer~0.o1" "answer~1.o1")))))
