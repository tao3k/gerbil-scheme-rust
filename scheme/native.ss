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
  gerbil-rs-scheme-object-pair-car-raw
  gerbil-rs-scheme-object-pair-cdr-raw)

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
