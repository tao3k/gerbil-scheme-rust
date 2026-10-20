;; Real Scheme request handling: SDK JSON codec, validation, heap allocation,
;; table aggregation and deterministic sorted output. Rust never implements it.
(def (instance-json-summary rows collect?)
  (when collect? (##gc))
  (let ((groups (make-table test: equal?)) (count 0) (total 0) (characters 0))
    (for-each
      (lambda (row)
        (match row
          ([category quantity price label]
           (unless (and (string? category) (integer? quantity) (exact? quantity)
                        (integer? price) (exact? price) (string? label))
             (error "invalid order row"))
           (let (amount (* quantity price))
             (set! count (1+ count))
             (set! total (+ total amount))
             (set! characters (+ characters (string-length label)))
             (table-set! groups category (+ amount (table-ref groups category 0)))))
          (else (error "invalid order row shape"))))
      rows)
    [total count characters
      (map (lambda (entry) [(car entry) (cdr entry)])
           (json-sort-alist (table->list groups)))]))

(def (instance-json-call bytes)
  (let* ((request (u8vector->json bytes))
         (response
           (match request
             (["summary" rows collect?] (instance-json-summary rows collect?))
             (["echo" value] value)
             (["capture" owner] (instance-lifecycle 1 owner))
             (["resume" operation owner] (instance-lifecycle operation owner))
             (["delay" seconds value] (thread-sleep! seconds) value)
             (else (error "unknown instance request")))))
    (json->u8vector response)))

(begin-foreign
 (c-define (owner-json-call root) (int64) int64 "gerbil_owner_json_call" "extern"
  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
    gerbil-scheme-rust/qualification/actors#instance-json-call)))
