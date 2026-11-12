;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Linked by a Python Library's ordinary compiler-owned module graph.
(import (only-in :gerbil/runtime/loader set-load-path!)
        :gerbil-scheme-rust/scheme/runtime)
(export gerbil-rs-python-configure! gerbil-rs-python-apply)

(def (gerbil-rs-python-configure! paths)
  (unless (and (list? paths) (andmap string? paths))
    (error "package loader roots must be a list of strings"))
  ;; This public runtime operation preserves Gerbil's normal module semantics.
  ;; No compiler process, raw .scm bootstrap or private loader registry is used.
  (set-load-path! paths)
  (void))

(def (gerbil-rs-python-apply request)
  (apply (vector-ref request 0) (vector-ref request 1)))

(begin-foreign
  (c-define (configure paths) (int64) int64 "gerbil_scheme_python_configure" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call paths
      gerbil-scheme-rust/scheme/python-runtime#gerbil-rs-python-configure!))
  (c-define (invoke request) (int64) int64 "gerbil_scheme_python_apply" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call request
      gerbil-scheme-rust/scheme/python-runtime#gerbil-rs-python-apply)))
