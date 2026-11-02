;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Native package operations. No Gerbil command-line process is launched here.
(import :gerbil/expander :std/ffi
        (only-in :gerbil/runtime/init gerbil-load-expander!)
        (rename-in (only-in :gerbil/tools/gxtest main) (main package-test-main))
        (only-in "runtime.ss" gerbil-rs-exception-message)
        (only-in "program-build.ss" gerbil-rs-stage-program)
        (only-in :std/source this-source-file)
        (only-in :gerbil/tools/env setup-local-pkg-env!))
(export main)
(extern namespace: gerbil/tools/gxtest collect-files)
(extern namespace: gerbil-scheme-rust/scheme/runtime gerbil-rs-observe-module-work)
(C-ffi-macrology)
(C-declare "#include <stdint.h>")

(def (main . _) (gerbil-load-expander!))

;; One tooling VM binds one immutable package prefix. This is not an alternate
;; environment-control protocol: calls scope Gerbil's existing standard paths.
(def package-paths #f)
(def (restore-environment name value)
  (if value (setenv name value) (setenv name)))
(def (with-package-paths action)
    (let ((prefix (getenv "GERBIL_PATH" #f))
          (libraries (getenv "GERBIL_LOADPATH" #f))
          (paths (load-path)))
      (dynamic-wind
        (lambda ()
          (when package-paths
            (setenv "GERBIL_PATH" (car package-paths))
            (setenv "GERBIL_LOADPATH" (cdr package-paths))
            (set-load-path! (append (string-split (cdr package-paths) #\:) paths))))
        action
        (lambda ()
          (set-load-path! paths)
          (restore-environment "GERBIL_PATH" prefix)
          (restore-environment "GERBIL_LOADPATH" libraries)))))

(def (package-configure prefix libraries)
  (unless (and (string? prefix) (not (string-empty? prefix))
               (string? libraries) (not (string-empty? libraries)))
    (error "nonempty package prefix and library paths required"))
  (let (selected (cons (path-expand prefix) libraries))
    (when (and package-paths (not (equal? selected package-paths)))
      (error "tooling owner is already bound to another package prefix"))
    (set! package-paths selected)
    0))

;; Match the native package command before expanding the package build script.
;; The existing immutable owner binding retains the native selection across calls;
;; with-package-paths restores the caller's environment even on the first failure.
(def (setup-package-env! create?)
  (unless package-paths
    (setup-local-pkg-env! create?)
    (let (prefix (getenv "GERBIL_PATH" #f))
      (when prefix
        (let* ((library (path-expand "lib" prefix))
               (extra (getenv "GERBIL_LOADPATH" #f))
               (libraries (if (and extra (not (string-empty? extra)))
                            (string-append library ":" extra) library)))
          (package-configure prefix libraries)
          (add-load-path! library))))))

(def (with-build-cores action)
  (let ((previous (getenv "GERBIL_BUILD_CORES" #f))
        (options (getenv "GAMBOPT" #f)))
    (dynamic-wind
      (lambda ()
        (setenv "GERBIL_BUILD_CORES" (number->string (##cpu-count)))
        ;; Child gsc has its own setup parameters. Preserve the paired
        ;; installation through Gambit's official relocation options too.
        (setenv "GAMBOPT" (string-append
          "~~bin=" (path-expand "~~bin")
          ",~~lib=" (path-expand "~~lib")
          ",~~include=" (path-expand "~~include")
          (if options (string-append "," options) ""))))
      action
      (lambda ()
        (restore-environment "GAMBOPT" options)
        (restore-environment "GERBIL_BUILD_CORES" previous)))))

(def (call-package directory action)
  (with-catch
    (lambda (failure)
      (display (gerbil-rs-exception-message failure) (current-error-port))
      (newline (current-error-port))
      (force-output (current-error-port))
      70)
    (lambda ()
      (unless (and (string? directory) (not (string-empty? directory)))
        (error "nonempty package directory required"))
      (parameterize ((current-directory (path-expand directory))
                     (current-expander-context (make-top-context)))
        (eval-syntax '(import :gerbil/core))
        (eval-syntax '(define-alias ##begin begin))
        (let (result (with-build-cores
                        (lambda () (with-package-paths
                          (lambda () (gerbil-rs-observe-module-work action))))))
          (force-output (current-output-port))
          (force-output (current-error-port))
          (if (and (exact-integer? result) (<= -2147483648 result 2147483647))
            result 0))))))

(def (package-invoke-main arguments)
  (when (and (pair? arguments) (equal? (car arguments) "clean") (not package-paths))
    (error "package cleanup requires explicitly bound package paths"))
  (setup-package-env! #t)
  (load (path-expand "build.ss" (current-directory)))
  (apply (eval 'main) arguments))
(def (package-main directory arguments)
  (call-package directory (lambda () (package-invoke-main arguments))))
(def (package-build directory) (package-main directory ["compile"]))
(def (package-clean directory) (package-main directory ["clean"]))
(def (package-main-arguments directory arguments count)
  (call-package directory
    (lambda ()
      (package-invoke-main
        (map (lambda (index) (package-argument arguments index)) (iota count))))))

(extern package-argument)

(def (with-test-progress action)
  (def (report stage value)
    (displayln "PACKAGE-TOOLS " stage " " value)
    (force-output))
  (let ((importer (current-expander-module-import))
        (evaluator (current-expander-module-eval)))
    ;; Native resolver observation belongs to call-package. Source expansion
    ;; uses the expander's own parameters; do not rebind optimized loader globals.
    (parameterize
      ((current-expander-module-import
         (lambda (path reload?)
           (report "import entered" path)
           (let (context (importer path reload?))
             (report "import returned" path)
             context)))
       (current-expander-module-eval
         (lambda (context)
           (report "evaluation entered" (expander-context-id context))
           (let (result (evaluator context))
             (report "evaluation returned" (expander-context-id context))
             result))))
      (action))))

(def (package-test directory tests)
  (with-test-progress
    (lambda ()
      (call-package directory
        (lambda ()
          (unless (and (string? tests) (not (string-empty? tests)))
            (error "nonempty package test path required"))
          (setup-package-env! #f)
          ;; Evaluate selected entries once for this request; dependencies remain cached.
          (for-each (lambda (file) (import-module file #t #t))
                    (collect-files [tests]))
          (package-test-main "-g" "-v" "5" "--" tests))))))

(def (package-build-spec directory specification)
  (call-package directory
    (lambda ()
      (setup-package-env! #t)
      (load (path-expand "build.ss" (current-directory)))
      (eval-syntax '(import (only-in :std/make make)))
      ((eval 'make) ((eval (string->symbol specification)))
        srcdir: (current-directory)
        libdir: (path-expand "lib" (gerbil-path)) optimize: #t))))

;; The imports arrive through the runtime's existing rooted Scheme object ABI.
;; No expression parser, argument protocol, or downstream compiler plan is used.
(def (package-stage source output imports)
  (call-package (current-directory)
    (lambda ()
      (unless (and (list? imports) (andmap symbol? imports))
        (error "native runtime imports must be a rooted symbol list"))
      (let* ((entry (path-expand source))
             (destination (path-expand output))
             (integration-root (path-expand ".." (path-directory (this-source-file)))))
        ;; Staging owns the shared package link declaration on every entry path.
        (eval-syntax `(import (only-in ,(path-expand "scheme/program-build.ss" integration-root)
                               gerbil-rs-stage-program)))
        (create-directory* destination)
        ((eval 'gerbil-rs-stage-program) entry destination imports)
        ;; write-json returns a byte count; that is data, not a tool exit code.
        (void)))))

(def (package-load-closure imports output)
  (call-package (current-directory)
    (lambda ()
      (unless (and (list? imports) (andmap symbol? imports))
        (error "native runtime imports must be a rooted symbol list"))
      (let (integration-root (path-expand ".." (path-directory (this-source-file))))
        (eval-syntax `(import (only-in ,(path-expand "scheme/python-package.ss" integration-root)
                               gerbil-rs-stage-load-closure)))
        ((eval 'gerbil-rs-stage-load-closure) imports (path-expand output))
        (void)))))

(begin-foreign
  (namespace ("gerbil-scheme-rust/scheme/build#" package-argument))
  (define package-argument
    (c-lambda ((pointer void) unsigned-int) UTF-8-string
      "___result = ((const char *const *)___arg1)[___arg2];"))
)
