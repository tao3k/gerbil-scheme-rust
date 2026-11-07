;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;; Diagnostic phase controls, never parser or bridge latency admission.
(import :gerbil-scheme-rust/scheme/utf8)
(export run-cost-matrix)
(extern namespace: #f
  gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-chunk-c)

(def texts
  (list->vector
   (map (lambda (fragment) (apply string-append (make-list 683 fragment)))
        '("a\x00;汉字😀" "b\x00;漢語🚀" "c\x00;中文🌍" "d\x00;文字🎉"))))
(def expected (vector-map string->utf8 texts))
(def capacity (* 4 (string-length (vector-ref texts 0))))
(def output (make-u8vector capacity))

(def (leaf text (empty? #f))
  (let ((length (string-length text)))
    (let loop ((start 0) (written 0))
      (if (< start length)
        (let* ((end (min length (+ start 256)))
               (next (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-chunk-c
                      text output (if empty? end start) end written)))
          (when (< next 0) (error "diagnostic leaf failed"))
          (loop end next))
        written))))

(def (batch mode jobs)
  (let loop ((index 0) (total 0))
    (if (< index jobs)
      (let* ((slot (modulo index 4))
             (text (vector-ref texts slot))
             (size
              (case mode
                ((control) (u8vector-length (vector-ref expected slot)))
                ((allocate allocate-unfilled)
                 (let ((bytes (if (eq? mode 'allocate)
                                (make-u8vector capacity)
                                (##make-u8vector capacity))))
                   (##u8vector-shrink! bytes 8196)
                   (u8vector-length bytes)))
                ((leaf) (leaf text))
                ((dispatch) (leaf text #t))
                ((encode) (u8vector-length (gerbil-rs-encode-utf8 text)))
                (else (error "unknown diagnostic phase" mode)))))
        (unless (= size (if (eq? mode 'dispatch) 0 8196))
          (error "diagnostic output size mismatch"))
        (loop (+ index 1) (+ total size)))
      (unless (= total (if (eq? mode 'dispatch) 0 (* jobs 8196)))
        (error "diagnostic batch mismatch")))))

;; Official ##exec-stats subtracts its own statistics object allocation.
;; Compiled driver/control overhead remains visible; these are not isolated
;; production C-call costs and no medians may be subtracted across runs.
(def (run-cost-matrix (modes '(control allocate allocate-unfilled dispatch leaf encode)))
 (for-each
 (lambda (jobs)
   (for-each
    (lambda (mode)
      (let loop ((sample 1) (receipts '()))
        (if (<= sample 20)
          (let ((stats (##exec-stats (lambda () (batch mode jobs)))))
            (when (eq? mode 'leaf)
              (unless (equal? (subu8vector output 0 8196) (vector-ref expected 3))
                (error "diagnostic leaf content mismatch")))
            (loop (+ sample 1) (cons stats receipts)))
          (begin
            (unless (= (length receipts) 20) (error "missing diagnostic samples"))
            (displayln "UTF8-COST jobs=" jobs " mode=" mode " samples=20 totals="
             (map (lambda (key)
                    (cons key (apply + (map (lambda (stats) (cdr (assq key stats))) receipts))))
                  '(user-time sys-time real-time gc-user-time gc-sys-time gc-real-time nb-gcs)))
            (let ((allocated (map (lambda (stats) (cdr (assq 'bytes-allocated stats))) receipts)))
              (displayln "UTF8-ALLOCATION jobs=" jobs " mode=" mode
                         " min=" (apply min allocated) " max=" (apply max allocated)
                         " negative-samples=" (length (filter negative? allocated))))))))
    modes))
 '(1000 10000 100000)))
