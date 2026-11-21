package: gerbil-scheme-rust/qualification
(export mailbox-failure-reset! mailbox-failure-seal! mailbox-failure-take! mailbox-failure-detach! mailbox-failure-contract!)
(extern mailbox-histogram-summarize!)
;;; Cold terminal receipt. No publisher/handler/consumer hot-path hook.
;;; Command identity fences single-use access; the private owner takes/releases
;;; the snapshot before the original command and borrowed progress word retire.
(def mailbox-failure-slot #f)
(def (mailbox-failure-reset!) (set! mailbox-failure-slot #f))
(def (mailbox-failure-seal! command phase now start rate total published consumed active
                           histogram source-peak total-peak max-lag last-publication over-window)
  ;; Failure capture must never replace the original business exception. These
  ;; arguments are values at the failing branch, not newly captured source boxes.
  (with-catch void
    (lambda ()
      (set! mailbox-failure-slot
        (vector command phase now start rate total published consumed active
          (u64vector-copy histogram) source-peak total-peak max-lag last-publication over-window)))))
;; Constant-size metadata projection. The frozen histogram is transferred as a
;; typed Scheme object; neither this path nor detach scans its bucket storage.
(def (mailbox-failure-detach! command)
  (let (snapshot mailbox-failure-slot)
    (if (not (and snapshot (eq? command (vector-ref snapshot 0)))) #f
      (begin
        (set! mailbox-failure-slot #f)
        (let* ((now (vector-ref snapshot 2)) (start (vector-ref snapshot 3))
               (rate (vector-ref snapshot 4)) (total (vector-ref snapshot 5))
               (published (vector-ref snapshot 6)) (consumed (vector-ref snapshot 7))
               (active (vector-ref snapshot 8)) (elapsed (- now start))
               (due (min total (1+ (quotient (* elapsed rate) 1000000000)))))
          (unless (and (<= 0 consumed published due total) (= active (- published consumed)))
            (error "invalid terminal source accounting"))
          (vector (symbol->string (vector-ref snapshot 1))
            (s64vector total rate elapsed due published consumed active -1 -1 -1
              (max (vector-ref snapshot 10) (- due published))
              (max (vector-ref snapshot 11) (- due consumed))
              (- due published) (- published consumed)
              (vector-ref snapshot 12) (vector-ref snapshot 13) (vector-ref snapshot 14))
            (vector-ref snapshot 9)))))))
;; No allocation, callback, lock or Scheme-object exposure inside this C call.
;; Reduce the frozen diagnostic histogram only; no handler uses this operation.
(begin-foreign
 (namespace ("gerbil-scheme-rust/qualification/mailbox-failure#" mailbox-histogram-summarize!))
 (define mailbox-histogram-summarize!
  (c-lambda (scheme-object scheme-object) int
   #<<C-END
___SCMOBJ ___temp;
if (!___U64VECTORP(___arg1) || !___S64VECTORP(___arg2) || ___S64VECTORSIZE(___arg2) != 3) ___return(-1);
const ___U64 n = ___U64VECTORSIZE(___arg1), maximum = 0x7fffffffffffffffULL;
const ___U64 *bins = ___CAST(const ___U64*, ___BODY_AS(___arg1, ___tU64VECTOR));
___U64 count = 0;
for (___U64 i = 0; i < n; ++i) {
 if (bins[i] > maximum - count) ___return(-2);
 count += bins[i];
}
___S64 p99 = -1, p999 = -1;
if (count) {
 const ___U64 r99 = (count / 100) * 99 + ((count % 100) * 99 + 99) / 100;
 const ___U64 r999 = (count / 1000) * 999 + ((count % 1000) * 999 + 999) / 1000;
 ___U64 sum = 0;
 for (___U64 i = 0; i < n; ++i) {
  sum += bins[i];
  if (sum >= r99 && p99 < 0) {
   if (i >= maximum / 1000) ___return(-3);
   p99 = (___S64)((i + 1) * 1000);
  }
  if (sum >= r999) {
   if (i >= maximum / 1000) ___return(-3);
   p999 = (___S64)((i + 1) * 1000); break;
  }
 }
}
___STORE_S64(___BODY_AS(___arg2, ___tS64VECTOR), 0, (___S64)count);
___STORE_S64(___BODY_AS(___arg2, ___tS64VECTOR), 1, p99);
___STORE_S64(___BODY_AS(___arg2, ___tS64VECTOR), 2, p999);
___return(0);
C-END
  )))
(def (mailbox-failure-fast-take! command)
  (let (report (mailbox-failure-detach! command))
    (and report
      (let ((core (vector-ref report 1)) (summary (make-s64vector 3 -1)))
        (unless (zero? (mailbox-histogram-summarize! (vector-ref report 2) summary))
          (error "invalid terminal histogram summary"))
        (s64vector-set! core 7 (s64vector-ref summary 0))
        (s64vector-set! core 8 (s64vector-ref summary 1))
        (s64vector-set! core 9 (s64vector-ref summary 2))
        (vector (vector-ref report 0) core)))))

;; Retained original Scheme oracle, used by correctness fixtures only. The
;; diagnostic FFI entry below deliberately calls detach, never this scan.
(def (mailbox-failure-take! command percentile)
  (let (report (mailbox-failure-detach! command))
    (and report
      (let* ((core (vector-ref report 1)) (histogram (vector-ref report 2))
             (samples (let loop ((i 0) (sum 0))
                        (if (= i (u64vector-length histogram)) sum
                          (loop (1+ i) (+ sum (u64vector-ref histogram i)))))))
        (s64vector-set! core 7 samples)
        (s64vector-set! core 8 (percentile histogram samples 99 100))
        (s64vector-set! core 9 (percentile histogram samples 999 1000))
        (vector (vector-ref report 0) core)))))

;; Deterministic cold-entry contract, used only by the isolated image fixture.
(def (mailbox-failure-contract! command percentile)
  (let ((original (vector 'original-error)) (histogram (u64vector 0 0 1 0)))
    (mailbox-failure-reset!)
    (with-catch
      (lambda (caught) (unless (eq? original caught) (error "terminal capture replaced exception")))
      (lambda ()
        (mailbox-failure-seal! command 'contract 1000002000 1000000000
          500000 4 1 1 0 histogram 0 0 0 0 0)
        (u64vector-set! histogram 2 19) (##gc) (raise original)))
    (unless (not (mailbox-failure-take! (vector 'wrong-command) percentile))
      (error "terminal command identity escaped"))
    (let* ((report (mailbox-failure-take! command percentile)) (core (vector-ref report 1)))
      (unless (and (= (s64vector-ref core 2) 2000) (= (s64vector-ref core 5) 1)
                   (= (s64vector-ref core 7) 1) (= (s64vector-ref core 8) 3000)
                   (= (s64vector-ref core 9) 3000) (not mailbox-failure-slot))
        (error "terminal frozen histogram/retirement contract")))
    (unless (not (mailbox-failure-take! command percentile)) (error "duplicate terminal take"))
    (let (histogram (u64vector 99 0 1))
      (mailbox-failure-seal! command 'contract 1000300000 1000000000
        500000 100 100 100 0 histogram 0 0 0 0 0)
      (##gc)
      (let* ((report (mailbox-failure-fast-take! command)) (core (vector-ref report 1)))
        (unless (and (= (s64vector-ref core 7) 100)
                     (= (s64vector-ref core 8) (percentile histogram 100 99 100))
                     (= (s64vector-ref core 9) (percentile histogram 100 999 1000))
                     (not (mailbox-failure-fast-take! command)))
          (error "terminal C reduction disagrees with original Scheme oracle"))))
    (s64vector 1)))
;; Fully qualified names are required inside begin-foreign: Gerbil's normal
;; lexical import rewriting does not rewrite its embedded Gambit body.
(begin-foreign
 (c-define (mailbox-terminal-take root) (int64) int64 "gerbil_owner_mailbox_terminal_take" "extern"
  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
   (lambda (command)
    (gerbil-scheme-rust/qualification/mailbox-failure#mailbox-failure-fast-take! command))))
 (c-define (mailbox-terminal-contract root) (int64) int64 "gerbil_owner_mailbox_terminal_contract" "extern"
  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
   (lambda (command)
    (gerbil-scheme-rust/qualification/mailbox-failure#mailbox-failure-contract!
     command gerbil-scheme-rust/qualification/actors#actor-arrival-percentile)))))
