;;; -*- Gerbil -*-
;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Standalone SDK capability probe, not an FFI safety or throughput receipt.
(export main)

(def (checksum)
  (let loop ((remaining 100000) (total 0))
    (if (zero? remaining) total
        (loop (- remaining 1) (+ total remaining)))))

(def (main)
  (let (original (##get-parallelism-level))
    (dynamic-wind
      (lambda () (##set-parallelism-level! 2))
      (lambda ()
        (unless (= (##get-parallelism-level) 2)
          (error "SDK did not admit two native processors"))
        (displayln "SMP processors=2")
        (let (workers (map (lambda (name) (spawn/name name checksum))
                          '(smp-left smp-right)))
          ;; Join every actor before restoring the standalone runtime setting.
          (let (results (map thread-join! workers))
            (unless (equal? results '(5000050000 5000050000))
              (error "native SMP actor checksum mismatch" results))))
        (displayln "SMP actor-checksums=OK"))
      (lambda () (##set-parallelism-level! original)))))
