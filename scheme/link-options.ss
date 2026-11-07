;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Included by the package build and compiler staging: one dependency declaration.
;;; Resolve only at build time, using the caller's target pkg-config environment.
(def (gerbil-scheme-rust-link-options)
  (filter (lambda (flag) (not (string-empty? flag)))
    ;; Preserve explicit system search paths for the archive's strict input
    ;; checks too; ordinary --libs may omit directories used by Linux targets.
    (string-split (pkg-config "openssl" "--libs" "--keep-system-libs" "zlib" "sqlite3") #\space)))
