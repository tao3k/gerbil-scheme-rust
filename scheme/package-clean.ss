;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Package-owned cleanup supplements std/make's declared SSI/static outputs.
(export clean-package-modules!)

(def (clean-package-modules! package prefix)
  (unless (symbol? package) (error "expected a declared package identifier" package))
  (let* ((name (symbol->string package))
         (parts (string-split name #\/)))
    (unless (and (not (string-empty? name))
                 (andmap (lambda (part) (not (member part '("" "." "..")))) parts))
      (error "package cleanup must stay inside its declared namespace" package))
    (remove-generated-tree! (path-expand (string-append "lib/" name) prefix))))

(def (remove-generated-tree! path)
  ;; lstat semantics: never traverse a generated namespace's symbolic links.
  (let (info (with-catch
              (lambda (failure)
                (if (no-such-file-or-directory-exception? failure) #f (raise failure)))
              (lambda () (file-info path #f))))
    (when info
      (case (file-info-type info)
        ((directory)
         (for-each (lambda (entry) (remove-generated-tree! (path-expand entry path)))
                   (filter (lambda (entry) (not (member entry '("." ".."))))
                     (directory-files (list path: path ignore-hidden: #f))))
         (delete-directory path))
        (else (delete-file path))))))
