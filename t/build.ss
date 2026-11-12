#!/usr/bin/env gxi
;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
(import :std/build-script)
;; Compiled diagnostic loops avoid attributing interpreter frames to FFI.
(defbuild-script '("utf8-cost") optimize: #t parallelize: 1)
