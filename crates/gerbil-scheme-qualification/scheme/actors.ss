;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Benchmark-only exports. Rust owns foreign entry; Gerbil owns actor execution.
package: gerbil-scheme-rust/qualification
(import :gerbil-scheme-rust/scheme/runtime "utf8-controls")
(export main)

(def (main . _) (void))

;; Updated only by the foreign-entry owner after joining every actor.
(def last-processor-count 0)
(def last-active-processors 0)
(def instance-marker 0)

;; Diagnostic-only synchronous callback from inside Scheme. The host owns the
;; context for the entire call; no Scheme object or context escapes its VM.
(extern instance-enter)
(begin-foreign
  (namespace ("gerbil-scheme-rust/qualification/actors#" instance-enter))
  (c-declare "struct gerbil_instance_probe_context { void (*entered)(void *); void *data; };")
  (define instance-enter
    (c-lambda ((pointer void)) void
      "if (___arg1) { struct gerbil_instance_probe_context *ctx = ___arg1; ctx->entered(ctx->data); }")))

(def (instance-batch marker jobs rounds context)
  (with-catch (lambda (exn) (display-exception exn (current-error-port)) -1)
    (lambda ()
      (set! instance-marker marker)
      (##gc)
      (instance-enter context)
      (let loop ((remaining jobs) (sum 0))
        (if (zero? remaining) sum
            (loop (1- remaining) (+ sum (checksum rounds))))))))

(def (checksum rounds)
  (let loop ((remaining rounds) (sum 0))
    (if (zero? remaining) sum
        (loop (1- remaining) (+ sum remaining)))))

(def (actor-batch processors actors jobs rounds fail?)
  ;; Restore the actual VM population, not the configuration level (zero means
  ;; machine-sized startup policy and is not the original processor count).
  (let ((original (##current-vm-processor-count)) (pending []) (results []))
    (dynamic-wind
      ;; The configuration setter alone does not resize an initialized VM.
      ;; Use Gambit's own processor startup and scheduler initialization path.
      (lambda () (##cvmr processors))
      (lambda ()
        (set! last-active-processors (##current-vm-processor-count))
        (unless (= last-active-processors processors)
          (error "SDK did not activate the requested VM processors"
                 processors last-active-processors))
        (let spawn-loop ((worker 0))
          (when (< worker actors)
            (let (actor
                  (spawn
                   (lambda ()
                     (let work-loop ((job worker) (sum 0))
                       (cond
                        ((>= job jobs) (cons sum (##current-processor-id)))
                        ((and fail? (zero? job)) (error "injected worker failure"))
                        (else (work-loop (+ job actors) (+ sum (checksum rounds)))))))))
              (set! pending (cons actor pending)))
            (spawn-loop (1+ worker))))
        ;; Observe every terminal actor, including after an earlier actor fails.
        (set! results
          (map (lambda (actor)
                 (with-catch (lambda (_) #f) (lambda () (thread-join! actor))))
               pending))
        ;; Count distinct sampled processors without imposing a 64-bit mask cap.
        ;; Placement samples do not prove simultaneous execution or speedup.
        (set! last-processor-count
          (length
           (foldl (lambda (result seen)
                    (if (and (pair? result) (not (member (cdr result) seen)))
                      (cons (cdr result) seen) seen))
                  [] results)))
        (if (andmap pair? results) (apply + (map car results)) -1))
      (lambda ()
        (for-each (lambda (actor)
                    (with-catch void (lambda () (thread-join! actor))))
                  pending)
        (##cvmr original)))))

(def (checked-actor-batch processors actors jobs rounds fail?)
  (with-catch (lambda (exn)
                (display-exception exn (current-error-port))
                -1)
    (lambda () (actor-batch processors actors jobs rounds (not (zero? fail?))))))

(begin-foreign
  (namespace ("gerbil-scheme-rust/qualification/actors#" actor-qualification-batch actor-qualification-processor-count actor-qualification-active-processors))
  (c-define (actor-qualification-active-processors)
    () unsigned-int32 "gerbil_actor_qualification_active_processors" "extern"
    gerbil-scheme-rust/qualification/actors#last-active-processors)
  (c-define (instance-qualification-marker)
    () unsigned-int32 "gerbil_instance_qualification_marker" "extern"
    gerbil-scheme-rust/qualification/actors#instance-marker)
  (c-define (instance-qualification-batch marker jobs rounds context)
    (unsigned-int32 unsigned-int32 unsigned-int32 (pointer void)) int64
    "gerbil_instance_qualification_batch" "extern"
    (gerbil-scheme-rust/qualification/actors#instance-batch marker jobs rounds context))
  (c-define (actor-qualification-processor-count)
    () unsigned-int32 "gerbil_actor_qualification_processor_count" "extern"
    gerbil-scheme-rust/qualification/actors#last-processor-count)
  (c-define (actor-qualification-batch processors actors jobs rounds fail?)
    (unsigned-int32 unsigned-int32 unsigned-int32 unsigned-int32 unsigned-int32) int64
    "gerbil_actor_qualification_batch" "extern"
    (gerbil-scheme-rust/qualification/actors#checked-actor-batch processors actors jobs rounds fail?)))
