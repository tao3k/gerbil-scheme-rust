;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Official numeric tower and strong tables; no borrowed heap words escape.
(def (gerbil-rs-exact-rational? value)
  (and (rational? value) (exact? value)))

(def (gerbil-rs-composite-view value kind)
  (case kind
    ((0 1)
     ;; Select official accessors once, then write into the final snapshot.
     ;; No temporary two-element vector is allocated per numeric object.
     (let-values (((valid? left right)
                   (if (= kind 0)
                     (values gerbil-rs-exact-rational? numerator denominator)
                     (values number? real-part imag-part))))
       (let* ((items (if (vector? value) value (vector value)))
              (count (vector-length items)) (out (make-vector (* 2 count))))
         (let loop ((index 0))
           (if (= index count) out
             (let (item (vector-ref items index))
               (and (valid? item)
                 (begin
                   (vector-set! out (* 2 index) (left item))
                   (vector-set! out (+ 1 (* 2 index)) (right item))
                   (loop (+ index 1))))))))))
    ((2)
     (and (or (table? value) (hash-table? value))
       (let ((out (make-vector (* 2 (if (table? value) (table-length value) (hash-length value))))) (index 0))
         ;; Official traversal invokes proc(key,value); order is unspecified.
         ((if (table? value) table-for-each
            (lambda (visit table) (hash-for-each visit table)))
           (lambda (key item)
             (vector-set! out index key)
             (vector-set! out (+ index 1) item)
             (set! index (+ index 2))) value)
         out)))
    (else #f)))

(begin-foreign
 (namespace ("gerbil-scheme-rust/scheme/runtime#"
   gerbil-rs-composite-create gerbil-rs-composite-snapshot
   gerbil-rs-table-create gerbil-rs-snapshot-length))
 (define gerbil-rs-snapshot-length
   (c-lambda ((pointer unsigned-int64) unsigned-int64) void "___arg1[0]=___arg2;"))
 (c-define (gerbil-rs-composite-create roots count kind)
   ((pointer int64) unsigned-int64 int32) int64
   "gerbil_scheme_rust_composite_create" "extern"
   (with-exception-catcher (lambda (_exception) 0)
     (lambda ()
       (if (not (and (fixnum? count) (fixnum? (* 2 count))
                     (or (= kind 0) (= kind 1)))) 0
         (let ((out (make-vector count)))
           (let loop ((index 0))
             (if (= index count)
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store! out)
               (let* ((left (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at roots (* 2 index))))
                      (right (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at roots (+ 1 (* 2 index)))))
                      (value (case kind
                               ((0) (and (integer? left) (exact? left)
                                         (integer? right) (exact? right) (not (= right 0))
                                         (/ left right)))
                               ((1) (and (real? left) (real? right)
                                         (make-rectangular left right))))))
                 (if (eq? value #f) 0
                   (begin (vector-set! out index value) (loop (+ index 1))))))))))))
 (c-define (gerbil-rs-table-create roots count equality)
   ((pointer int64) unsigned-int64 int32) int64
   "gerbil_scheme_rust_table_create" "extern"
   (with-exception-catcher (lambda (_exception) 0)
     (lambda ()
       (let ((test (case equality ((0 3) eq?) ((1 4) eqv?) ((2 5) equal?) (else #f)))
             (gerbil? (<= 3 equality 5)))
         (if (not (and test (fixnum? count)
                       (fixnum? (* 2 count)))) 0
           (let ((out (if gerbil?
                        (case equality ((3) (make-hash-table-eq))
                                       ((4) (make-hash-table-eqv)) (else (make-hash-table)))
                        (make-table test: test weak-keys: #f weak-values: #f))))
             (let loop ((index 0))
               (if (= index count)
                 (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store! out)
                 (let ((key (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at roots (* 2 index))))
                       (value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at roots (+ 1 (* 2 index))))))
                   (if (or (eq? key gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing)
                           (eq? value gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing)) 0
                     (begin (if gerbil? (hash-put! out key value) (table-set! out key value))
                            (loop (+ index 1)))))))))))))
 (c-define (gerbil-rs-composite-snapshot root index kind length)
   (int64 int64 int32 (pointer unsigned-int64)) int64
   "gerbil_scheme_rust_composite_snapshot" "extern"
   (with-exception-catcher (lambda (_exception) 0)
     (lambda ()
       (let* ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value root index))
              (snapshot (gerbil-scheme-rust/scheme/runtime#gerbil-rs-composite-view value kind)))
         (if (not snapshot) 0
           (let ((token (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store! snapshot)))
             (gerbil-scheme-rust/scheme/runtime#gerbil-rs-snapshot-length length (vector-length snapshot))
             token)))))))
