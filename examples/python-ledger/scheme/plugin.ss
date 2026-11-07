;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Runtime-loaded DSL component with an ordinary hygienic macro.
(export ledger-sum)
(defrule (ledger-sum item ...) (foldl + 0 (list item ...)))
