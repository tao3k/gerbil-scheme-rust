;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Owner-local creation and classification, independent of application schemas.
(begin-foreign
 (c-define (gerbil-rs-scalar-object tag integer real)
   (int32 int64 double) int64 "gerbil_scheme_rust_scalar_object" "extern"
   (with-exception-catcher (lambda (_) 0)
     (lambda ()
       (let ((store gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!))
         (case tag
           ((0) (store #!void))
           ((1) (store (not (= integer 0))))
           ((2) (if (fixnum? integer) (store integer) 0))
           ((3) (if (and (<= 0 integer #x10ffff) (not (<= #xd800 integer #xdfff)))
                  (store (integer->char integer)) 0))
           ((4) (store real))
           ((5) (store '()))
           (else 0))))))
 (c-define (gerbil-rs-object-kind root index)
   (int64 int64) int32 "gerbil_scheme_rust_object_kind" "extern"
   (with-exception-catcher (lambda (_) -1)
     (lambda ()
       (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value root index)))
         (cond
           ((eq? value gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing) -1)
           ((eq? value #!void) 0) ((boolean? value) 1)
           ((and (integer? value) (exact? value)) 2) ((string? value) 3)
           ((null? value) 4) ((pair? value) 5) ((vector? value) 6)
           ((table? value) 7) ((char? value) 9)
           ((flonum? value) 10) ((u8vector? value) 11) ((symbol? value) 12)
           ((keyword? value) 13) ((procedure? value) 14)
           ((with-exception-catcher (lambda (_) #f) (lambda () (hash-table? value))) 8)
           (else 15)))))))
