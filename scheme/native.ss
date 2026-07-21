;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

(import :std/foreign)
(export
  gerbil-rs-abi-version
  gerbil-rs-add-i64
  gerbil-rs-is-even-i64
  gerbil-rs-compare-i64
  gerbil-rs-scheme-null-value-raw
  gerbil-rs-fixture-void-raw
  gerbil-rs-fixture-pair-raw
  gerbil-rs-fixture-proper-list-raw
  gerbil-rs-fixture-improper-list-raw
  gerbil-rs-fixture-true-raw
  gerbil-rs-fixture-false-raw
  gerbil-rs-fixture-fixnum-raw
  gerbil-rs-fixture-char-ascii-raw
  gerbil-rs-fixture-char-bmp-raw
  gerbil-rs-fixture-char-non-bmp-raw
  gerbil-rs-fixture-flonum-finite-raw
  gerbil-rs-fixture-flonum-nan-raw
  gerbil-rs-fixture-flonum-pos-inf-raw
  gerbil-rs-fixture-flonum-neg-inf-raw
  gerbil-rs-fixture-flonum-neg-zero-raw
  gerbil-rs-fixture-bytevector-raw
  gerbil-rs-scheme-object-null?-raw
  gerbil-rs-scheme-object-void?-raw
  gerbil-rs-scheme-object-bytevector?-raw
  gerbil-rs-scheme-object-pair?-raw
  gerbil-rs-scheme-object-list?-raw
  gerbil-rs-scheme-object-boolean?-raw
  gerbil-rs-scheme-object-boolean-value-raw
  gerbil-rs-scheme-object-fixnum?-raw
  gerbil-rs-scheme-object-fixnum-value-raw
  gerbil-rs-scheme-object-char?-raw
  gerbil-rs-scheme-object-char-value-raw
  gerbil-rs-scheme-object-flonum?-raw
  gerbil-rs-scheme-object-flonum-value-raw
  gerbil-rs-scheme-object-bytevector-length-raw
  gerbil-rs-scheme-object-bytevector-u8-ref-raw
  gerbil-rs-bytevector->bytestring-root-raw
  gerbil-rs-bytestring->bytevector-root-raw
  gerbil-rs-root-string-length-raw
  gerbil-rs-root-string-char-ref-raw
  gerbil-rs-root-bytevector-length-raw
  gerbil-rs-root-bytevector-u8-ref-raw
  gerbil-rs-root-release-raw
  gerbil-rs-scheme-object-pair-car-raw
  gerbil-rs-scheme-object-pair-cdr-raw)

;; Values created by conversion APIs must remain reachable across the C ABI.
;; Rust owns the returned positive token and releases it through the matching
;; root-release export.  A token of zero is reserved for fail-closed errors.
(def gerbil-rs-next-root-id 1)
(def gerbil-rs-rooted-values [])

(def (gerbil-rs-rooted-value-store! value)
  (let (root-id gerbil-rs-next-root-id)
    (set! gerbil-rs-next-root-id (1+ root-id))
    (set! gerbil-rs-rooted-values
      (cons (cons root-id value) gerbil-rs-rooted-values))
    root-id))

