;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Build-time only: preserve the compiler-owned runtime dependency graph.
(import :gerbil/compiler
        :gerbil/expander
        :std/srfi/1
        :std/srfi/13
        :std/text/json)
(export gerbil-rs-stage-program)

;; The versioned compiler adapter lives here, never in a downstream Rust
;; parser. These are the same native graph operations used by compile-exe.
(extern namespace: gxc
  find-runtime-module-deps find-static-module-file gerbil-runtime-modules)

(def (system-context? context)
  (let (id (symbol->string (expander-context-id context)))
    (or (string-prefix? "gerbil/" id) (string-prefix? "std/" id))))

(def (gerbil-rs-stage-program source output-dir)
  (let* ((output (path-expand "program" output-dir))
         (context (import-module source))
         (dependencies (find-runtime-module-deps context))
         (contexts
          (filter (lambda (ctx)
                    (not (string-prefix? "gerbil/core" (symbol->string (expander-context-id ctx)))))
                  (append dependencies (list context))))
         (ids (delete-duplicates
               (append gerbil-runtime-modules
                       ;; Preserve compile-exe's system-before-user link order.
                       (map (lambda (ctx) (symbol->string (expander-context-id ctx)))
                            (append (filter system-context? contexts)
                                    (filter (lambda (ctx) (not (system-context? ctx))) contexts))))
               string=?)))
    ;; The entry module's main must return normally without entering an event
    ;; loop. Its ordinary exports become callable after shared runtime setup.
    (compile-exe source [invoke-gsc: #f keep-scm: #t optimize: #t output-file: output])
    (let* ((home (gerbil-home))
           (link-options (call-with-input-file (path-expand "lib/libgerbil.ldd" home) read))
           (modules
            (map (lambda (id)
                   (let ((scm (find-static-module-file id))
                         (system? (or (string-prefix? "gerbil/" id) (string-prefix? "std/" id))))
                     (hash ("module" id) ("scm" scm) ("system" system?))))
                 ids)))
      (call-with-output-file (path-expand "program.json" output-dir)
        (lambda (port)
          (write-json
           (hash ("schema" "gerbil-scheme-rust.aot-program.v1")
                 ("modules" (list->vector modules))
                 ("stub" (string-append output "__exe.scm"))
                 ("library_dir" (path-expand "lib" home))
                 ("link_options" (list->vector link-options)))
           port))))))
