#!/usr/bin/env gxi
;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

(import :std/build-script)

;; Canonical Gerbil build truth. Cargo owns Rust dependency resolution and
;; Bazel invokes this script as an outer orchestration layer.
(defbuild-script
  '("scheme/native")
  parallelize: 1)
