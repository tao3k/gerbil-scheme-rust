;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

(import :gerbil-scheme-rust/scheme/runtime
        :gerbil-scheme-rust/scheme/utf8)

;; Test internals without widening the standalone library's public API.
(extern namespace: #f
  gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-chunk-c
  gerbil-scheme-rust/scheme/runtime#gerbil-rs-observe-module-work
  gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
  gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing
  gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-release!)

;; Differential checks against the installed official codec, covering every
;; Unicode scalar in bounded blocks, including NUL and all encoding boundaries.
(let loop ((start 0) (checked 0))
  (if (> start #x10ffff)
    (displayln "UTF8-CONFORMANCE scalars=" checked " official-parity=OK")
    (let* ((end (min (+ start 4096) #x110000))
           (chars
            (let collect ((c start) (out '()))
              (cond ((= c end) (reverse out))
                    ((<= #xd800 c #xdfff) (collect (+ c 1) out))
                    (else (collect (+ c 1) (cons (integer->char c) out))))))
           (text (list->string chars))
           (actual (gerbil-rs-encode-utf8 text)))
      (unless (equal? actual (string->utf8 text))
        (error "UTF-8 official parity failed" start))
      (when (= (modulo start 65536) 0)
        (displayln "UTF8-CONFORMANCE completed-through=" end))
      (loop end (+ checked (string-length text))))))

(for-each
 (lambda (text)
   (unless (equal? (gerbil-rs-encode-utf8 text)
                   (string->utf8 text))
     (error "UTF-8 short/ASCII parity failed" text)))
 (list "" "a" (make-string 8192 #\a) "a\x00;汉字😀"))

(let loop ((codepoint #xd800))
 (when (<= codepoint #xdfff)
   ;; Public integer->char already rejects surrogates. Construct only this
   ;; negative fixture with the raw primitive to exercise our encoder's guard.
   (let ((text (string (##integer->char codepoint))))
     (unless (with-exception-catcher (lambda (_) #t)
               (lambda ()
                 (gerbil-rs-encode-utf8 text)
                 #f))
       (error "UTF-8 encoder accepted surrogate" codepoint)))
   ;; Exhaust the aligned block without another test-only module dependency.
   (loop (+ codepoint 1))))
(displayln "UTF8-CONFORMANCE surrogate-rejection=OK")

;; Private bulk-leaf rejection must not touch output before validating bounds.
(let ((output (make-u8vector 1024 #xa5)) (text (make-string 257 (integer->char #x1f600))))
  (for-each
   (lambda (bounds)
     (unless (< (apply gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-chunk-c
                       text output bounds) 0)
       (error "UTF-8 chunk accepted invalid bounds" bounds))
     (unless (equal? output (make-u8vector 1024 #xa5))
       (error "UTF-8 chunk changed output on invalid bounds" bounds)))
   (list (list 0 257 0) (list 1 0 0) (list 0 258 0)
         (list 0 256 1) (list 0 1 1025)
         (list -1 1 0) (list 0 -1 0) (list 0 1 -1)
         (list 0.0 1 0) (list 0 1.0 0) (list 0 1 #f)
         (list (expt 2 100) 1 0)))
  (unless (< (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-chunk-c
              output output 0 1 0) 0)
    (error "UTF-8 chunk accepted bytevector as string"))
  (unless (< (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-chunk-c
              text text 0 1 0) 0)
    (error "UTF-8 chunk accepted string as bytevector"))
  (displayln "UTF8-CONFORMANCE bounded-bulk-type-rejection=OK"))

;; Exact ASCII byte bounds are sufficient. Preserve the former offset case
;; as a positive control; the four-byte-scalar case above still rejects before
;; writing when that same span is genuinely too small.
(let ((output (make-u8vector 1024 #xa5)) (text (make-string 257 #\a)))
  (unless (= (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-chunk-c
               text output 0 256 1) 257)
    (error "UTF-8 chunk rejected exact ASCII bounds"))
  (unless (and (= (u8vector-ref output 0) #xa5)
               (equal? (subu8vector output 1 257) (make-u8vector 256 97))
               (equal? (subu8vector output 257 1024) (make-u8vector 767 #xa5)))
    (error "UTF-8 chunk exceeded exact ASCII bounds")))

;; The same Scheme string changes in place, not just between distinct roots.
;; Earlier bytevectors must remain independent and the next conversion must
;; observe the new character widths, including embedded NUL.
(let* ((text (make-string 4096 #\a))
       (first (gerbil-rs-encode-utf8 text)))
  (for-each
   (lambda (character)
     (string-set! text 2047 character)
     (unless (equal? (gerbil-rs-encode-utf8 text) (string->utf8 text))
       (error "UTF-8 mutable text parity failed" character))
     (unless (equal? first (string->utf8 (make-string 4096 #\a)))
       (error "UTF-8 output aliases mutable input")))
   (map integer->char (list #x0 #x7f #x80 #x7ff #x800 #xd7ff #xe000 #xffff #x10000 #x10ffff)))
  (displayln "UTF8-CONFORMANCE in-place-mutation snapshot-independence=OK"))

(unless (= (gerbil-rs-abi-version) 1)
  (error "native ABI version does not match Rust ABI version 1"))

(unless (= (gerbil-rs-add-i64 40 2) 42)
  (error "native scalar ABI returned the wrong value"))

(displayln "gerbil native scalar gate: 42")

(let* ((expected (make-string 8192 #\a))
       (token (gerbil-rs-root-string (string-copy expected)))
       (bytes (gerbil-rs-root-bytevector (u8vector 1 2 255))))
  ;; Only the registry retains the copied string across this explicit GC.
  (##gc)
  (unless (equal? expected
                 (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref token))
    (error "strong root did not survive GC"))
  (unless (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-release! token)
    (error "live root release failed"))
  (##gc)
  (unless (eq? (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref token)
               gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing)
    (error "released root remained visible"))
  (when (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-release! token)
    (error "duplicate root release succeeded"))
  (unless (equal? (u8vector 1 2 255)
                 (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref bytes))
    (error "unrelated strong root changed after release and GC"))
  (unless (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-release! bytes)
    (error "retained bytevector root release failed"))
  (displayln "ROOT-REGISTRY strong-GC release isolation=OK"))

;; A package operation must restore the native resolver after success, nested
;; observation and exceptions; a subsequent unrelated operation keeps its owner.
(let ((original ##get-module) (requests []))
  (def (reference module)
    (set! requests (cons module requests))
    'resolved)
  (def (observe action)
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-observe-module-work action))
  (dynamic-wind
    (lambda () (##get-module-set! reference))
    (lambda ()
      (unless (eq? (observe (lambda () (##get-module 'success))) 'resolved)
        (error "module observer changed resolver result"))
      (unless (eq? ##get-module reference)
        (error "module observer leaked after successful return"))
      (let ((caught #f) (outer #f))
        (observe
          (lambda ()
            (set! outer ##get-module)
            (with-catch
              (lambda (failure) (set! caught failure))
              (lambda ()
                (observe
                  (lambda ()
                    (##get-module 'nested)
                    (raise 'expected-resolver-test-failure)))))
            (unless (and (eq? caught 'expected-resolver-test-failure)
                         (eq? ##get-module outer))
              (error "nested resolver observation changed exception or owner")))))
      (unless (eq? ##get-module reference)
        (error "module observer leaked after nested exception"))
      (with-catch void
        (lambda () (observe (lambda () (raise 'expected-outer-failure)))))
      (unless (and (eq? ##get-module reference)
                   (eq? (##get-module 'after-failure) 'resolved)
                   (equal? (reverse requests) '(success nested after-failure)))
        (error "resolver delegation or failure restoration changed" requests)))
    (lambda () (##get-module-set! original)))
  (unless (eq? ##get-module original)
    (error "resolver test leaked native process state"))
  (displayln "MODULE-RESOLVER success exception nesting delegation restoration=OK"))
