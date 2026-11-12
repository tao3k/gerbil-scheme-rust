;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Build-time inspection in a fresh SDK process, independent of compiler caches.
(import :gerbil/runtime/loader)
(export gerbil-rs-observe-program-metadata)

(def (gerbil-rs-observe-program-metadata output imports)
  (unless (and (string? output) (list? imports) (andmap symbol? imports))
    (error "invalid program metadata observation"))
  (eval `(import ,@imports))
  ;; Use the official loader's identities and dependency order. No Scheme
  ;; dependency parser or VM objects cross this build-time boundary.
  (let (modules
        (list-sort (lambda (a b) (< (module-load-order (car a))
                                    (module-load-order (car b))))
          (filter (lambda (entry)
                    (and (string? (cdr entry))
                         (not (member (car entry)
                           '("gerbil-scheme-rust/scheme/program-metadata"
                             "gerbil-scheme-rust/scheme/program-metadata~0")))))
                  (list-modules))))
    (call-with-output-file output
      (lambda (port)
        (write (map (lambda (entry)
                      (let (source (string-append (path-strip-extension (cdr entry)) ".scm"))
                        (unless (file-exists? source)
                          (error "missing compiler metadata carrier" (car entry) source))
                        (list (car entry) source))) modules) port)))))
