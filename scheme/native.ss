;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

(import :std/foreign)
(export
  gerbil-rs-abi-version
  gerbil-rs-add-i64
  gerbil-rs-is-even-i64
  gerbil-rs-compare-i64
  gerbil-rs-scheme-null-value-raw
  gerbil-rs-fixture-pair-raw
  gerbil-rs-fixture-proper-list-raw
  gerbil-rs-fixture-improper-list-raw
  gerbil-rs-scheme-object-null?-raw
  gerbil-rs-scheme-object-pair?-raw
  gerbil-rs-scheme-object-list?-raw)

;; This is intentionally a scalar ABI proof. Rich values stay behind an opaque
;; runtime boundary until ownership, error, and thread contracts are versioned.
(begin-ffi
  (gerbil-rs-abi-version
   gerbil-rs-add-i64
   gerbil-rs-is-even-i64
   gerbil-rs-compare-i64
   gerbil-rs-scheme-null-value-raw
   gerbil-rs-fixture-pair-raw
   gerbil-rs-fixture-proper-list-raw
   gerbil-rs-fixture-improper-list-raw
   gerbil-rs-scheme-object-null?-raw
   gerbil-rs-scheme-object-pair?-raw
   gerbil-rs-scheme-object-list?-raw)
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

  (c-define (gerbil-rs-scheme-object-null?-raw value)
      (scheme-object)
      int32
      "gerbil_scheme_rust_scheme_object_is_null_raw"
      "extern"
    (if (null? value) 1 0))

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
    (if (list? value) 1 0)))
