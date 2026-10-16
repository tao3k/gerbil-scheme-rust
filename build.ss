#!/usr/bin/env gxi
;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

(import :std/build-script)

;; Canonical Gerbil build truth. Cargo owns Rust dependency resolution and
;; Bazel invokes this script as an outer orchestration layer.
(defbuild-script
  '("scheme/utf8"
    (gxc: "scheme/runtime" (extra-inputs: ("scheme/runtime-collections.ss"
      "scheme/runtime-object-kinds.ss" "scheme/runtime-composites.ss")))
    "scheme/program-build"
    "scheme/python-runtime"
    "scheme/python-package")
  optimize: #t
  parallelize: 1)
