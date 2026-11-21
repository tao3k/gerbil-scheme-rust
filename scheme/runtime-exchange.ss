;;; Native typed service exchange. Each call is synchronous on the owning VM.
;;; Borrowed Rust spans are copied before admission; no pointer is retained.
(begin-foreign
 (namespace ("gerbil-scheme-rust/scheme/runtime#"
   gerbil-rs-exchange-enter gerbil-rs-exchange-harvest gerbil-rs-slot-numeric-copy
   gerbil-rs-exchange-write gerbil-rs-exchange-payload))
 (define gerbil-rs-exchange-write
   (c-lambda ((pointer int64) unsigned-int64 int64) void "___arg1[___arg2]=___arg3;"))
 (define (gerbil-rs-exchange-payload kind bytes size tag tag-size)
   (let ((metadata (make-s64vector tag-size))
         (data (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-value kind bytes size)))
     (let copy ((i 0))
       (when (< i tag-size)
         (s64vector-set! metadata i (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at tag i))
         (copy (+ i 1))))
     (and data (vector metadata data))))
 (c-define (gerbil-rs-exchange-enter capability kind bytes size tag tag-size header length)
   (int64 int32 (pointer unsigned-int8) unsigned-int64 (pointer int64) unsigned-int64 (pointer int64) unsigned-int64) int64
   "gerbil_scheme_rust_exchange_enter" "extern"
   (set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-call-error #f)
   (with-exception-catcher
     (lambda (failure) (set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-call-error
       (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exception-message failure)) 0)
     (lambda ()
       (let ((entry (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref capability)))
         (if (not (and (vector? entry) (= (vector-length entry) 2)
                       (procedure? (vector-ref entry 0)) (<= 2 length 4))) 0
           (let* ((op (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at header 0))
                  (lane (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at header 1))
                  (short (if (>= length 3) (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at header 2) 0))
                  (units (if (= length 4) (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at header 3) 1)))
             (if (not (and (= op 1) (>= lane -1) (<= 0 short 1) (positive? units))) 0
               (let ((payload (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exchange-payload kind bytes size tag tag-size)))
                 (if payload ((vector-ref entry 0) payload lane (= short 1) units) 0)))))))))
 ;; Metadata format [receipt-count, status-count, receipt..., statuses...].
 ;; Only the result batch becomes a root. Metadata is copied into caller storage.
 (c-define (gerbil-rs-exchange-harvest capability metadata capacity batch-output)
   (int64 (pointer int64) unsigned-int64 (pointer int64)) int64
   "gerbil_scheme_rust_exchange_harvest" "extern"
   (set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-call-error #f)
   (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exchange-write batch-output 0 0)
   (with-exception-catcher
     (lambda (failure) (set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-call-error
       (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exception-message failure)) -1)
     (lambda ()
       (let ((entry (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref capability)))
         (unless (and (vector? entry) (= (vector-length entry) 2) (procedure? (vector-ref entry 1)))
           (error "invalid exchange capability"))
         (let* ((packet ((vector-ref entry 1))) (receipt (vector-ref packet 0))
                (statuses (vector-ref packet 1)) (batch (vector-ref packet 2))
                (r (s64vector-length receipt)) (n (s64vector-length statuses)))
           (unless (and (<= n 65536) (<= (+ 2 r n) capacity)) (error "exchange metadata capacity"))
           (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exchange-write metadata 0 r)
           (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exchange-write metadata 1 n)
           (let copy ((i 0))
             (when (< i r)
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exchange-write metadata (+ 2 i) (s64vector-ref receipt i))
               (copy (+ i 1))))
           (let copy ((i 0))
             (when (< i n)
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exchange-write metadata (+ 2 r i) (s64vector-ref statuses i))
               (copy (+ i 1))))
           (when (positive? n)
             (let ((root (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store! batch)))
               (unless (positive? root) (error "exchange batch root"))
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exchange-write batch-output 0 root)))
           n)))))
 ;; Validate/copy before clearing the slot. Failed checks leave custody intact.
 (c-define (gerbil-rs-slot-numeric-copy root index kind output capacity)
   (int64 unsigned-int64 int32 (pointer unsigned-int8) unsigned-int64) int64
   "gerbil_scheme_rust_result_slot_numeric_copy" "extern"
   (with-exception-catcher (lambda (_) -1)
     (lambda ()
       (let ((batch (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root)))
         (if (not (and (vector? batch) (= (vector-length batch) 2) (positive? (vector-ref batch 0))
                       (vector? (vector-ref batch 1)) (< index (vector-length (vector-ref batch 1))))) -1
           (let ((slot (vector-ref (vector-ref batch 1) index)))
             (if (not (and (vector? slot) (= (vector-length slot) 2) (eq? (vector-ref slot 0) #t))) -1
               (let* ((value (vector-ref slot 1))
                      (size (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-span value kind #f 0 0 2)))
                 (cond
                   ((negative? size) -1)
                   ((> size capacity) -2)
                   ((not (= 0 (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-span value kind output 0 size 0))) -1)
                   (else
                     (vector-set! slot 1 #f) (vector-set! slot 0 #f)
                     (vector-set! batch 0 (- (vector-ref batch 0) 1)) size))))))))))
)
