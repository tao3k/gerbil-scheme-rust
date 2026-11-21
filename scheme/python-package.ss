;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Build-time only. The official expander owns parsing and phase dependencies.
(import :gerbil/expander :gerbil/runtime/loader
        (only-in :std/encoding/json write-json))
(export gerbil-rs-stage-load-closure)

(def (gerbil-rs-stage-load-closure entries output)
  (def contexts (make-hash-table-eq))
  (def artifacts (make-hash-table))
  (def (visit dependency)
    (cond
      ((module-context? dependency)
       (unless (hash-get contexts dependency)
         (hash-put! contexts dependency #t)
         (let (prelude (core-context-prelude dependency)) (when prelude (visit prelude)))
         (for-each visit (module-context-import dependency))
         (for-each visit (module-context-export dependency))))
      ((prelude-context? dependency)
       (for-each visit (prelude-context-import dependency)))
      ((module-import? dependency) (visit (module-import-source dependency)))
      ((module-export? dependency) (visit (module-export-context dependency)))
      ((import-set? dependency)
       (visit (import-set-source dependency))
       (for-each visit (import-set-imports dependency)))
      (else (error "unsupported official module dependency" dependency))))
  (for-each (lambda (entry) (visit (import-module entry))) entries)
  (def (add path)
    (unless (file-exists? path) (error "missing module closure resource" path))
    (hash-put! artifacts (path-normalize path) #t))
  (def (collect context)
    (let (path (module-context-path context))
      ;; Nested contexts share the containing module's metadata and artifacts.
      (when (string? path)
        (let* ((id (expander-context-id context))
               (metadata (if (equal? (path-extension path) ".ssi") path
                             (core-resolve-library-module-path (string->symbol (string-append ":" (symbol->string id))))))
               (base (path-strip-extension metadata))
               (directory (path-directory base))
               (stem (path-strip-directory base)))
          (unless (equal? (path-extension metadata) ".ssi")
            (error "module is not compiled; downstream compilation is forbidden" id metadata))
          (add metadata)
          ;; Native parts are compiler-named stem.oN and stem~phi.oN. Keep the
          ;; contiguous versions the public loader uses; never descend SDK trees.
          (for-each
            (lambda (name)
              (let* ((extension (path-extension name))
                     (part (path-strip-extension name))
                     (suffix (and (string-prefix? (string-append stem "~") part)
                                  (substring part (+ (string-length stem) 1) (string-length part)))))
                (when (and (string-prefix? ".o" extension)
                           (string->number (substring extension 2 (string-length extension)))
                           (or (equal? part stem) (and suffix (> (string-length suffix) 0))))
                  (add (path-expand name directory)))))
            (directory-files directory))))))
  (for-each collect (hash-keys contexts))
  (call-with-output-file output
    (lambda (port)
      (write-json port
        (hash ("schema" "gerbil-scheme-python.closure.v1")
              ("entries" (list->vector (map (lambda (e) (if (symbol? e) (symbol->string e) e)) entries)))
              ("files" (list->vector (list-sort string<? (hash-keys artifacts)))))))))
