;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
prelude: "./metadata-template-prelude"
(import :gerbil-scheme-rust/scheme/runtime (for-template :std/func (runtime: :std/iter)))
(export main)
(def (main . _) (void))
