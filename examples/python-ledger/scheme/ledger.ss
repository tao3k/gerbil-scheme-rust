;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; An independent stateful Scheme Library, not an ASCENT rule fixture.
(import :gerbil-scheme-rust/scheme/runtime
        :gerbil-scheme-rust/scheme/python-runtime
        :gerbil-scheme-rust/scheme/actor-batch
        :gerbil-python-ledger/scheme/plugin
        :gerbil/expander
        (only-in :gerbil/runtime/init gerbil-load-expander!))
(export main ledger)
(def entries [])
(def (main . _) (void))
(def actor-rank 0)
(def actor-rank-lock (make-mutex 'actor-rank))
(def actor-service #f)
(def (actor-handler worker item)
  (let* ((id (vector-ref item 0)) (lines (vector-ref item 1))
         (mixed? (vector-ref item 2)))
    (when (and mixed? (zero? (modulo id 7))) (thread-sleep! 0.001))
    (let ((total (let loop ((index 0) (sum 0))
                    (if (= index (vector-length lines)) sum
                      (let (line (vector-ref lines index))
                        (loop (1+ index) (+ sum (* (vector-ref line 0) (vector-ref line 1)))))))))
      (mutex-lock! actor-rank-lock)
      (set! actor-rank (1+ actor-rank))
      (let (rank actor-rank)
        (mutex-unlock! actor-rank-lock)
        (vector id total rank worker)))))
 ;; The delimiter returns inside Scheme; a saved resumption never returns through
 ;; a previous foreign stack. The opaque procedure remains a native Scheme root.
(def ledger-meta (lambda (_) (error "missing ledger delimiter")))
(def (ledger-reset thunk)
  (call/cc (lambda (return)
    (let (previous ledger-meta)
      (set! ledger-meta (lambda (value) (set! ledger-meta previous) (return value)))
      (let (value (thunk)) (ledger-meta value))))))
(def (ledger-shift handler)
  (call/cc (lambda (resume)
    (let (value (handler (lambda (value) (ledger-reset (lambda () (resume value))))))
      (ledger-meta value)))))
(def (ledger-configure paths)
  (gerbil-rs-python-configure! paths))
(def (ledger-close _)
  (when actor-service
    (gerbil-rs-actor-batch-close! actor-service)
    (set! actor-service #f))
  (void))
(def (ledger request)
  (unless (vector? request) (error "ledger request must be a vector"))
  (let (operation (vector-ref request 0))
    (cond
      ((equal? operation "actors")
       (unless actor-service (set! actor-service (make-gerbil-rs-actor-batch actor-handler 4)))
       (set! actor-rank 0)
       (gerbil-rs-actor-batch! actor-service
         (vector (s64vector (vector-ref request 2)) (vector-ref request 1))))
      ((equal? operation "append")
       (let (value (vector-ref request 1))
         (unless (integer? value) (error "ledger requires an exact integer"))
         (set! entries (cons value entries))
         (length entries)))
      ((equal? operation "total") (foldl + 0 entries))
      ((equal? operation "entries") (reverse entries))
      ((equal? operation "echo") (vector-ref request 1))
      ((equal? operation "shared")
       (let (value (vector "retained")) (vector value value)))
      ((equal? operation "cycle")
       (let (value (vector #f)) (vector-set! value 0 value) value))
      ((equal? operation "procedure") (lambda (v) (+ v 1)))
      ((equal? operation "saved-continuation")
       (ledger-reset (lambda ()
         (let (payload (make-vector 65536 1))
           (+ (vector-length payload) (ledger-shift (lambda (resume) resume)))))))
      ((equal? operation "continuation")
       (let ((saved #f) (initial 0))
         (set! initial (##continuation-capture (lambda (k) (set! saved k) 10)))
         (if saved
           (let (k saved) (set! saved #f) (##continuation-return k 41))
           (1+ initial))))
      ((equal? operation "dsl")
       (gerbil-load-expander!)
       (parameterize ((current-expander-context (make-top-context)))
         (eval-syntax '(import :gerbil/core :gerbil-python-ledger/scheme/plugin))
         (eval-syntax (call-with-input-string (vector-ref request 1) read-syntax))))
      ((or (equal? operation "macro-expand") (equal? operation "macro-eval"))
       (gerbil-load-expander!)
       (parameterize ((current-expander-context (make-top-context)))
         (eval-syntax '(import :gerbil/core :gerbil-python-ledger/scheme/plugin))
         (let* ((syntax (call-with-input-string (vector-ref request 1) read-syntax))
                (count (vector-ref request 2))
                (expected (eval-syntax syntax)))
           (let loop ((remaining count))
             (unless (zero? remaining)
               (if (equal? operation "macro-expand")
                 (core-expand syntax)
                 (unless (equal? (eval-syntax syntax) expected)
                   (error "macro evaluation changed result")))
               (loop (1- remaining))))
           (vector count expected))))
      ((equal? operation "gc") (##gc) (length entries))
      (else (error "unknown ledger operation" operation)))))
(begin-foreign
  (c-define (invoke request) (int64) int64 "gerbil_python_ledger_call" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call request
      gerbil-python-ledger/scheme/ledger#ledger)))

(begin-foreign
  (c-define (configure paths) (int64) int64 "gerbil_python_ledger_configure" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call paths
      gerbil-python-ledger/scheme/ledger#ledger-configure))
  (c-define (close request) (int64) int64 "gerbil_python_ledger_close" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call request
      gerbil-python-ledger/scheme/ledger#ledger-close)))
