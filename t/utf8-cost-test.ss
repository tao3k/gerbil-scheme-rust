;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
(import :gerbil-scheme-rust/t/utf8-cost)
;; gxi invokes main once; never also invoke it explicitly at top level.
(def (main . phases)
  (if (null? phases)
    (run-cost-matrix)
    (run-cost-matrix (map string->symbol phases))))
