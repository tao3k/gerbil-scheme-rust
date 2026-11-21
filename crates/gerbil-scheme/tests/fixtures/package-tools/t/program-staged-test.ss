(import :std/test)
(export program-staged-test)
(def program-staged-test
  (test-suite "Compiler-owned program stage"
    (test-case "explicit caller stage remains separate from package cleanup"
      (check (file-exists? "program/program.json") => #t)
      (check (file-exists? "program/program__exe.scm") => #t))))
