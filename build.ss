#!/usr/bin/env gxi
;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

(import :std/build-script (only-in :std/make pkg-config-libs)
        (only-in "scheme/package-clean" clean-package-modules!))
(export gerbil-scheme-rust-link-options)
(include "scheme/link-options.ss")

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
    (gxc: "scheme/program-build" (extra-inputs: ("scheme/link-options.ss")))
    (gxc: "scheme/build" "-ld-options" ,(string-join (gerbil-scheme-rust-link-options) " "))
    "scheme/python-runtime"
    "scheme/python-package")
  optimize: #t
  parallelize: (##cpu-count))

(def package-build-main main)
(def (main . arguments)
  (let (result (apply package-build-main arguments))
    (when (equal? arguments ["clean"])
      (clean-package-modules!
        (pgetq package: (call-with-input-file "gerbil.pkg" read))
        (getenv "GERBIL_PATH" (path-expand "~/.gerbil"))))
    result))
