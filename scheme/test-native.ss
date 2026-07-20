;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

(import :gerbil-scheme-rust/scheme/native)

(unless (= (gerbil-rs-abi-version) 1)
  (error "native ABI version does not match Rust ABI version 1"))

(unless (= (gerbil-rs-add-i64 40 2) 42)
  (error "native scalar ABI returned the wrong value"))

(displayln "gerbil native scalar gate: 42")
