;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Build-time only: preserve the compiler-owned runtime dependency graph.
(import :gerbil/compiler
        :gerbil/expander
        (only-in :std/list/list delete-duplicates/hash)
        (only-in :std/make pkg-config-libs)
        (only-in :std/encoding/json write-json))
(export gerbil-rs-stage-program)

(include "link-options.ss")

;; The versioned compiler adapter lives here, never in a downstream Rust
;; parser. These are the same native graph operations used by compile-exe.
(extern namespace: gxc
  find-runtime-module-deps find-static-module-file static-module-name gerbil-runtime-modules)

(def (system-context? context)
  (let (id (symbol->string (expander-context-id context)))
    (or (string-prefix? "gerbil/" id) (string-prefix? "std/" id))))

;; Namespace ordering and installed object availability are different facts.
;; A tools module may ship only .scm; compile that source through the ordinary
;; programme archive path instead of claiming a reusable installed AOT object.
(def (installed-aot? id scm)
  (and (or (string-prefix? "gerbil/" id) (string-prefix? "std/" id))
       (file-exists? (string-append (path-strip-extension scm) ".c"))
       (file-exists? (string-append (path-strip-extension scm) ".o"))))

(def (gerbil-rs-stage-program source output-dir (runtime-imports []) (separate-procedures []) link-options: (declared-link-options []))
  (let* ((output (path-expand "program" output-dir))
         (context (import-module source))
         (dependencies (find-runtime-module-deps context))
         (contexts
          (filter (lambda (ctx)
                    (not (string-prefix? "gerbil/core" (symbol->string (expander-context-id ctx)))))
                  (append dependencies (list context))))
         (ids (delete-duplicates/hash
               (append gerbil-runtime-modules
                       ;; Preserve compile-exe's system-before-user link order.
                       (map (lambda (ctx) (symbol->string (expander-context-id ctx)))
                            (append (filter system-context? contexts)
                                    (filter (lambda (ctx) (not (system-context? ctx)))
                                            contexts)))))))
    (unless (and (list? separate-procedures)
                 (andmap (lambda (id) (and (string? id) (member id ids))) separate-procedures))
      (error "procedure layout requires compiler-known module identities" separate-procedures))
    ;; The entry module's main must return normally without entering an event
    ;; loop. Its ordinary exports become callable after shared runtime setup.
    (compile-exe source [invoke-gsc: #f keep-scm: #t optimize: #t output-file: output])
    (let* ((home (gerbil-home))
           ;; All staging clients receive the same owner-declared target paths.
           ;; Installed Gerbil metadata remains required, but its historical -L
           ;; directories cannot hide the current pkg-config declaration.
           (link-options (append (gerbil-scheme-rust-link-options) declared-link-options
              (call-with-input-file (path-expand "lib/libgerbil.ldd" home) read)))
           (modules
            (map (lambda (id)
                   (let* ((scm (find-static-module-file id))
                         (system? (and (installed-aot? id scm) #t)))
                     (let (module (hash ("module" id) ("scm" scm) ("system" system?)))
                       (when (member id separate-procedures)
                         (hash-put! module "host_layout" "separate-procedures"))
                       module)))
                 ids)))
      (when (pair? runtime-imports)
        (let* ((stub (string-append output "__exe.scm"))
               (builtin (program-builtins stub))
               (base (delete-duplicates/hash (append builtin ids)))
               (metadata (program-metadata-modules runtime-imports output-dir base)))
          (set! modules (append modules metadata))
          (program-register-builtins! stub base metadata)))
      (call-with-output-file (path-expand "program.json" output-dir)
        (lambda (port)
          (write-json
           port
           (hash ("schema" "gerbil-scheme-rust.aot-program.v1")
                 ("modules" (list->vector modules))
                 ("stub" (string-append output "__exe.scm"))
                 ("library_dir" (path-expand "lib" home))
                 ("link_options" (list->vector link-options)))))))))


;; Opt-in packaging of existing library metadata; user DSL remains interpreted.
(def (program-builtins stub)
  (call-with-input-file stub
    (lambda (port)
      (match (read port)
        (['define 'builtin-modules ['quote modules]] modules)
        (else (error "unsupported compiler entry stub" stub))))))

(def (program-register-builtins! stub modules metadata)
  (let (forms (call-with-input-file stub
                 (lambda (port)
                   (read port)
                   (let loop ((forms []))
                     (let (form (read port))
                       (if (eof-object? form) (reverse forms)
                         (loop (cons form forms))))))))
    (call-with-output-file stub
      (lambda (port)
        (write `(define builtin-modules ',modules) port)
        (newline port)
        ;; Keep the SDK loader's state/error/load-order handling. Only replace
        ;; its raw carrier read while an exact linked library module is loading.
        ;; Gambit registers these descriptors with preload disabled; their
        ;; top-level expressions execute in the original expander context.
        (write
          `(let ((original-module load-module)
                 (original-load load)
                 (active (make-parameter #f))
                 (linked ',(map (lambda (module)
                                 (cons (hash-ref module "module")
                                       (string->symbol
                                         (path-strip-directory
                                           (path-strip-extension (hash-ref module "scm"))))))
                               ;; Empty runtime reexports have no Gambit
                               ;; descriptor; retain the SDK's empty-file load.
                               (filter (lambda (module)
                                         (> (file-info-size (file-info (hash-ref module "scm"))) 0))
                                       metadata))))
             (set! load-module
               (lambda (module)
                 (let ((entry (assoc module linked)))
                   (if entry
                     (parameterize ((active (cdr entry))) (original-module module))
                     (parameterize ((active #f)) (original-module module))))))
             (set! load
               (lambda (path . options)
                 (if (and (active) (null? options))
                   (let ((module (active)))
                     (parameterize ((active #f)) (##load-module module))
                     path)
                   (apply original-load path options)))))
          port)
        (newline port)
        (for-each (lambda (form) (write form port) (newline port)) forms)))))

(def (program-metadata-modules imports output-dir builtin)
  (let* ((observation (path-expand "metadata-modules.dat" output-dir))
         (expression
           (call-with-output-string
             (lambda (port)
               (write '(import :gerbil-scheme-rust/scheme/program-metadata) port)
               (newline port)
               (write `(gerbil-rs-observe-program-metadata ,observation ',imports) port))))
         (child (open-process
                  [path: (path-expand "bin/gxi" (gerbil-home))
                   arguments: ["-:debug=q" "-e" expression]
                   stdin-redirection: #f stdout-redirection: #f stderr-redirection: #f]))
         (status (process-status child)))
    (close-port child)
    (unless (= status 0) (error "compiler metadata observation failed" status))
    ;; Additional library runtime dependencies are lazy too. In particular,
    ;; compiler/base constructs classes and cannot precede runtime/MOP setup.
    ;; Never merge these contexts into compile-exe's eager bootstrap order.
    (let ((rows (append (call-with-input-file observation read)
                  (filter-map
                    (lambda (ctx)
                      (let (id (symbol->string (expander-context-id ctx)))
                        (and (not (string-prefix? "gerbil/core" id))
                             (list id (find-static-module-file id)))))
                    (apply append (map (lambda (ctx)
                                         (append (find-runtime-module-deps ctx) [ctx]))
                                       (map import-module imports))))))
          (seen (list->hash-table (map (lambda (id) (cons id #t))
                    (append builtin (map (cut string-append <> "~0") builtin)))))
          (directory (path-expand "metadata" output-dir)))
      (create-directory* directory)
      (filter-map
        (lambda (row)
          (match row
            ([original source]
             (let* ((parent (and (string-suffix? "~0" original)
                                (substring original 0 (- (string-length original) 2))))
                    (static-parent (and parent
                                     (with-catch (lambda (_) #f)
                                       (lambda () (find-static-module-file parent)))))
                    (id (if (and static-parent (file-exists? static-parent)) parent original))
                    (static (or (and (equal? id parent) static-parent)
                                (with-catch (lambda (_) #f)
                                  (lambda () (find-static-module-file id))))))
               (and (not (hash-get seen id))
                 (let* ((installed? (and static (file-exists? static)))
                        (scm (if installed? static
                               (path-expand (string-append (static-module-name (string->symbol id)) ".scm")
                                            directory)))
                        (system? (and installed? (installed-aot? id scm))))
                   (unless installed?
                     (when (file-exists? scm) (delete-file scm))
                     (copy-file source scm))
                   (hash-put! seen id #t)
                   (hash-put! seen (string-append id "~0") #t)
                   (hash ("module" id) ("scm" scm) ("system" (and system? #t)) ("deferred" #t))))))
            (else (error "invalid compiler metadata observation" row))))
        rows))))
