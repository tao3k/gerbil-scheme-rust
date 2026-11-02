;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Native package operations. No Gerbil command-line process is launched here.
(import :gerbil/expander :std/ffi
        (only-in :gerbil/runtime/init gerbil-load-expander!)
        (rename-in (only-in :gerbil/tools/gxtest main) (main package-test-main))
        (only-in "runtime.ss" gerbil-rs-exception-message)
        (only-in "program-build.ss" gerbil-rs-stage-program))
(export main)
(C-ffi-macrology)
(C-declare "#include <stdint.h>")

(def (main . _) (gerbil-load-expander!))

;; One tooling VM binds one immutable package prefix. This is not an alternate
;; environment-control protocol: calls scope Gerbil's existing standard paths.
(def package-paths #f)
(def (restore-environment name value)
  (if value (setenv name value) (setenv name)))
(def (with-package-paths action)
  (if (not package-paths) (action)
    (let ((prefix (getenv "GERBIL_PATH" #f))
          (libraries (getenv "GERBIL_LOADPATH" #f))
          (paths (load-path)))
      (dynamic-wind
        (lambda ()
          (setenv "GERBIL_PATH" (car package-paths))
          (setenv "GERBIL_LOADPATH" (cdr package-paths))
          (set-load-path! (append (string-split (cdr package-paths) #\:) paths)))
        action
        (lambda ()
          (set-load-path! paths)
          (restore-environment "GERBIL_PATH" prefix)
          (restore-environment "GERBIL_LOADPATH" libraries))))))

(def (package-configure prefix libraries)
  (unless (and (string? prefix) (not (string-empty? prefix))
               (string? libraries) (not (string-empty? libraries)))
    (error "nonempty package prefix and library paths required"))
  (let (selected (cons (path-expand prefix) libraries))
    (when (and package-paths (not (equal? selected package-paths)))
      (error "tooling owner is already bound to another package prefix"))
    (set! package-paths selected)
    0))

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
        (let (result (with-build-cores (lambda () (with-package-paths action))))
          (force-output (current-output-port))
          (force-output (current-error-port))
          (if (and (exact-integer? result) (<= -2147483648 result 2147483647))
            result 0))))))

(def (package-main directory arguments)
  (call-package directory
    (lambda ()
      (load (path-expand "build.ss" (current-directory)))
      (apply (eval 'main) arguments))))
(def (package-build directory) (package-main directory ["compile"]))
(def (package-main-arguments directory arguments count)
  (call-package directory
    (lambda ()
      (load (path-expand "build.ss" (current-directory)))
      (apply (eval 'main)
        (map (lambda (index) (package-argument arguments index)) (iota count))))))

(def (package-stage directory source output)
  (call-package directory
    (lambda ()
      (create-directory* (path-expand output))
      (gerbil-rs-stage-program (path-expand source) (path-expand output))
      0)))

(extern package-argument)

(def (package-test directory tests)
  (call-package directory
    (lambda ()
      (unless (and (string? tests) (not (string-empty? tests)))
        (error "nonempty package test path required"))
      (package-test-main "-g" "-v" "5" "--" tests))))

(begin-foreign
  (namespace ("gerbil-scheme-rust/scheme/package-tools#" package-argument))
  (define package-argument
    (c-lambda ((pointer void) unsigned-int) UTF-8-string
      "___result = ((const char *const *)___arg1)[___arg2];"))
  (c-define (package-main-entry directory arguments count) (UTF-8-string (pointer void) unsigned-int) int32
    "gerbil_scheme_rust_package_main_scheme" ""
    (gerbil-scheme-rust/scheme/package-tools#package-main-arguments directory arguments count))
  (c-define (package-stage-entry directory source output) (UTF-8-string UTF-8-string UTF-8-string) int32
    "gerbil_scheme_rust_package_stage_scheme" ""
    (gerbil-scheme-rust/scheme/package-tools#package-stage directory source output))
  (c-define (package-paths-entry prefix libraries) (UTF-8-string UTF-8-string) int32
    "gerbil_scheme_rust_package_paths_scheme" ""
    (gerbil-scheme-rust/scheme/package-tools#call-package (current-directory)
      (lambda () (gerbil-scheme-rust/scheme/package-tools#package-configure prefix libraries))))
  (c-define (package-build-entry directory) (UTF-8-string) int32
    "gerbil_scheme_rust_package_build_scheme" ""
    (gerbil-scheme-rust/scheme/package-tools#package-build directory))
  (c-define (package-test-entry directory tests) (UTF-8-string UTF-8-string) int32
    "gerbil_scheme_rust_package_test_scheme" ""
    (gerbil-scheme-rust/scheme/package-tools#package-test directory tests)))
