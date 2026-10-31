;;; Use the installed Gerbil compiler; GERBIL_PATH must name the output prefix.
(import :gerbil/compiler)
(export main)

(def (main source output-prefix)
  (let* ((prefix (path-expand output-prefix))
         (options [output-dir: (path-expand "lib" prefix)
                   invoke-gsc: #t optimize: #t static: #t
                   gsc-options: ["-cc-options" "-O2"
                                 "-ld-options" "-Wl,-undefined,dynamic_lookup"]]))
    (compile-module source options)
    (compile-exe source
      (append [output-file: (path-expand "actor-ring" prefix)] options))))
