;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
(import :std/test :std/io/tempfile :std/encoding/json
        (only-in :std/error Error?)
        (only-in :std/make pkg-config pkg-config-libs)
        (only-in :std/source this-source-file)
        (only-in :gerbil/runtime/loader add-load-path!)
        :gerbil/compiler
        :gerbil-scheme-rust/scheme/program-build)
(export link-options-test)

(def (stage-link-options directory explicit)
  (let* ((source (path-expand "metadata-entry.ss" (path-directory (this-source-file))))
         (library (path-expand "lib" directory)))
    (create-directory* library)
    (add-load-path! library)
    (compile-module source [output-dir: library invoke-gsc: #f optimize: #t static: #t])
    (gerbil-rs-stage-program source directory gsc-options: ["-ld-options" (string-join explicit " ")])
    (parameterize ((current-json-read-options (make-JSONReadOptions object-as-hash: #t)))
      (let (flags (hash-ref (call-with-input-file (path-expand "program.json" directory) read-json)
                           "link_options"))
        (if (vector? flags) (vector->list flags) flags)))))

(def link-options-test
  (test-suite "Owner-declared native dependencies reach low-level staging"
    (test-case "ordinary staging carries current target package flags first"
      (call-with-temporary-directory "program-link-options"
        (lambda (directory)
          (let ((caller-policy (getenv "PKG_CONFIG_ALLOW_SYSTEM_LIBS" #f))
                (expected (filter (lambda (flag) (not (string-empty? flag)))
                            (string-split (pkg-config-libs "openssl" "zlib" "sqlite3") #\space)))
                (actual (stage-link-options directory [])))
            (check-equal? (getenv "PKG_CONFIG_ALLOW_SYSTEM_LIBS" #f) caller-policy)
            (check-equal? (take actual (length expected)) expected)
            (for-each (lambda (flag) (check (member flag actual) ? pair?))
                      '("-lcrypto" "-lssl" "-lz" "-lsqlite3"))))))
    (test-case "explicit consumer flags retain their order and duplicates"
      (call-with-temporary-directory "program-link-order"
        (lambda (directory)
          (let* ((explicit '("-L/consumer-target" "-lconsumer" "-lconsumer"))
                 (expected (filter (lambda (flag) (not (string-empty? flag)))
                             (string-split (pkg-config-libs "openssl" "zlib" "sqlite3") #\space)))
                 (actual (stage-link-options directory explicit)))
            (check-equal? (take (drop actual (length expected)) (length explicit)) explicit)))))
    (test-case "system-directory classification follows standard Make flags"
      (call-with-temporary-directory "program-link-system-paths"
        (lambda (directory)
          (let ((previous (getenv "PKG_CONFIG_SYSTEM_LIBRARY_PATH" #f))
                (target (pkg-config "libcrypto" "--variable=libdir")))
            (check (string-empty? target) => #f)
            (dynamic-wind
              (lambda () (setenv "PKG_CONFIG_SYSTEM_LIBRARY_PATH" target))
              (lambda ()
                (let ((expected (filter (lambda (flag) (not (string-empty? flag)))
                                  (string-split (pkg-config-libs "openssl" "zlib" "sqlite3") #\space)))
                      (actual (stage-link-options directory [])))
                  (check-equal? (take actual (length expected)) expected)
                  (for-each (lambda (flag) (check (member flag actual) ? pair?))
                            '("-lcrypto" "-lssl" "-lz" "-lsqlite3"))))
              (lambda ()
                (if previous (setenv "PKG_CONFIG_SYSTEM_LIBRARY_PATH" previous)
                    (setenv "PKG_CONFIG_SYSTEM_LIBRARY_PATH"))))))))
    (test-case "missing target package metadata cannot fall back to installed Gerbil paths"
      (call-with-temporary-directory "program-link-missing"
        (lambda (directory)
          (let ((path (getenv "PKG_CONFIG_PATH" #f))
                (libdir (getenv "PKG_CONFIG_LIBDIR" #f)))
            (dynamic-wind
              (lambda ()
                (setenv "PKG_CONFIG_PATH" "")
                (setenv "PKG_CONFIG_LIBDIR" directory))
              (lambda () (check-exception (stage-link-options directory []) Error?))
              (lambda ()
                (if path (setenv "PKG_CONFIG_PATH" path) (setenv "PKG_CONFIG_PATH"))
                (if libdir (setenv "PKG_CONFIG_LIBDIR" libdir) (setenv "PKG_CONFIG_LIBDIR"))))))))))
