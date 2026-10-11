;;; -*- Gerbil -*-
;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Standalone SDK capability probe, not an FFI safety or throughput receipt.
(export main)

(def (checksum)
  (let loop ((remaining 100000) (total 0))
    (if (zero? remaining) total
        (loop (- remaining 1) (+ total remaining)))))

(def (main)
  (let ((original (##current-vm-processor-count))
        (requested (##cpu-count)))
    (unless (> requested 1)
      (error "SMP qualification requires a multi-processor host" requested))
    (dynamic-wind
      (lambda () (##cvmr requested))
      (lambda ()
        (let (active (##current-vm-processor-count))
          (displayln "SMP requested=" requested " active=" active)
          (unless (= active requested)
            (error "SDK did not activate the requested VM processors" requested active)))
        (let (workers (map (lambda (name) (spawn/name name checksum))
                          '(smp-left smp-right)))
          ;; Join every actor before restoring the standalone runtime setting.
          (let (results (map thread-join! workers))
            (unless (equal? results '(5000050000 5000050000))
              (error "native SMP actor checksum mismatch" results))))
        (displayln "SMP actor-checksums=OK"))
      (lambda () (##cvmr original)))))
