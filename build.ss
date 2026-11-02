#!/usr/bin/env gxi
;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

(import :std/build-script (only-in :std/make pkg-config-libs))
(export gerbil-scheme-rust-link-options)
(def (gerbil-scheme-rust-link-options)
  (filter (lambda (flag) (not (string-empty? flag)))
    (string-split (pkg-config-libs "openssl" "zlib" "sqlite3") #\space)))

;; Canonical Gerbil build truth. Cargo owns Rust dependency resolution and
;; Bazel invokes this script as an outer orchestration layer.
(defbuild-script
  `("scheme/utf8"
    (gxc: "scheme/runtime" (extra-inputs: ("scheme/runtime-collections.ss"
      "scheme/runtime-object-kinds.ss" "scheme/runtime-composites.ss")))
    (gxc: "scheme/service-budget" (extra-inputs: ("scheme/service-progress.ss"
      "scheme/service-work.ss" "scheme/service-work-activation.ss" "scheme/service-work-turn.ss" "scheme/service-runtime.ss" "scheme/service-native.ss")))
    "scheme/actor-batch"
    (gxc: "scheme/async-service" (extra-inputs: ("scheme/service-port-wake.ss")))
    "scheme/program-metadata"
    "scheme/program-build"
    (gxc: "scheme/package-tools" "-ld-options" ,(string-join (gerbil-scheme-rust-link-options) " "))
    "scheme/python-runtime"
    "scheme/python-package")
  optimize: #t
  parallelize: (##cpu-count))
