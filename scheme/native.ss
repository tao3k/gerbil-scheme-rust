;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

(import :std/foreign)
(export gerbil-rs-abi-version gerbil-rs-add-i64 gerbil-rs-is-even-i64 gerbil-rs-compare-i64)

;; This is intentionally a scalar ABI proof. Rich values stay behind an opaque
;; runtime boundary until ownership, error, and thread contracts are versioned.
(begin-ffi (gerbil-rs-abi-version gerbil-rs-add-i64 gerbil-rs-is-even-i64 gerbil-rs-compare-i64)
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
     (else 0))))
