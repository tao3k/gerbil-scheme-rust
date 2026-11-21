#!/usr/bin/env gxi
;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

(import :std/build-script (only-in :std/make pkg-config-libs)
        (only-in "scheme/package-clean" clean-package-modules!))
(export gerbil-scheme-rust-link-options gerbil-scheme-rust-program-build-spec)
(include "scheme/link-options.ss")

;; The runtime projection shares its declarations with the full package build.
(def (runtime-build-spec)
  '("scheme/utf8"
    (gxc: "scheme/runtime" (extra-inputs: ("scheme/runtime-collections.ss"
      "scheme/runtime-object-kinds.ss" "scheme/runtime-composites.ss")))))

(def program-metadata-target "scheme/program-metadata")

;; The metadata observer runs in a separate Gerbil process during AOT staging.
(def (gerbil-scheme-rust-program-build-spec)
  (append (runtime-build-spec) (list program-metadata-target)))

;; Canonical Gerbil build truth. Cargo owns Rust dependency resolution and
;; Bazel invokes this script as an outer orchestration layer.
(defbuild-script
  (append (runtime-build-spec)
    `((gxc: "scheme/service-budget" (extra-inputs: ("scheme/service-progress.ss"
        "scheme/service-work.ss" "scheme/service-work-activation.ss" "scheme/service-work-turn.ss" "scheme/service-runtime.ss" "scheme/service-native.ss")))
      "scheme/runtime-host"
      "scheme/actor-batch"
      (gxc: "scheme/arrow" "-ld-options" ,(cond-expand (darwin "-lSystem") (else "-lc")))
      (gxc: "scheme/async-service" (extra-inputs: ("scheme/service-port-wake.ss")))
      ,program-metadata-target
      (gxc: "scheme/program-build" (extra-inputs: ("scheme/link-options.ss")))
      (gxc: "scheme/build" "-ld-options" ,(string-join (gerbil-scheme-rust-link-options) " "))
      "scheme/python-runtime"
      "scheme/python-package"))
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
