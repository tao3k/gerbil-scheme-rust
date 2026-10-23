;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Exercise the frozen SDK linker, rather than trusting a manifest assertion.
(import :gerbil/runtime/gambit)

(def (run directory arguments)
  (let* ((child (open-process
                  [path: (path-expand "bin/gsc" (gerbil-home))
                   directory: directory arguments: arguments
                   stdin-redirection: #f stdout-redirection: #f stderr-redirection: #f]))
         (status (process-status child)))
    (close-port child)
    (unless (= status 0) (error "metadata linker control failed" arguments status))))

(def (main output)
  (create-directory* output)
  (call-with-output-file (path-expand "deferred.scm" output)
    (lambda (port)
      (display "(declare (standard-bindings) (extended-bindings))\n(set! events (cons (active-context) events))\n(define metadata-answer 42)\n" port)))
  (call-with-output-file (path-expand "main.scm" output)
    (lambda (port)
      (display "(declare (standard-bindings) (extended-bindings))\n(define events '(main))\n(define active-context (make-parameter 'wrong))\n(if (not (equal? events '(main))) (error \"premature metadata initialization\"))\n(parameterize ((active-context 'metadata)) (##load-module 'deferred))\n(if (not (equal? events '(metadata main))) (error \"metadata activation context/order\"))\n(##load-module 'deferred)\n(if (not (equal? events '(metadata main))) (error \"metadata initialized twice\"))\n(if (not (= metadata-answer 42)) (error \"metadata result\"))\n(display \"LAZY-METADATA-INITIALIZATION-OK\")\n(newline)\n" port)))
  (run output ["-c" "deferred.scm"])
  (run output ["-c" "main.scm"])
  (run output ["-e" "(link-incremental '((\"deferred.c\" (preload . #f)) \"main.c\") output: \"linked.c\")"])
  (for-each (lambda (name) (run output ["-obj" (string-append name ".c")]))
            '("deferred" "main" "linked"))
  (run output ["-exe" "-o" "metadata-control" "deferred.o" "main.o" "linked.o"])
  (let* ((child (open-process [path: (path-expand "metadata-control" output)
                              stdin-redirection: #f stdout-redirection: #f stderr-redirection: #f]))
         (status (process-status child)))
    (close-port child)
    (unless (= status 0) (error "metadata initialization order changed" status))))