(def (gerbil-rs-rooted-value-ref root-id)
  (let lp ((rest gerbil-rs-rooted-values))
    (cond
     ((null? rest) #f)
     ((= (caar rest) root-id) (cdar rest))
     (else (lp (cdr rest))))))

(def (gerbil-rs-rooted-values-remove rest root-id)
  (cond
   ((null? rest) (values rest #f))
   ((= (caar rest) root-id) (values (cdr rest) #t))
   (else
    (call-with-values
     (lambda () (gerbil-rs-rooted-values-remove (cdr rest) root-id))
     (lambda (tail found?)
       (values (if found? (cons (car rest) tail) rest) found?))))))

(def (gerbil-rs-rooted-value-release! root-id)
  (call-with-values
   (lambda ()
     (gerbil-rs-rooted-values-remove gerbil-rs-rooted-values root-id))
   (lambda (rooted-values found?)
     (when found?
       (set! gerbil-rs-rooted-values rooted-values))
     found?)))

(def (gerbil-rs-bytestring-delimiter code)
  (cond
   ((= code -1) #f)
   ((and (>= code 0)
         (<= code #x10ffff)
         (not (<= #xd800 code #xdfff)))
    (integer->char code))
   (else #!void)))

;; Embedded native archives link this module without the complete Gerbil std
;; module graph. Keep the official :std/misc/bytes wire semantics in this AOT
;; bridge so these conversion calls do not leave unresolved module procedures.
(def gerbil-rs-upper-hex-digits "0123456789ABCDEF")

(def (gerbil-rs-upper-hex-digit value)
  (string-ref gerbil-rs-upper-hex-digits value))

(def (gerbil-rs-hex-value character)
  (cond
   ((char<=? #\0 character #\9)
    (- (char->integer character) (char->integer #\0)))
   ((char<=? #\A character #\F)
    (+ 10 (- (char->integer character) (char->integer #\A))))
   ((char<=? #\a character #\f)
    (+ 10 (- (char->integer character) (char->integer #\a))))
   (else -1)))

(def (gerbil-rs-u8vector->bytestring value delimiter)
  (if (and (u8vector? value) (or (not delimiter) (char? delimiter)))
    (let* ((length (u8vector-length value))
           (delimiter-count (if (and (> length 0) delimiter) (1- length) 0))
           (bytestring (make-string (+ (* length 2) delimiter-count))))
      (let lp ((index 0) (offset 0))
        (when (< index length)
          (let ((byte (u8vector-ref value index)))
            (when (and (> index 0) delimiter)
              (string-set! bytestring offset delimiter)
              (set! offset (1+ offset)))
            (string-set!
             bytestring
             offset
             (gerbil-rs-upper-hex-digit (arithmetic-shift byte -4)))
            (string-set!
             bytestring
             (1+ offset)
             (gerbil-rs-upper-hex-digit (bitwise-and byte #x0f)))
            (lp (1+ index) (+ offset 2)))))
      bytestring)
    #f))

(def (gerbil-rs-hex-byte bytestring offset)
  (let ((high (gerbil-rs-hex-value (string-ref bytestring offset)))
        (low (gerbil-rs-hex-value (string-ref bytestring (1+ offset)))))
    (if (and (>= high 0) (>= low 0))
      (+ (arithmetic-shift high 4) low)
      -1)))

(def (gerbil-rs-bytestring->u8vector bytestring delimiter)
  (if (and (string? bytestring) (or (not delimiter) (char? delimiter)))
    (let* ((length (string-length bytestring))
           (valid-length?
            (if delimiter
              (or (zero? length) (zero? (modulo (1+ length) 3)))
              (zero? (modulo length 2))))
           (byte-count
            (if delimiter (quotient (1+ length) 3) (quotient length 2)))
           (value (and valid-length? (make-u8vector byte-count))))
      (if value
        (let lp ((index 0))
          (if (< index byte-count)
            (let* ((offset (if delimiter (* index 3) (* index 2)))
                   (delimiter-valid?
                    (or (not delimiter)
                        (zero? index)
                        (eq? delimiter (string-ref bytestring (1- offset)))))
                   (byte (and delimiter-valid?
                              (gerbil-rs-hex-byte bytestring offset))))
              (if (and byte (>= byte 0))
                (begin
                  (u8vector-set! value index byte)
                  (lp (1+ index)))
                #f))
            value))
        #f))
    #f))

(def (gerbil-rs-bytevector->bytestring-root value delimiter-code)
  (let (bytestring
        (gerbil-rs-u8vector->bytestring
         value
         (gerbil-rs-bytestring-delimiter delimiter-code)))
    (if bytestring (gerbil-rs-rooted-value-store! bytestring) 0)))

(def (gerbil-rs-bytestring->bytevector-root bytestring delimiter-code)
  (let (bytevector
        (gerbil-rs-bytestring->u8vector
         bytestring
         (gerbil-rs-bytestring-delimiter delimiter-code)))
    (if bytevector (gerbil-rs-rooted-value-store! bytevector) 0)))

;; This is intentionally a scalar ABI proof. Rich values stay behind an opaque
;; runtime boundary until ownership, error, and thread contracts are versioned.
(begin-ffi
  (gerbil-rs-abi-version
   gerbil-rs-add-i64
   gerbil-rs-is-even-i64
   gerbil-rs-compare-i64
   gerbil-rs-scheme-null-value-raw
   gerbil-rs-fixture-void-raw
   gerbil-rs-fixture-pair-raw
   gerbil-rs-fixture-proper-list-raw
   gerbil-rs-fixture-improper-list-raw
   gerbil-rs-fixture-true-raw
   gerbil-rs-fixture-false-raw
   gerbil-rs-fixture-fixnum-raw
   gerbil-rs-fixture-char-ascii-raw
   gerbil-rs-fixture-char-bmp-raw
   gerbil-rs-fixture-char-non-bmp-raw
   gerbil-rs-fixture-flonum-finite-raw
   gerbil-rs-fixture-flonum-nan-raw
   gerbil-rs-fixture-flonum-pos-inf-raw
   gerbil-rs-fixture-flonum-neg-inf-raw
   gerbil-rs-fixture-flonum-neg-zero-raw
   gerbil-rs-fixture-bytevector-raw
   gerbil-rs-scheme-object-null?-raw
   gerbil-rs-scheme-object-void?-raw
   gerbil-rs-scheme-object-bytevector?-raw
   gerbil-rs-scheme-object-pair?-raw
   gerbil-rs-scheme-object-list?-raw
   gerbil-rs-scheme-object-boolean?-raw
   gerbil-rs-scheme-object-boolean-value-raw
   gerbil-rs-scheme-object-fixnum?-raw
   gerbil-rs-scheme-object-fixnum-value-raw
   gerbil-rs-scheme-object-char?-raw
   gerbil-rs-scheme-object-char-value-raw
   gerbil-rs-scheme-object-flonum?-raw
   gerbil-rs-scheme-object-flonum-value-raw
   gerbil-rs-scheme-object-bytevector-length-raw
   gerbil-rs-scheme-object-bytevector-u8-ref-raw
   gerbil-rs-bytevector->bytestring-root-raw
   gerbil-rs-bytestring->bytevector-root-raw
   gerbil-rs-root-string-length-raw
   gerbil-rs-root-string-char-ref-raw
   gerbil-rs-root-bytevector-length-raw
   gerbil-rs-root-bytevector-u8-ref-raw
   gerbil-rs-root-release-raw
   gerbil-rs-scheme-object-pair-car-raw
   gerbil-rs-scheme-object-pair-cdr-raw)
  (c-define (gerbil-rs-abi-version)
    () unsigned-int32
    "gerbil_scheme_rust_abi_version"
    "extern"
    1)
  (c-define (gerbil-rs-add-i64 left right)
    (int64 int64) int64
    "gerbil_scheme_rust_add_i64"
    "extern"
      (+ left right))
  (c-define (gerbil-rs-is-even-i64 value)
      (int64)
      int32
      "gerbil_scheme_rust_is_even_i64"
      "extern"
    (if (even? value) 1 0))

  (c-define (gerbil-rs-compare-i64 left right)
      (int64 int64)
      int32
      "gerbil_scheme_rust_compare_i64"
      "extern"
    (cond
     ((< left right) -1)
     ((> left right) 1)
     (else 0)))

(c-define (gerbil-rs-scheme-null-value-raw)
    ()
    scheme-object
    "gerbil_scheme_rust_scheme_null_value_raw"
    "extern"
  '())

(c-define (gerbil-rs-fixture-void-raw)
    ()
    scheme-object
    "gerbil_scheme_rust_fixture_void_raw"
    "extern"
  #!void)

(c-define (gerbil-rs-fixture-pair-raw)
      ()
      scheme-object
      "gerbil_scheme_rust_fixture_pair_raw"
      "extern"
    (cons 1 2))

  (c-define (gerbil-rs-fixture-proper-list-raw)
      ()
      scheme-object
      "gerbil_scheme_rust_fixture_proper_list_raw"
      "extern"
    (list 1 2))

  (c-define (gerbil-rs-fixture-improper-list-raw)
      ()
      scheme-object
      "gerbil_scheme_rust_fixture_improper_list_raw"
      "extern"
    (cons 1 2))

  (c-define (gerbil-rs-fixture-true-raw)
      ()
      scheme-object
      "gerbil_scheme_rust_fixture_true_raw"
      "extern"
    #t)

  (c-define (gerbil-rs-fixture-false-raw)
      ()
      scheme-object
      "gerbil_scheme_rust_fixture_false_raw"
      "extern"
    #f)

  (c-define (gerbil-rs-fixture-fixnum-raw)
      ()
      scheme-object
      "gerbil_scheme_rust_fixture_fixnum_raw"
      "extern"
    42)

  (c-define (gerbil-rs-fixture-char-ascii-raw)
      ()
      scheme-object
      "gerbil_scheme_rust_fixture_char_ascii_raw"
      "extern"
    #\A)

  (c-define (gerbil-rs-fixture-char-bmp-raw)
      ()
      scheme-object
      "gerbil_scheme_rust_fixture_char_bmp_raw"
      "extern"
    (integer->char #x03bb))

  (c-define (gerbil-rs-fixture-char-non-bmp-raw)
      ()
      scheme-object
      "gerbil_scheme_rust_fixture_char_non_bmp_raw"
      "extern"
    (integer->char #x1f642))

  (c-define (gerbil-rs-fixture-flonum-finite-raw)
      ()
      scheme-object
      "gerbil_scheme_rust_fixture_flonum_finite_raw"
      "extern"
    42.5)

  (c-define (gerbil-rs-fixture-flonum-nan-raw)
      ()
      scheme-object
      "gerbil_scheme_rust_fixture_flonum_nan_raw"
      "extern"
    +nan.0)

  (c-define (gerbil-rs-fixture-flonum-pos-inf-raw)
      ()
      scheme-object
      "gerbil_scheme_rust_fixture_flonum_pos_inf_raw"
      "extern"
    +inf.0)

  (c-define (gerbil-rs-fixture-flonum-neg-inf-raw)
      ()
      scheme-object
      "gerbil_scheme_rust_fixture_flonum_neg_inf_raw"
      "extern"
    -inf.0)

(c-define (gerbil-rs-fixture-flonum-neg-zero-raw)
    ()
    scheme-object
    "gerbil_scheme_rust_fixture_flonum_neg_zero_raw"
    "extern"
  -0.0)

(c-define (gerbil-rs-fixture-bytevector-raw)
    ()
    scheme-object
    "gerbil_scheme_rust_fixture_bytevector_raw"
    "extern"
  #u8(255 127 11 1 0))

(c-define (gerbil-rs-scheme-object-null?-raw value)
    (scheme-object)
    int32
    "gerbil_scheme_rust_scheme_object_is_null_raw"
    "extern"
  (if (null? value) 1 0))

(c-define (gerbil-rs-scheme-object-void?-raw value)
    (scheme-object)
    int32
    "gerbil_scheme_rust_scheme_object_is_void_raw"
    "extern"
  (if (eq? value #!void) 1 0))

(c-define (gerbil-rs-scheme-object-bytevector?-raw value)
    (scheme-object)
    int32
    "gerbil_scheme_rust_scheme_object_is_bytevector_raw"
    "extern"
  (if (u8vector? value) 1 0))

(c-define (gerbil-rs-scheme-object-pair?-raw value)
      (scheme-object)
      int32
      "gerbil_scheme_rust_scheme_object_is_pair_raw"
      "extern"
    (if (pair? value) 1 0))

  (c-define (gerbil-rs-scheme-object-list?-raw value)
      (scheme-object)
      int32
      "gerbil_scheme_rust_scheme_object_is_list_raw"
      "extern"
    (if (list? value) 1 0))

  (c-define (gerbil-rs-scheme-object-boolean?-raw value)
      (scheme-object)
      int32
      "gerbil_scheme_rust_scheme_object_is_boolean_raw"
      "extern"
    (if (boolean? value) 1 0))

  (c-define (gerbil-rs-scheme-object-boolean-value-raw value)
      (scheme-object)
      int32
      "gerbil_scheme_rust_scheme_object_boolean_value_raw"
      "extern"
    (if value 1 0))

  (c-define (gerbil-rs-scheme-object-fixnum?-raw value)
      (scheme-object)
      int32
      "gerbil_scheme_rust_scheme_object_is_fixnum_raw"
      "extern"
    (if (fixnum? value) 1 0))

  (c-define (gerbil-rs-scheme-object-fixnum-value-raw value)
      (scheme-object)
      long
      "gerbil_scheme_rust_scheme_object_fixnum_value_raw"
      "extern"
    value)

  (c-define (gerbil-rs-scheme-object-char?-raw value)
      (scheme-object)
      int32
      "gerbil_scheme_rust_scheme_object_is_char_raw"
      "extern"
    (if (char? value) 1 0))

  (c-define (gerbil-rs-scheme-object-char-value-raw value)
      (scheme-object)
      int32
      "gerbil_scheme_rust_scheme_object_char_value_raw"
      "extern"
    (char->integer value))

  (c-define (gerbil-rs-scheme-object-flonum?-raw value)
      (scheme-object)
      int32
      "gerbil_scheme_rust_scheme_object_is_flonum_raw"
      "extern"
    (if (flonum? value) 1 0))

(c-define (gerbil-rs-scheme-object-flonum-value-raw value)
    (scheme-object)
    double
    "gerbil_scheme_rust_scheme_object_flonum_value_raw"
    "extern"
  value)

(c-define (gerbil-rs-scheme-object-bytevector-length-raw value)
    (scheme-object)
    int64
    "gerbil_scheme_rust_scheme_object_bytevector_length_raw"
    "extern"
  (if (u8vector? value) (u8vector-length value) -1))

(c-define (gerbil-rs-scheme-object-bytevector-u8-ref-raw value index)
    (scheme-object int64)
    int32
    "gerbil_scheme_rust_scheme_object_bytevector_u8_ref_raw"
    "extern"
  (if (and (u8vector? value)
           (>= index 0)
           (< index (u8vector-length value)))
    (u8vector-ref value index)
    -1))

(c-define (gerbil-rs-bytevector->bytestring-root-raw value delimiter-code)
    (scheme-object int32)
    int64
    "gerbil_scheme_rust_bytevector_to_bytestring_root_raw"
    "extern"
  (gerbil-scheme-rust/scheme/native#gerbil-rs-bytevector->bytestring-root
   value
   delimiter-code))

(c-define (gerbil-rs-bytestring->bytevector-root-raw bytestring delimiter-code)
    (char-string int32)
    int64
    "gerbil_scheme_rust_bytestring_to_bytevector_root_raw"
    "extern"
  (gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring->bytevector-root
   bytestring
   delimiter-code))

(c-define (gerbil-rs-root-string-length-raw root-id)
    (int64)
    int64
    "gerbil_scheme_rust_root_string_length_raw"
    "extern"
  (let ((value
         (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref root-id)))
    (if (string? value) (string-length value) -1)))

(c-define (gerbil-rs-root-string-char-ref-raw root-id index)
    (int64 int64)
    int32
    "gerbil_scheme_rust_root_string_char_ref_raw"
    "extern"
  (let ((value
         (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref root-id)))
    (if (and (string? value)
             (>= index 0)
             (< index (string-length value)))
      (char->integer (string-ref value index))
      -1)))

(c-define (gerbil-rs-root-bytevector-length-raw root-id)
    (int64)
    int64
    "gerbil_scheme_rust_root_bytevector_length_raw"
    "extern"
  (let ((value
         (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref root-id)))
    (if (u8vector? value) (u8vector-length value) -1)))

(c-define (gerbil-rs-root-bytevector-u8-ref-raw root-id index)
    (int64 int64)
    int32
    "gerbil_scheme_rust_root_bytevector_u8_ref_raw"
    "extern"
  (let ((value
         (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref root-id)))
    (if (and (u8vector? value)
             (>= index 0)
             (< index (u8vector-length value)))
      (u8vector-ref value index)
      -1)))

(c-define (gerbil-rs-root-release-raw root-id)
    (int64)
    int32
    "gerbil_scheme_rust_root_release_raw"
    "extern"
  (if (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-release! root-id)
    1
    0))

(c-define (gerbil-rs-scheme-object-pair-car-raw value)
      (scheme-object)
      scheme-object
      "gerbil_scheme_rust_scheme_object_pair_car_raw"
      "extern"
    (if (pair? value) (car value) #f))

  (c-define (gerbil-rs-scheme-object-pair-cdr-raw value)
      (scheme-object)
      scheme-object
      "gerbil_scheme_rust_scheme_object_pair_cdr_raw"
      "extern"
    (if (pair? value) (cdr value) #f)))
