;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

(import :gerbil-scheme-rust/scheme/native)
(export main)

(def (main . _args)
  (displayln (gerbil-rs-add-i64 40 2)))
