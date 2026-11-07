#!/usr/bin/env gxi
(import :std/build-script)
(defbuild-script '("scheme/plugin" "scheme/ledger") optimize: #t parallelize: 1)
