(declare (block) (standard-bindings) (extended-bindings))
(begin
  (define gerbil-scheme-rust/scheme/runtime::timestamp 1793333971)
  (begin
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-abi-version
      (lambda () '1))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-add-i64
      (lambda (_%left783%_ _%right784%_) (+ _%left783%_ _%right784%_)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-is-even-i64
      (lambda (_%value781%_) (if (even? _%value781%_) '1 '0)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-compare-i64
      (lambda (_%left775%_ _%right776%_)
        (if (< _%left775%_ _%right776%_)
            '-1
            (if (> _%left775%_ _%right776%_) '1 '0))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-next-root-id '1)
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values
      (make-table 'test: eqv? 'weak-keys: '#f 'weak-values: '#f))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing
      (let () (declare (not safe)) (##gensym 'missing-root)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-string
      (lambda (_%value773%_)
        (if (string? _%value773%_)
            (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
             _%value773%_)
            '0)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-bytevector
      (lambda (_%value771%_)
        (if (u8vector? _%value771%_)
            (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
             _%value771%_)
            '0)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-object
      (lambda (_%value769%_)
        (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
         _%value769%_)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-call-error '#f)
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-exception-message
      (lambda (_%exception760%_)
        (let ((__tmp9604
               (lambda (_%_762%_) '"Scheme exception (formatting failed)"))
              (__tmp9603
               (lambda ()
                 (let ((_%message767%_
                        (call-with-output-string
                         (lambda (_%port765%_)
                           (let ()
                             (declare (not safe))
                             (display-exception__%
                              _%exception760%_
                              _%port765%_))))))
                   (substring
                    _%message767%_
                    '0
                    (min '4096
                         (let ()
                           (declare (not safe))
                           (##string-length _%message767%_))))))))
          (declare (not safe))
          (__with-catch __tmp9604 __tmp9603))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call
      (lambda (_%root752%_ _%procedure753%_)
        (set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-call-error '#f)
        (let ((__tmp9606
               (lambda (_%exception755%_)
                 (set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-call-error
                       (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exception-message
                        _%exception755%_))
                 '0))
              (__tmp9605
               (lambda ()
                 (let ((_%value758%_
                        (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                         _%root752%_)))
                   (if (eq? _%value758%_
                            gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing)
                       '0
                       (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
                        (_%procedure753%_ _%value758%_)))))))
          (declare (not safe))
          (__with-catch __tmp9606 __tmp9605))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
      (lambda (_%value748%_)
        (let ((_%root-id750%_
               gerbil-scheme-rust/scheme/runtime#gerbil-rs-next-root-id))
          (set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-next-root-id
                (+ _%root-id750%_ '1))
          (let ()
            (declare (not safe))
            (##table-set!
             gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values
             _%root-id750%_
             _%value748%_))
          _%root-id750%_)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
      (lambda (_%root-id746%_)
        (let ()
          (declare (not safe))
          (##table-ref
           gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values
           _%root-id746%_
           gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-release!
      (lambda (_%root-id744%_)
        (if (eq? (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                  _%root-id744%_)
                 gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing)
            '#f
            (begin
              (let ()
                (declare (not safe))
                (##table-set!
                 gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values
                 _%root-id744%_))
              '#t))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-collection-view
      (lambda (_%value738%_ _%kind739%_)
        (let ((_%$e741%_ _%kind739%_))
          (if (eq? '0 _%$e741%_)
              (if (vector? _%value738%_) _%value738%_ '#f)
              (if (eq? '1 _%$e741%_)
                  (if (list? _%value738%_) (list->vector _%value738%_) '#f)
                  (if (eq? '2 _%$e741%_)
                      (if (pair? _%value738%_)
                          (vector (let ()
                                    (declare (not safe))
                                    (##car _%value738%_))
                                  (let ()
                                    (declare (not safe))
                                    (##cdr _%value738%_)))
                          '#f)
                      '#f))))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value
      (lambda (_%root730%_ _%index731%_)
        (let ((_%value733%_
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                _%root730%_)))
          (if (= _%index731%_ '-1)
              _%value733%_
              (if (and (>= _%index731%_ '0)
                       (vector? _%value733%_)
                       (< _%index731%_
                          (let ()
                            (declare (not safe))
                            (##vector-length _%value733%_))))
                  (vector-ref _%value733%_ _%index731%_)
                  gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing)))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-identifier-packet
      (lambda (_%value699%_)
        (let* ((_%values701%_
                (if (vector? _%value699%_) _%value699%_ (vector _%value699%_)))
               (_%count703%_
                (let () (declare (not safe)) (##vector-length _%values701%_)))
               (_%names705%_
                (let () (declare (not safe)) (##make-vector _%count703%_)))
               (_%kinds707%_
                (let () (declare (not safe)) (##make-u8vector _%count703%_))))
          (let _%prepare710%_ ((_%index712%_ '0) (_%capacity713%_ '0))
            (if (= _%index712%_ _%count703%_)
                (vector _%names705%_ _%kinds707%_ _%capacity713%_)
                (let* ((_%item715%_ (vector-ref _%values701%_ _%index712%_))
                       (_%kind720%_
                        (if (symbol? _%item715%_)
                            '0
                            (if (keyword? _%item715%_) '1 '-1)))
                       (_%name725%_
                        (let ((_%$e722%_ _%kind720%_))
                          (if (eq? '0 _%$e722%_)
                              (symbol->string _%item715%_)
                              (if (eq? '1 _%$e722%_)
                                  (keyword->string _%item715%_)
                                  '#f)))))
                  (if _%name725%_
                      (let ((_%next728%_
                             (+ _%capacity713%_
                                (* '4
                                   (let ()
                                     (declare (not safe))
                                     (##string-length _%name725%_))))))
                        (if (fixnum? _%next728%_)
                            (begin
                              (vector-set!
                               _%names705%_
                               _%index712%_
                               _%name725%_)
                              (u8vector-set!
                               _%kinds707%_
                               _%index712%_
                               _%kind720%_)
                              (_%prepare710%_ (+ _%index712%_ '1) _%next728%_))
                            '#f))
                      '#f)))))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-result-batch-pending
      (lambda (_%batch697%_)
        (if _%batch697%_ (vector-ref _%batch697%_ '0) '0)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-clear-result-batch!
      (lambda (_%batch693%_)
        (if _%batch693%_
            (begin
              (vector-for-each
               (lambda (_%slot695%_)
                 (vector-set! _%slot695%_ '1 '#f)
                 (vector-set! _%slot695%_ '0 '#f))
               (vector-ref _%batch693%_ '1))
              (vector-set! _%batch693%_ '0 '0))
            '#!void)))
    (namespace
     ("gerbil-scheme-rust/scheme/runtime#"
      gerbil-rs-root-id-at
      gerbil-rs-write-root-id
      gerbil-rs-scalar-copy-c
      gerbil-rs-object-scalar-raw
      gerbil-rs-root-count-raw
      gerbil-rs-force-gc-raw
      gerbil-rs-read-limb
      gerbil-rs-write-limb
      gerbil-rs-read-small-integer
      gerbil-rs-copy-small-integer
      gerbil-rs-copy-integer-value
      gerbil-rs-object-root-raw
      gerbil-rs-object-word-raw
      gerbil-rs-collection-create-raw
      gerbil-rs-collection-length-raw
      gerbil-rs-collection-element-raw
      gerbil-rs-collection-snapshot-raw
      gerbil-rs-object-scalar-indexed
      gerbil-rs-bigint-size-indexed
      gerbil-rs-bigint-copy-indexed
      gerbil-rs-roots-release-raw
      gerbil-rs-bigint-create-raw
      gerbil-rs-bigint-size-raw
      gerbil-rs-bigint-copy-raw
      gerbil-rs-bigint-create-byte-control
      gerbil-rs-bigint-copy-byte-control))
    (namespace
     ("gerbil-scheme-rust/scheme/runtime#"
      gerbil-rs-numeric-span
      gerbil-rs-numeric-create
      gerbil-rs-numeric-size
      gerbil-rs-numeric-copy
      gerbil-rs-numeric-value
      gerbil-rs-numeric-batch-create
      gerbil-rs-numeric-batch-value
      gerbil-rs-numeric-header-at
      gerbil-rs-numeric-service-create
      gerbil-rs-tagged-numeric-service-create
      gerbil-rs-numeric-row-pointer
      gerbil-rs-numeric-row-size
      gerbil-rs-numeric-service-enter
      gerbil-rs-numeric-batch-snapshot
      gerbil-rs-numeric-batch-copy
      gerbil-rs-numeric-write-size))
    (namespace
     ("gerbil-scheme-rust/scheme/runtime#"
      gerbil-rs-identifier-create
      gerbil-rs-identifiers-prepare
      gerbil-rs-identifiers-copy
      gerbil-rs-object-identical
      gerbil-rs-identifier-metadata
      gerbil-rs-identifier-slot
      gerbil-rs-identifier-end
      gerbil-rs-pointer-offset))
    (define gerbil-rs-identifier-metadata
      (c-lambda
       ((pointer unsigned-int64) unsigned-int64 unsigned-int64)
       void
       "___arg1[0]=___arg2; ___arg1[1]=___arg3;"))
    (define gerbil-rs-identifier-slot
      (c-lambda
       ((pointer unsigned-int64)
        (pointer unsigned-int8)
        unsigned-int64
        unsigned-int64
        int32)
       void
       "___arg1[___arg3]=___arg4; ___arg2[___arg3]=___arg5;"))
    (define gerbil-rs-identifier-end
      (c-lambda
       ((pointer unsigned-int64) unsigned-int64 unsigned-int64)
       void
       "___arg1[___arg2]=___arg3;"))
    (define gerbil-rs-pointer-offset
      (c-lambda
       ((pointer void) unsigned-int64)
       (pointer void)
       "___return(___arg2 == 0 ? ___arg1 : (unsigned char*)___arg1 + ___arg2);"))
    (c-define
     (gerbil-rs-identifier-create string-root kind)
     (int64 int32)
     int64
     "gerbil_scheme_rust_identifier_create"
     "extern"
     (with-exception-catcher
      (lambda (_exception) 0)
      (lambda ()
        (let ((name (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                     string-root)))
          (if (not (string? name))
              0
              (let ((value (case kind
                             ((0) (string->symbol name))
                             ((1) (string->keyword name))
                             ((2) (string->uninterned-symbol name))
                             (else #f))))
                (if value
                    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
                     value)
                    0)))))))
    (c-define
     (gerbil-rs-object-identical left li right ri)
     (int64 int64 int64 int64)
     int32
     "gerbil_scheme_rust_object_identical"
     "extern"
     (if (eq? (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value
               left
               li)
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value
               right
               ri))
         1
         0))
    (c-define
     (gerbil-rs-identifiers-prepare root index metadata)
     (int64 int64 (pointer unsigned-int64))
     int64
     "gerbil_scheme_rust_identifiers_prepare"
     "extern"
     (with-exception-catcher
      (lambda (_exception) 0)
      (lambda ()
        (let* ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value
                       root
                       index))
               (packet (and (not (eq? value
                                      gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing))
                            (gerbil-scheme-rust/scheme/runtime#gerbil-rs-identifier-packet
                             value))))
          (if (not packet)
              0
              (begin
                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-identifier-metadata
                 metadata
                 (vector-length (vector-ref packet 0))
                 (vector-ref packet 2))
                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
                 packet)))))))
    (c-define
     (gerbil-rs-identifiers-copy root bytes capacity offsets kinds count)
     (int64 (pointer void)
            unsigned-int64
            (pointer unsigned-int64)
            (pointer unsigned-int8)
            unsigned-int64)
     int64
     "gerbil_scheme_rust_identifiers_copy"
     "extern"
     (with-exception-catcher
      (lambda (_exception) -1)
      (lambda ()
        (let* ((packet (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                        root))
               (names (vector-ref packet 0))
               (tags (vector-ref packet 1)))
          (if (not (and (= count (vector-length names))
                        (>= capacity (vector-ref packet 2))))
              -1
              (let encode ((index 0) (written 0))
                (if (= index count)
                    (begin
                      (gerbil-scheme-rust/scheme/runtime#gerbil-rs-identifier-end
                       offsets
                       count
                       written)
                      written)
                    (let ((size (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-into
                                 (vector-ref names index)
                                 (gerbil-scheme-rust/scheme/runtime#gerbil-rs-pointer-offset
                                  bytes
                                  written)
                                 (- capacity written))))
                      (gerbil-scheme-rust/scheme/runtime#gerbil-rs-identifier-slot
                       offsets
                       kinds
                       index
                       written
                       (u8vector-ref tags index))
                      (encode (+ index 1) (+ written size))))))))))
    (c-declare "#include <string.h>\n#include <stdint.h>")
    (define gerbil-rs-numeric-span
      (c-lambda
       (scheme-object
        int32
        (pointer unsigned-int8)
        unsigned-int64
        unsigned-int64
        int32)
       int64
       "___SCMOBJ ___temp;\nvoid *body = 0; ___U64 count = 0, width = 0;\n#define GRS_NUMERIC(K,T,W) case K: if (!___##T##VECTORP(___arg1)) ___return(-1); count=___##T##VECTORSIZE(___arg1); width=W; body=___BODY_AS(___arg1,___t##T##VECTOR); break\nswitch (___arg2) {\n  GRS_NUMERIC(0,U16,2); GRS_NUMERIC(1,U32,4); GRS_NUMERIC(2,U64,8);\n  GRS_NUMERIC(3,F32,4); GRS_NUMERIC(4,F64,8);\n  GRS_NUMERIC(5,S16,2); GRS_NUMERIC(6,S32,4); GRS_NUMERIC(7,S64,8);\n  GRS_NUMERIC(8,S8,1);\n  default: ___return(-1);\n}\n#undef GRS_NUMERIC\nif (___arg6 == 2) ___return(count);\nif (___arg4 > count || ___arg5 > count-___arg4 ||\n    ___arg5 > SIZE_MAX/width || (___arg5 && !___arg3)) ___return(-1);\nif (___arg5) {\n  unsigned char *start = (unsigned char*)body + ___arg4*width;\n  unsigned char *destination, *source;\n  if (___arg6 == 0) { destination=___arg3; source=start; }\n  else if (___arg6 == 1) { destination=start; source=___arg3; }\n  else ___return(-1);\n  /* Constant-size copies allow unaligned spans without aliasing casts. */\n  ___U64 bytes=___arg5*width, offset=0;\n  for (; bytes-offset >= 8; offset+=8) {\n    ___U64 word; memcpy(&word,source+offset,8); memcpy(destination+offset,&word,8);\n  }\n  for (; offset < bytes; offset++) destination[offset]=source[offset];\n}\n___return(0);"))
    (define (gerbil-rs-numeric-value kind bytes size)
      (let ((value (case kind
                     ((0) (make-u16vector size))
                     ((1) (make-u32vector size))
                     ((2) (make-u64vector size))
                     ((3) (make-f32vector size))
                     ((4) (make-f64vector size))
                     ((5) (make-s16vector size))
                     ((6) (make-s32vector size))
                     ((7) (make-s64vector size))
                     ((8) (make-s8vector size))
                     (else #f))))
        (and value
             (= 0
                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-span
                 value
                 kind
                 bytes
                 0
                 size
                 1))
             value)))
    (c-define
     (gerbil-rs-numeric-create kind bytes size)
     (int32 (pointer unsigned-int8) unsigned-int64)
     int64
     "gerbil_scheme_rust_numeric_create"
     "extern"
     (with-exception-catcher
      (lambda (_exception) 0)
      (lambda ()
        (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-value
                      kind
                      bytes
                      size)))
          (if value
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
               value)
              0)))))
    (define gerbil-rs-numeric-row-pointer
      (c-lambda
       ((pointer void) unsigned-int64)
       (pointer unsigned-int8)
       "___return(((unsigned char * const *)___arg1)[___arg2]);"))
    (define gerbil-rs-numeric-row-size
      (c-lambda
       ((pointer unsigned-int64) unsigned-int64)
       unsigned-int64
       "___return(___arg1[___arg2]);"))
    (define (gerbil-rs-numeric-batch-value kind pointers sizes count quantum)
      (and (<= 0 kind 8)
           (<= 1 count 256)
           (<= 0 quantum 256)
           (let ((input (make-vector count)))
             (let loop ((index 0))
               (if (= index count)
                   (vector (make-s64vector 1 quantum) input)
                   (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-value
                                 kind
                                 (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-row-pointer
                                  pointers
                                  index)
                                 (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-row-size
                                  sizes
                                  index))))
                     (and value
                          (begin
                            (vector-set! input index value)
                            (loop (+ index 1))))))))))
    (c-define
     (gerbil-rs-numeric-batch-create kind pointers sizes count quantum)
     (int32 (pointer void)
            (pointer unsigned-int64)
            unsigned-int64
            unsigned-int64)
     int64
     "gerbil_scheme_rust_numeric_batch_create"
     "extern"
     (with-exception-catcher
      (lambda (_exception) 0)
      (lambda ()
        (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-batch-value
                      kind
                      pointers
                      sizes
                      count
                      quantum)))
          (if value
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
               value)
              0)))))
    (define gerbil-rs-numeric-header-at
      (c-lambda
       ((pointer int64) unsigned-int64)
       int64
       "___return(___arg1[___arg2]);"))
    (c-define
     (gerbil-rs-numeric-service-create
      kind
      pointers
      sizes
      count
      quantum
      header
      length)
     (int32 (pointer void)
            (pointer unsigned-int64)
            unsigned-int64
            unsigned-int64
            (pointer int64)
            unsigned-int64)
     int64
     "gerbil_scheme_rust_numeric_service_create"
     "extern"
     (with-exception-catcher
      (lambda (_exception) 0)
      (lambda ()
        (if (not (<= 2 length 4))
            0
            (let ((operation (make-s64vector length)))
              (let loop ((index 0))
                (when (< index length)
                  (s64vector-set!
                   operation
                   index
                   (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at
                    header
                    index))
                  (loop (+ index 1))))
              (if (not (and (= (s64vector-ref operation 0) 1)
                            (>= (s64vector-ref operation 1) -1)
                            (or (< length 3)
                                (<= 0 (s64vector-ref operation 2) 1))
                            (or (< length 4)
                                (positive? (s64vector-ref operation 3)))))
                  0
                  (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-batch-value
                                kind
                                pointers
                                sizes
                                count
                                quantum)))
                    (if value
                        (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
                         (vector operation value))
                        0))))))))
    (c-define
     (gerbil-rs-tagged-numeric-service-create
      kind
      bytes
      size
      tag
      tag-size
      header
      length)
     (int32 (pointer unsigned-int8)
            unsigned-int64
            (pointer int64)
            unsigned-int64
            (pointer int64)
            unsigned-int64)
     int64
     "gerbil_scheme_rust_tagged_numeric_service_create"
     "extern"
     (with-exception-catcher
      (lambda (_exception) 0)
      (lambda ()
        (if (not (<= 2 length 4))
            0
            (let ((operation (make-s64vector length))
                  (metadata (make-s64vector tag-size)))
              (let loop ((index 0))
                (when (< index length)
                  (s64vector-set!
                   operation
                   index
                   (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at
                    header
                    index))
                  (loop (+ index 1))))
              (let loop ((index 0))
                (when (< index tag-size)
                  (s64vector-set!
                   metadata
                   index
                   (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at
                    tag
                    index))
                  (loop (+ index 1))))
              (if (not (and (= (s64vector-ref operation 0) 1)
                            (>= (s64vector-ref operation 1) -1)
                            (or (< length 3)
                                (<= 0 (s64vector-ref operation 2) 1))
                            (or (< length 4)
                                (positive? (s64vector-ref operation 3)))))
                  0
                  (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-value
                                kind
                                bytes
                                size)))
                    (if value
                        (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
                         (vector operation (vector metadata value)))
                        0))))))))
    (c-define
     (gerbil-rs-numeric-service-enter
      kind
      pointers
      sizes
      count
      quantum
      header
      length
      capability)
     (int32 (pointer void)
            (pointer unsigned-int64)
            unsigned-int64
            unsigned-int64
            (pointer int64)
            unsigned-int64
            int64)
     int64
     "gerbil_scheme_rust_numeric_service_enter"
     "extern"
     (set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-call-error #f)
     (with-exception-catcher
      (lambda (exception)
        (set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-call-error
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exception-message
               exception))
        0)
      (lambda ()
        (let ((entry (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                      capability)))
          (if (not (and (procedure? entry) (<= 2 length 4)))
              0
              (let* ((op (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at
                          header
                          0))
                     (lane (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at
                            header
                            1))
                     (short (if (>= length 3)
                                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at
                                 header
                                 2)
                                0))
                     (units (if (= length 4)
                                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at
                                 header
                                 3)
                                1)))
                (if (not (and (= op 1)
                              (>= lane -1)
                              (<= 0 short 1)
                              (positive? units)))
                    0
                    (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-batch-value
                                  kind
                                  pointers
                                  sizes
                                  count
                                  quantum)))
                      (if value
                          (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
                           (entry value lane (= short 1) units))
                          0)))))))))
    (define gerbil-rs-numeric-write-size
      (c-lambda
       ((pointer unsigned-int64) unsigned-int64 unsigned-int64)
       void
       "___arg1[___arg2]=___arg3;"))
    (c-define
     (gerbil-rs-numeric-batch-snapshot root kind sizes count)
     (int64 int32 (pointer unsigned-int64) unsigned-int64)
     int64
     "gerbil_scheme_rust_numeric_batch_snapshot"
     "extern"
     (with-exception-catcher
      (lambda (_) 0)
      (lambda ()
        (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                      root)))
          (if (not (and (<= 1 count 256)
                        (vector? value)
                        (= count (vector-length value))))
              0
              (let ((snapshot (##vector-copy value)))
                (let scan ((index 0))
                  (if (= index count)
                      (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
                       snapshot)
                      (let ((size (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-span
                                   (vector-ref snapshot index)
                                   kind
                                   #f
                                   0
                                   0
                                   2)))
                        (if (< size 0)
                            0
                            (begin
                              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-write-size
                               sizes
                               index
                               size)
                              (scan (+ index 1)))))))))))))
    (c-define
     (gerbil-rs-numeric-batch-copy root kind pointers sizes count)
     (int64 int32 (pointer void) (pointer unsigned-int64) unsigned-int64)
     int32
     "gerbil_scheme_rust_numeric_batch_copy"
     "extern"
     (with-exception-catcher
      (lambda (_) -1)
      (lambda ()
        (let ((snapshot
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                root)))
          (if (not (and (<= 1 count 256)
                        (vector? snapshot)
                        (= count (vector-length snapshot))))
              -1
              (let validate ((index 0))
                (if (= index count)
                    (let copy ((index 0))
                      (if (= index count)
                          0
                          (if (= 0
                                 (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-span
                                  (vector-ref snapshot index)
                                  kind
                                  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-row-pointer
                                   pointers
                                   index)
                                  0
                                  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-row-size
                                   sizes
                                   index)
                                  0))
                              (copy (+ index 1))
                              -1)))
                    (if (= (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-span
                            (vector-ref snapshot index)
                            kind
                            #f
                            0
                            0
                            2)
                           (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-row-size
                            sizes
                            index))
                        (validate (+ index 1))
                        -1))))))))
    (c-define
     (gerbil-rs-numeric-size root index kind)
     (int64 int64 int32)
     int64
     "gerbil_scheme_rust_numeric_size"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-span
      (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value root index)
      kind
      #f
      0
      0
      2))
    (c-define
     (gerbil-rs-numeric-copy root index kind output offset size)
     (int64 int64 int32 (pointer unsigned-int8) unsigned-int64 unsigned-int64)
     int64
     "gerbil_scheme_rust_numeric_copy"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-span
      (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value root index)
      kind
      output
      offset
      size
      0))
    (define gerbil-rs-root-id-at
      (c-lambda
       ((pointer int64) unsigned-int64)
       int64
       "___return(___arg1[___arg2]);"))
    (define gerbil-rs-write-root-id
      (c-lambda
       ((pointer int64) unsigned-int64 int64)
       void
       "___arg1[___arg2] = ___arg3;"))
    (c-declare "#include <string.h>")
    (define gerbil-rs-scalar-copy-c
      (c-lambda
       (scheme-object (pointer unsigned-int8))
       int32
       "___FLONUMP_DECL\n___U32 tag=0, reserved=0;\n___U64 bits=0;\nif (___arg1 == ___FAL || ___arg1 == ___TRU) { tag=1; bits=___arg1 == ___TRU; }\nelse if (___FIXNUMP(___arg1)) { tag=2; bits=(___U64)(___S64)___INT(___arg1); }\nelse if (___CHARP(___arg1)) {\n  bits=___ORD(___arg1);\n  if (bits <= 0x10ffff && !(bits >= 0xd800 && bits <= 0xdfff)) tag=3;\n} else if (___FLONUMP(___arg1)) { tag=4; double n=___F64UNBOX(___arg1); memcpy(&bits,&n,8); }\nif (tag == 0) ___return(3);\nmemcpy(___arg2,&tag,4); memcpy(___arg2+4,&reserved,4); memcpy(___arg2+8,&bits,8);\n___return(0);"))
    (c-define
     (gerbil-rs-object-scalar-raw root output)
     (int64 (pointer unsigned-int8))
     int32
     "gerbil_scheme_rust_object_scalar_raw"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-scalar-copy-c
      (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root)
      output))
    (c-define
     (gerbil-rs-object-scalar-indexed root index output)
     (int64 int64 (pointer unsigned-int8))
     int32
     "gerbil_scheme_rust_object_scalar_indexed"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-scalar-copy-c
      (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value root index)
      output))
    (c-define
     (gerbil-rs-collection-snapshot-raw root kind size)
     (int64 int32 unsigned-int64)
     int64
     "gerbil_scheme_rust_collection_snapshot_raw"
     "extern"
     (with-exception-catcher
      (lambda (_exception) 0)
      (lambda ()
        (let ((values (gerbil-scheme-rust/scheme/runtime#gerbil-rs-collection-view
                       (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                        root)
                       kind)))
          (if (not (and values (= size (vector-length values))))
              0
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
               (if (= kind 0) (##vector-copy values) values)))))))
    (define gerbil-rs-read-limb
      (c-lambda
       ((pointer unsigned-int8) unsigned-int64 unsigned-int64)
       unsigned-int64
       "___U64 n=0; for (___U64 i=0;i<___arg3;i++) n |= (___U64)___arg1[___arg2+i] << (i*8); ___return(n);"))
    (define gerbil-rs-write-limb
      (c-lambda
       ((pointer unsigned-int8) unsigned-int64 unsigned-int64 unsigned-int64)
       void
       "for (___U64 i=0;i<___arg3;i++) ___arg1[___arg2+i] = (___U8)(___arg4 >> (i*8));"))
    (define gerbil-rs-read-small-integer
      (c-lambda
       ((pointer unsigned-int8) unsigned-int64 int32)
       scheme-object
       "___S64 n = ___arg2 == 0 ? 0 : ___arg1[0]; ___return(___FIX(___arg3 == 0 ? n : -n));"))
    (define gerbil-rs-copy-small-integer
      (c-lambda
       (scheme-object (pointer unsigned-int8) unsigned-int64)
       int32
       "if (!___FIXNUMP(___arg1)) ___return(-2);\n___S64 n = ___INT(___arg1);\n___U64 magnitude = n < 0 ? (___U64)(-n) : (___U64)n;\nif ((___arg3 == 0 && magnitude != 0) ||\n    (___arg3 == 1 && (magnitude == 0 || magnitude > 255))) ___return(-1);\nif (___arg3 == 1) ___arg2[0] = (___U8)magnitude;\n___return(n < 0 ? 1 : 0);"))
    (define (gerbil-rs-copy-integer-value value output size)
      (let ((small (if (<= size 1)
                       (gerbil-scheme-rust/scheme/runtime#gerbil-rs-copy-small-integer
                        value
                        output
                        size)
                       -2)))
        (if (not (= small -2))
            small
            (if (not (and (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer?
                           value)
                          (= size
                             (quotient (+ (integer-length (abs value)) 7) 8))))
                -1
                (begin
                  (let lp ((offset 0) (rest (abs value)))
                    (when (< offset size)
                      (let ((count (min 8 (- size offset))))
                        (gerbil-scheme-rust/scheme/runtime#gerbil-rs-write-limb
                         output
                         offset
                         count
                         (bitwise-and rest 18446744073709551615))
                        (lp (+ offset count)
                            (arithmetic-shift rest (* -8 count))))))
                  (if (< value 0) 1 0))))))
    (c-define
     (gerbil-rs-object-root-raw value)
     (scheme-object)
     int64
     "gerbil_scheme_rust_object_root_raw"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store! value))
    (c-define
     (gerbil-rs-object-word-raw root)
     (int64)
     scheme-object
     "gerbil_scheme_rust_object_word_raw"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root))
    (c-define
     (gerbil-rs-root-count-raw)
     ()
     int64
     "gerbil_scheme_rust_root_count_raw"
     "extern"
     (table-length gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values))
    (c-define
     (gerbil-rs-force-gc-raw)
     ()
     int32
     "gerbil_scheme_rust_force_gc_raw"
     "extern"
     (##gc)
     0)
    (c-define
     (gerbil-rs-collection-create-raw roots size kind)
     ((pointer int64) unsigned-int64 int32)
     int64
     "gerbil_scheme_rust_collection_create_raw"
     "extern"
     (with-exception-catcher
      (lambda (_exception) 0)
      (lambda ()
        (if (or (not (<= 0 kind 2)) (and (= kind 2) (not (= size 2))))
            0
            (let ((values (make-vector size)))
              (let lp ((index 0))
                (if (= index size)
                    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
                     (case kind
                       ((0) values)
                       ((1) (vector->list values))
                       ((2)
                        (cons (vector-ref values 0) (vector-ref values 1)))))
                    (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                                  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at
                                   roots
                                   index))))
                      (if (eq? value
                               gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing)
                          0
                          (begin
                            (vector-set! values index value)
                            (lp (+ index 1))))))))))))
    (c-define
     (gerbil-rs-collection-length-raw root kind)
     (int64 int32)
     int64
     "gerbil_scheme_rust_collection_length_raw"
     "extern"
     (let ((values (gerbil-scheme-rust/scheme/runtime#gerbil-rs-collection-view
                    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                     root)
                    kind)))
       (if values (vector-length values) -1)))
    (c-define
     (gerbil-rs-collection-element-raw root kind index)
     (int64 int32 unsigned-int64)
     int64
     "gerbil_scheme_rust_collection_element_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                   root)))
       (case kind
         ((0)
          (if (and (vector? value) (< index (vector-length value)))
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
               (vector-ref value index))
              0))
         ((1)
          (if (and (list? value) (< index (length value)))
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
               (list-ref value index))
              0))
         ((2)
          (if (and (pair? value) (< index 2))
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
               (if (= index 0) (car value) (cdr value)))
              0))
         (else 0))))
    (c-define
     (gerbil-rs-roots-release-raw roots size)
     ((pointer int64) unsigned-int64)
     int32
     "gerbil_scheme_rust_roots_release_raw"
     "extern"
     (if (= size 2)
         (let ((first (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at
                       roots
                       0))
               (second (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at
                        roots
                        1)))
           (if (and (> first 0)
                    (> second first)
                    (not (eq? (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                               first)
                              gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing))
                    (not (eq? (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                               second)
                              gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing)))
               (begin
                 (table-set!
                  gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values
                  first)
                 (table-set!
                  gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values
                  second)
                 0)
               3))
         (let validate ((index 0) (previous 0))
           (if (= index size)
               (begin
                 (let release ((index 0))
                   (when (< index size)
                     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-release!
                      (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at
                       roots
                       index))
                     (release (+ index 1))))
                 0)
               (let ((id (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at
                          roots
                          index)))
                 (if (and (> id previous)
                          (not (eq? (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                                     id)
                                    gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing)))
                     (validate (+ index 1) id)
                     3))))))
    (c-define
     (gerbil-rs-bigint-create-raw source size negative)
     ((pointer unsigned-int8) unsigned-int64 int32)
     int64
     "gerbil_scheme_rust_bigint_create_raw"
     "extern"
     (if (<= size 1)
         (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
          (gerbil-scheme-rust/scheme/runtime#gerbil-rs-read-small-integer
           source
           size
           negative))
         (let lp ((end size) (number 0))
           (if (zero? end)
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
                (if (= negative 0) number (- number)))
               (let* ((start (max 0 (- end 8))) (count (- end start)))
                 (lp start
                     (bitwise-ior
                      (arithmetic-shift number (* count 8))
                      (gerbil-scheme-rust/scheme/runtime#gerbil-rs-read-limb
                       source
                       start
                       count))))))))
    (c-define
     (gerbil-rs-bigint-size-raw root)
     (int64)
     int64
     "gerbil_scheme_rust_bigint_size_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                   root)))
       (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer? value)
           (quotient (+ (integer-length (abs value)) 7) 8)
           -1)))
    (c-define
     (gerbil-rs-bigint-size-indexed root index)
     (int64 int64)
     int64
     "gerbil_scheme_rust_bigint_size_indexed"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value
                   root
                   index)))
       (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer? value)
           (quotient (+ (integer-length (abs value)) 7) 8)
           -1)))
    (c-define
     (gerbil-rs-bigint-create-byte-control source size negative)
     ((pointer unsigned-int8) unsigned-int64 int32)
     int64
     "gerbil_scheme_rust_bigint_create_byte_control"
     "extern"
     (let lp ((end size) (number 0))
       (if (zero? end)
           (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
            (if (= negative 0) number (- number)))
           (lp (- end 1)
               (bitwise-ior
                (arithmetic-shift number 8)
                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-read-limb
                 source
                 (- end 1)
                 1))))))
    (c-define
     (gerbil-rs-bigint-copy-byte-control root output size)
     (int64 (pointer unsigned-int8) unsigned-int64)
     int32
     "gerbil_scheme_rust_bigint_copy_byte_control"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                   root)))
       (if (not (and (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer?
                      value)
                     (= size (quotient (+ (integer-length (abs value)) 7) 8))))
           -1
           (begin
             (let lp ((offset 0) (rest (abs value)))
               (when (< offset size)
                 (gerbil-scheme-rust/scheme/runtime#gerbil-rs-write-limb
                  output
                  offset
                  1
                  (bitwise-and rest 255))
                 (lp (+ offset 1) (arithmetic-shift rest -8))))
             (if (< value 0) 1 0)))))
    (c-define
     (gerbil-rs-bigint-copy-raw root output size)
     (int64 (pointer unsigned-int8) unsigned-int64)
     int32
     "gerbil_scheme_rust_bigint_copy_raw"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-copy-integer-value
      (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root)
      output
      size))
    (c-define
     (gerbil-rs-bigint-copy-indexed root index output size)
     (int64 int64 (pointer unsigned-int8) unsigned-int64)
     int32
     "gerbil_scheme_rust_bigint_copy_indexed"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-copy-integer-value
      (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value root index)
      output
      size))
    (namespace
     ("gerbil-scheme-rust/scheme/runtime#" gerbil-rs-result-slot-take))
    (c-define
     (gerbil-rs-result-slot-take root index)
     (int64 unsigned-int64)
     int64
     "gerbil_scheme_rust_result_slot_take"
     "extern"
     (with-exception-catcher
      (lambda (_) 0)
      (lambda ()
        (let ((batch (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                      root)))
          (if (and (vector? batch)
                   (= (vector-length batch) 2)
                   (positive? (vector-ref batch 0))
                   (vector? (vector-ref batch 1))
                   (< index (vector-length (vector-ref batch 1))))
              (let ((slot (vector-ref (vector-ref batch 1) index)))
                (if (and (vector? slot)
                         (= (vector-length slot) 2)
                         (eq? (vector-ref slot 0) #t))
                    (let ((result (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
                                   (vector-ref slot 1))))
                      (vector-set! slot 1 #f)
                      (vector-set! slot 0 #f)
                      (vector-set! batch 0 (- (vector-ref batch 0) 1))
                      result)
                    0))
              0)))))
    (namespace
     ("gerbil-scheme-rust/scheme/runtime#"
      gerbil-rs-exchange-enter
      gerbil-rs-exchange-harvest
      gerbil-rs-slot-numeric-copy
      gerbil-rs-exchange-write
      gerbil-rs-exchange-payload))
    (define gerbil-rs-exchange-write
      (c-lambda
       ((pointer int64) unsigned-int64 int64)
       void
       "___arg1[___arg2]=___arg3;"))
    (define (gerbil-rs-exchange-payload kind bytes size tag tag-size)
      (let ((metadata (make-s64vector tag-size))
            (data (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-value
                   kind
                   bytes
                   size)))
        (let copy ((i 0))
          (when (< i tag-size)
            (s64vector-set!
             metadata
             i
             (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at
              tag
              i))
            (copy (+ i 1))))
        (and data (vector metadata data))))
    (c-define
     (gerbil-rs-exchange-enter
      capability
      kind
      bytes
      size
      tag
      tag-size
      header
      length)
     (int64 int32
            (pointer unsigned-int8)
            unsigned-int64
            (pointer int64)
            unsigned-int64
            (pointer int64)
            unsigned-int64)
     int64
     "gerbil_scheme_rust_exchange_enter"
     "extern"
     (set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-call-error #f)
     (with-exception-catcher
      (lambda (failure)
        (set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-call-error
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exception-message
               failure))
        0)
      (lambda ()
        (let ((entry (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                      capability)))
          (if (not (and (vector? entry)
                        (= (vector-length entry) 2)
                        (procedure? (vector-ref entry 0))
                        (<= 2 length 4)))
              0
              (let* ((op (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at
                          header
                          0))
                     (lane (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at
                            header
                            1))
                     (short (if (>= length 3)
                                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at
                                 header
                                 2)
                                0))
                     (units (if (= length 4)
                                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-header-at
                                 header
                                 3)
                                1)))
                (if (not (and (= op 1)
                              (>= lane -1)
                              (<= 0 short 1)
                              (positive? units)))
                    0
                    (let ((payload (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exchange-payload
                                    kind
                                    bytes
                                    size
                                    tag
                                    tag-size)))
                      (if payload
                          ((vector-ref entry 0) payload lane (= short 1) units)
                          0)))))))))
    (c-define
     (gerbil-rs-exchange-harvest capability metadata capacity batch-output)
     (int64 (pointer int64) unsigned-int64 (pointer int64))
     int64
     "gerbil_scheme_rust_exchange_harvest"
     "extern"
     (set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-call-error #f)
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exchange-write
      batch-output
      0
      0)
     (with-exception-catcher
      (lambda (failure)
        (set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-call-error
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exception-message
               failure))
        -1)
      (lambda ()
        (let ((entry (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                      capability)))
          (unless (and
                   (vector? entry)
                   (= (vector-length entry) 2)
                   (procedure? (vector-ref entry 1)))
            (error "invalid exchange capability"))
          (let* ((packet ((vector-ref entry 1)))
                 (receipt (vector-ref packet 0))
                 (statuses (vector-ref packet 1))
                 (batch (vector-ref packet 2))
                 (r (s64vector-length receipt))
                 (n (s64vector-length statuses)))
            (unless (and (<= n 65536) (<= (+ 2 r n) capacity))
              (error "exchange metadata capacity"))
            (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exchange-write
             metadata
             0
             r)
            (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exchange-write
             metadata
             1
             n)
            (let copy ((i 0))
              (when (< i r)
                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exchange-write
                 metadata
                 (+ 2 i)
                 (s64vector-ref receipt i))
                (copy (+ i 1))))
            (let copy ((i 0))
              (when (< i n)
                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exchange-write
                 metadata
                 (+ 2 r i)
                 (s64vector-ref statuses i))
                (copy (+ i 1))))
            (when (positive? n)
              (let ((root (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
                           batch)))
                (unless (positive? root) (error "exchange batch root"))
                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exchange-write
                 batch-output
                 0
                 root)))
            n)))))
    (c-define
     (gerbil-rs-slot-numeric-copy root index kind output capacity)
     (int64 unsigned-int64 int32 (pointer unsigned-int8) unsigned-int64)
     int64
     "gerbil_scheme_rust_result_slot_numeric_copy"
     "extern"
     (with-exception-catcher
      (lambda (_) -1)
      (lambda ()
        (let ((batch (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                      root)))
          (if (not (and (vector? batch)
                        (= (vector-length batch) 2)
                        (positive? (vector-ref batch 0))
                        (vector? (vector-ref batch 1))
                        (< index (vector-length (vector-ref batch 1)))))
              -1
              (let ((slot (vector-ref (vector-ref batch 1) index)))
                (if (not (and (vector? slot)
                              (= (vector-length slot) 2)
                              (eq? (vector-ref slot 0) #t)))
                    -1
                    (let* ((value (vector-ref slot 1))
                           (size (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-span
                                  value
                                  kind
                                  #f
                                  0
                                  0
                                  2)))
                      (cond ((negative? size) -1)
                            ((> size capacity) -2)
                            ((not (= 0
                                     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-span
                                      value
                                      kind
                                      output
                                      0
                                      size
                                      0)))
                             -1)
                            (else
                             (vector-set! slot 1 #f)
                             (vector-set! slot 0 #f)
                             (vector-set! batch 0 (- (vector-ref batch 0) 1))
                             size))))))))))
    (c-define
     (gerbil-rs-scalar-object tag integer real)
     (int32 int64 double)
     int64
     "gerbil_scheme_rust_scalar_object"
     "extern"
     (with-exception-catcher
      (lambda (_) 0)
      (lambda ()
        (let ((store gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!))
          (case tag
            ((0) (store #!void))
            ((1) (store (not (= integer 0))))
            ((2) (if (fixnum? integer) (store integer) 0))
            ((3)
             (if (and (<= 0 integer 1114111) (not (<= 55296 integer 57343)))
                 (store (integer->char integer))
                 0))
            ((4) (store real))
            ((5) (store '()))
            (else 0))))))
    (c-define
     (gerbil-rs-object-kind root index)
     (int64 int64)
     int32
     "gerbil_scheme_rust_object_kind"
     "extern"
     (with-exception-catcher
      (lambda (_) -1)
      (lambda ()
        (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value
                      root
                      index)))
          (cond ((eq? value
                      gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing)
                 -1)
                ((eq? value #!void) 0)
                ((boolean? value) 1)
                ((and (integer? value) (exact? value)) 2)
                ((string? value) 3)
                ((null? value) 4)
                ((pair? value) 5)
                ((vector? value) 6)
                ((table? value) 7)
                ((char? value) 9)
                ((flonum? value) 10)
                ((u8vector? value) 11)
                ((symbol? value) 12)
                ((keyword? value) 13)
                ((procedure? value) 14)
                ((with-exception-catcher
                  (lambda (_) #f)
                  (lambda () (hash-table? value)))
                 8)
                (else 15))))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-rational?
      (lambda (_%value691%_)
        (if (rational? _%value691%_) (exact? _%value691%_) '#f)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-composite-view
      (lambda (_%value653%_ _%kind654%_)
        (let ((_%$e656%_ _%kind654%_))
          (if (or (eq? '0 _%$e656%_) (eq? '1 _%$e656%_))
              (let ((_g9607_ (if (= _%kind654%_ '0)
                                 (values gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-rational?
                                         numerator
                                         denominator)
                                 (values number? real-part imag-part))))
                (begin
                  #!void
                  (let ((_%valid?662%_
                         (let ()
                           (declare (not safe))
                           (##values-ref _g9607_ 0)))
                        (_%left663%_
                         (let ()
                           (declare (not safe))
                           (##values-ref _g9607_ 1)))
                        (_%right664%_
                         (let ()
                           (declare (not safe))
                           (##values-ref _g9607_ 2))))
                    (let* ((_%items666%_
                            (if (vector? _%value653%_)
                                _%value653%_
                                (vector _%value653%_)))
                           (_%count668%_
                            (let ()
                              (declare (not safe))
                              (##vector-length _%items666%_)))
                           (_%out670%_ (make-vector (* '2 _%count668%_))))
                      (let _%loop673%_ ((_%index675%_ '0))
                        (if (= _%index675%_ _%count668%_)
                            _%out670%_
                            (let ((_%item677%_
                                   (vector-ref _%items666%_ _%index675%_)))
                              (if (_%valid?662%_ _%item677%_)
                                  (begin
                                    (vector-set!
                                     _%out670%_
                                     (* '2 _%index675%_)
                                     (_%left663%_ _%item677%_))
                                    (vector-set!
                                     _%out670%_
                                     (+ '1 (* '2 _%index675%_))
                                     (_%right664%_ _%item677%_))
                                    (_%loop673%_ (+ _%index675%_ '1)))
                                  '#f))))))))
              (if (eq? '2 _%$e656%_)
                  (if (or (table? _%value653%_)
                          (let ()
                            (declare (not safe))
                            (##structure-direct-instance-of?
                             _%value653%_
                             'HashTable::t)))
                      (let ((_%out682%_
                             (make-vector
                              (* '2
                                 (if (table? _%value653%_)
                                     (let ()
                                       (declare (not safe))
                                       (##table-length _%value653%_))
                                     (let ()
                                       (declare (not safe))
                                       (hash-length _%value653%_))))))
                            (_%index683%_ '0))
                        ((if (table? _%value653%_)
                             table-for-each
                             (lambda (_%visit685%_ _%table686%_)
                               (let ()
                                 (declare (not safe))
                                 (hash-for-each _%visit685%_ _%table686%_))))
                         (lambda (_%key688%_ _%item689%_)
                           (vector-set! _%out682%_ _%index683%_ _%key688%_)
                           (vector-set!
                            _%out682%_
                            (+ _%index683%_ '1)
                            _%item689%_)
                           (set! _%index683%_ (+ _%index683%_ '2)))
                         _%value653%_)
                        _%out682%_)
                      '#f)
                  '#f)))))
    (namespace
     ("gerbil-scheme-rust/scheme/runtime#"
      gerbil-rs-composite-create
      gerbil-rs-composite-snapshot
      gerbil-rs-table-create
      gerbil-rs-snapshot-length))
    (define gerbil-rs-snapshot-length
      (c-lambda
       ((pointer unsigned-int64) unsigned-int64)
       void
       "___arg1[0]=___arg2;"))
    (c-define
     (gerbil-rs-composite-create roots count kind)
     ((pointer int64) unsigned-int64 int32)
     int64
     "gerbil_scheme_rust_composite_create"
     "extern"
     (with-exception-catcher
      (lambda (_exception) 0)
      (lambda ()
        (if (not (and (fixnum? count)
                      (fixnum? (* 2 count))
                      (or (= kind 0) (= kind 1))))
            0
            (let ((out (make-vector count)))
              (let loop ((index 0))
                (if (= index count)
                    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
                     out)
                    (let* ((left (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                                  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at
                                   roots
                                   (* 2 index))))
                           (right (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                                   (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at
                                    roots
                                    (+ 1 (* 2 index)))))
                           (value (case kind
                                    ((0)
                                     (and (integer? left)
                                          (exact? left)
                                          (integer? right)
                                          (exact? right)
                                          (not (= right 0))
                                          (/ left right)))
                                    ((1)
                                     (and (real? left)
                                          (real? right)
                                          (make-rectangular left right))))))
                      (if (eq? value #f)
                          0
                          (begin
                            (vector-set! out index value)
                            (loop (+ index 1))))))))))))
    (c-define
     (gerbil-rs-table-create roots count equality)
     ((pointer int64) unsigned-int64 int32)
     int64
     "gerbil_scheme_rust_table_create"
     "extern"
     (with-exception-catcher
      (lambda (_exception) 0)
      (lambda ()
        (let ((test (case equality
                      ((0 3) eq?)
                      ((1 4) eqv?)
                      ((2 5) equal?)
                      (else #f)))
              (gerbil? (<= 3 equality 5)))
          (if (not (and test (fixnum? count) (fixnum? (* 2 count))))
              0
              (let ((out (if gerbil?
                             (case equality
                               ((3) (make-hash-table-eq))
                               ((4) (make-hash-table-eqv))
                               (else (make-hash-table)))
                             (make-table
                              test:
                              test
                              weak-keys:
                              #f
                              weak-values:
                              #f))))
                (let loop ((index 0))
                  (if (= index count)
                      (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
                       out)
                      (let ((key (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                                  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at
                                   roots
                                   (* 2 index))))
                            (value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                                    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at
                                     roots
                                     (+ 1 (* 2 index))))))
                        (if (or (eq? key
                                     gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing)
                                (eq? value
                                     gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing))
                            0
                            (begin
                              (if gerbil?
                                  (hash-put! out key value)
                                  (table-set! out key value))
                              (loop (+ index 1)))))))))))))
    (c-define
     (gerbil-rs-composite-snapshot root index kind length)
     (int64 int64 int32 (pointer unsigned-int64))
     int64
     "gerbil_scheme_rust_composite_snapshot"
     "extern"
     (with-exception-catcher
      (lambda (_exception) 0)
      (lambda ()
        (let* ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value
                       root
                       index))
               (snapshot
                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-composite-view
                 value
                 kind)))
          (if (not snapshot)
              0
              (let ((token (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
                            snapshot)))
                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-snapshot-length
                 length
                 (vector-length snapshot))
                token))))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-bytestring-delimiter
      (lambda (_%code648%_)
        (if (= _%code648%_ '-1)
            '#f
            (if (and (>= _%code648%_ '0)
                     (<= _%code648%_ '1114111)
                     (not (<= '55296 _%code648%_ '57343)))
                (integer->char _%code648%_)
                '#!void))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-upper-hex-digits
      '"0123456789ABCDEF")
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-upper-hex-digit
      (lambda (_%value646%_)
        (string-ref
         gerbil-scheme-rust/scheme/runtime#gerbil-rs-upper-hex-digits
         _%value646%_)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-hex-value
      (lambda (_%character640%_)
        (if (char<=? '#\0 _%character640%_ '#\9)
            (- (char->integer _%character640%_)
               (let () (declare (not safe)) (##char->integer '#\0)))
            (if (char<=? '#\A _%character640%_ '#\F)
                (+ '10
                   (- (char->integer _%character640%_)
                      (let () (declare (not safe)) (##char->integer '#\A))))
                (if (char<=? '#\a _%character640%_ '#\f)
                    (+ '10
                       (- (char->integer _%character640%_)
                          (let ()
                            (declare (not safe))
                            (##char->integer '#\a))))
                    '-1)))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-u8vector->bytestring
      (lambda (_%value620%_ _%delimiter621%_)
        (if (and (u8vector? _%value620%_)
                 (or (not _%delimiter621%_) (char? _%delimiter621%_)))
            (let* ((_%length626%_ (u8vector-length _%value620%_))
                   (_%delimiter-count628%_
                    (if (and (> _%length626%_ '0) _%delimiter621%_)
                        (- _%length626%_ '1)
                        '0))
                   (_%bytestring630%_
                    (make-string
                     (+ (* _%length626%_ '2) _%delimiter-count628%_))))
              (let _%lp633%_ ((_%index635%_ '0) (_%offset636%_ '0))
                (if (< _%index635%_ _%length626%_)
                    (let ((_%byte638%_
                           (u8vector-ref _%value620%_ _%index635%_)))
                      (if (and (> _%index635%_ '0) _%delimiter621%_)
                          (begin
                            (string-set!
                             _%bytestring630%_
                             _%offset636%_
                             _%delimiter621%_)
                            (set! _%offset636%_ (+ _%offset636%_ '1)))
                          '#!void)
                      (string-set!
                       _%bytestring630%_
                       _%offset636%_
                       (gerbil-scheme-rust/scheme/runtime#gerbil-rs-upper-hex-digit
                        (arithmetic-shift _%byte638%_ '-4)))
                      (string-set!
                       _%bytestring630%_
                       (+ _%offset636%_ '1)
                       (gerbil-scheme-rust/scheme/runtime#gerbil-rs-upper-hex-digit
                        (bitwise-and _%byte638%_ '15)))
                      (_%lp633%_ (+ _%index635%_ '1) (+ _%offset636%_ '2)))
                    '#!void))
              _%bytestring630%_)
            '#f)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-hex-byte
      (lambda (_%bytestring614%_ _%offset615%_)
        (let ((_%high617%_
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-hex-value
                (string-ref _%bytestring614%_ _%offset615%_)))
              (_%low618%_
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-hex-value
                (string-ref _%bytestring614%_ (+ _%offset615%_ '1)))))
          (if (and (>= _%high617%_ '0) (>= _%low618%_ '0))
              (+ (arithmetic-shift _%high617%_ '4) _%low618%_)
              '-1))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-bytestring->u8vector
      (lambda (_%bytestring579%_ _%delimiter580%_)
        (if (and (string? _%bytestring579%_)
                 (or (not _%delimiter580%_) (char? _%delimiter580%_)))
            (let* ((_%length585%_ (string-length _%bytestring579%_))
                   (_%valid-length?590%_
                    (if _%delimiter580%_
                        (let ((_%$e587%_ (zero? _%length585%_)))
                          (if _%$e587%_
                              _%$e587%_
                              (zero? (modulo (+ _%length585%_ '1) '3))))
                        (zero? (modulo _%length585%_ '2))))
                   (_%byte-count592%_
                    (if _%delimiter580%_
                        (quotient (+ _%length585%_ '1) '3)
                        (quotient _%length585%_ '2)))
                   (_%value594%_
                    (if _%valid-length?590%_
                        (make-u8vector _%byte-count592%_)
                        '#f)))
              (if _%value594%_
                  (let _%lp597%_ ((_%index599%_ '0))
                    (if (< _%index599%_ _%byte-count592%_)
                        (let* ((_%offset601%_
                                (if _%delimiter580%_
                                    (* _%index599%_ '3)
                                    (* _%index599%_ '2)))
                               (_%delimiter-valid?609%_
                                (let ((_%$e603%_ (not _%delimiter580%_)))
                                  (if _%$e603%_
                                      _%$e603%_
                                      (let ((_%$e606%_ (zero? _%index599%_)))
                                        (if _%$e606%_
                                            _%$e606%_
                                            (eq? _%delimiter580%_
                                                 (string-ref
                                                  _%bytestring579%_
                                                  (- _%offset601%_ '1))))))))
                               (_%byte611%_
                                (if _%delimiter-valid?609%_
                                    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-hex-byte
                                     _%bytestring579%_
                                     _%offset601%_)
                                    '#f)))
                          (if (and _%byte611%_ (>= _%byte611%_ '0))
                              (begin
                                (u8vector-set!
                                 _%value594%_
                                 _%index599%_
                                 _%byte611%_)
                                (_%lp597%_ (+ _%index599%_ '1)))
                              '#f))
                        _%value594%_))
                  '#f))
            '#f)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-bytevector->bytestring-root
      (lambda (_%value574%_ _%delimiter-code575%_)
        (let ((_%bytestring577%_
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-u8vector->bytestring
                _%value574%_
                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-bytestring-delimiter
                 _%delimiter-code575%_))))
          (if _%bytestring577%_
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
               _%bytestring577%_)
              '0))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-bytestring->bytevector-root
      (lambda (_%bytestring569%_ _%delimiter-code570%_)
        (let ((_%bytevector572%_
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-bytestring->u8vector
                _%bytestring569%_
                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-bytestring-delimiter
                 _%delimiter-code570%_))))
          (if _%bytevector572%_
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
               _%bytevector572%_)
              '0))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-u8vector->uint
      (lambda (_%value560%_ _%byte-order561%_ _%size562%_)
        (let _%lp564%_ ((_%index566%_
                         (if (= _%byte-order561%_ '0) '0 (- _%size562%_ '1)))
                        (_%result567%_ '0))
          (if (if (= _%byte-order561%_ '0)
                  (< _%index566%_ _%size562%_)
                  (>= _%index566%_ '0))
              (_%lp564%_
               (if (= _%byte-order561%_ '0)
                   (+ _%index566%_ '1)
                   (- _%index566%_ '1))
               (bitwise-ior
                (arithmetic-shift _%result567%_ '8)
                (u8vector-ref _%value560%_ _%index566%_)))
              _%result567%_))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-u8vector->sint
      (lambda (_%value549%_ _%byte-order550%_ _%size551%_)
        (if (zero? _%size551%_)
            '0
            (let* ((_%uint553%_
                    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-u8vector->uint
                     _%value549%_
                     _%byte-order550%_
                     _%size551%_))
                   (_%bits555%_ (* _%size551%_ '8))
                   (_%sign-bit557%_ (arithmetic-shift '1 (- _%bits555%_ '1))))
              (if (zero? (bitwise-and _%uint553%_ _%sign-bit557%_))
                  _%uint553%_
                  (- _%uint553%_ (arithmetic-shift '1 _%bits555%_)))))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-uint->u8vector
      (lambda (_%uint538%_ _%byte-order539%_ _%size540%_)
        (let ((_%value542%_ (make-u8vector _%size540%_)))
          (let _%lp544%_ ((_%index546%_ '0) (_%rest547%_ _%uint538%_))
            (if (< _%index546%_ _%size540%_)
                (begin
                  (u8vector-set!
                   _%value542%_
                   (if (= _%byte-order539%_ '0)
                       (- _%size540%_ _%index546%_ '1)
                       _%index546%_)
                   (bitwise-and _%rest547%_ '255))
                  (_%lp544%_
                   (+ _%index546%_ '1)
                   (arithmetic-shift _%rest547%_ '-8)))
                '#!void))
          _%value542%_)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-sint->u8vector
      (lambda (_%sint534%_ _%byte-order535%_ _%size536%_)
        (gerbil-scheme-rust/scheme/runtime#gerbil-rs-uint->u8vector
         (if (< _%sint534%_ '0)
             (+ _%sint534%_ (arithmetic-shift '1 (* _%size536%_ '8)))
             _%sint534%_)
         _%byte-order535%_
         _%size536%_)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-bytevector->uint
      (lambda (_%root-id528%_ _%byte-order529%_ _%size530%_)
        (let ((_%value532%_
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                _%root-id528%_)))
          (if (u8vector? _%value532%_)
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-u8vector->uint
               _%value532%_
               _%byte-order529%_
               _%size530%_)
              '0))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-bytevector->sint
      (lambda (_%root-id522%_ _%byte-order523%_ _%size524%_)
        (let ((_%value526%_
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                _%root-id522%_)))
          (if (u8vector? _%value526%_)
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-u8vector->sint
               _%value526%_
               _%byte-order523%_
               _%size524%_)
              '0))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-uint->bytevector-root
      (lambda (_%uint518%_ _%byte-order519%_ _%size520%_)
        (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
         (gerbil-scheme-rust/scheme/runtime#gerbil-rs-uint->u8vector
          _%uint518%_
          _%byte-order519%_
          _%size520%_))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-sint->bytevector-root
      (lambda (_%sint514%_ _%byte-order515%_ _%size516%_)
        (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
         (gerbil-scheme-rust/scheme/runtime#gerbil-rs-sint->u8vector
          _%sint514%_
          _%byte-order515%_
          _%size516%_))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-i64-min
      (- (arithmetic-shift '1 '63)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-i64-max
      (- (arithmetic-shift '1 '63) '1))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-u64-max
      (- (arithmetic-shift '1 '64) '1))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer?
      (lambda (_%value512%_)
        (if (integer? _%value512%_) (exact? _%value512%_) '#f)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer-fits-i64?
      (lambda (_%value510%_)
        (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer?
             _%value510%_)
            (<= gerbil-scheme-rust/scheme/runtime#gerbil-rs-i64-min
                _%value510%_
                gerbil-scheme-rust/scheme/runtime#gerbil-rs-i64-max)
            '#f)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer-fits-u64?
      (lambda (_%value508%_)
        (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer?
             _%value508%_)
            (<= '0
                _%value508%_
                gerbil-scheme-rust/scheme/runtime#gerbil-rs-u64-max)
            '#f)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-i64->exact-integer-root
      (lambda (_%value506%_)
        (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
         _%value506%_)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-u64->exact-integer-root
      (lambda (_%value504%_)
        (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
         _%value504%_)))
    (namespace
     ("gerbil-scheme-rust/scheme/runtime#"
      gerbil-rs-abi-version-native
      gerbil-rs-add-i64-native
      gerbil-rs-is-even-i64-native
      gerbil-rs-compare-i64-native
      gerbil-rs-scheme-null-value-raw
      gerbil-rs-fixture-void-raw
      gerbil-rs-fixture-pair-raw
      gerbil-rs-fixture-proper-list-raw
      gerbil-rs-fixture-improper-list-raw
      gerbil-rs-fixture-true-raw
      gerbil-rs-fixture-false-raw
      gerbil-rs-fixture-fixnum-raw
      gerbil-rs-fixture-exact-integer-large-positive-raw
      gerbil-rs-fixture-exact-integer-large-negative-raw
      gerbil-rs-fixture-char-ascii-raw
      gerbil-rs-fixture-char-bmp-raw
      gerbil-rs-fixture-char-non-bmp-raw
      gerbil-rs-fixture-flonum-finite-raw
      gerbil-rs-fixture-flonum-nan-raw
      gerbil-rs-fixture-flonum-pos-inf-raw
      gerbil-rs-fixture-flonum-neg-inf-raw
      gerbil-rs-fixture-flonum-neg-zero-raw
      gerbil-rs-fixture-bytevector-raw
      gerbil-rs-scheme-object-null?-raw
      gerbil-rs-scheme-object-void?-raw
      gerbil-rs-scheme-object-bytevector?-raw
      gerbil-rs-scheme-object-pair?-raw
      gerbil-rs-scheme-object-list?-raw
      gerbil-rs-scheme-object-boolean?-raw
      gerbil-rs-scheme-object-boolean-value-raw
      gerbil-rs-scheme-object-fixnum?-raw
      gerbil-rs-scheme-object-fixnum-value-raw
      gerbil-rs-scheme-object-exact-integer?-raw
      gerbil-rs-scheme-object-exact-integer-fits-i64?-raw
      gerbil-rs-scheme-object-exact-integer-fits-u64?-raw
      gerbil-rs-scheme-object-exact-integer-i64-value-raw
      gerbil-rs-scheme-object-exact-integer-u64-value-raw
      gerbil-rs-scheme-object-char?-raw
      gerbil-rs-scheme-object-char-value-raw
      gerbil-rs-scheme-object-flonum?-raw
      gerbil-rs-scheme-object-flonum-value-raw
      gerbil-rs-scheme-object-bytevector-length-raw
      gerbil-rs-scheme-object-bytevector-u8-ref-raw
      gerbil-rs-bytevector->bytestring-root-raw
      gerbil-rs-bytestring->bytevector-root-raw
      gerbil-rs-bytevector->uint-raw
      gerbil-rs-bytevector->sint-raw
      gerbil-rs-root-bytevector->uint-raw
      gerbil-rs-root-bytevector->sint-raw
      gerbil-rs-uint->bytevector-root-raw
      gerbil-rs-sint->bytevector-root-raw
      gerbil-rs-i64->exact-integer-root-raw
      gerbil-rs-u64->exact-integer-root-raw
      gerbil-rs-root-exact-integer?-raw
      gerbil-rs-root-exact-integer-fits-i64?-raw
      gerbil-rs-root-exact-integer-fits-u64?-raw
      gerbil-rs-root-exact-integer-i64-value-raw
      gerbil-rs-root-exact-integer-u64-value-raw
      gerbil-rs-root-string-length-raw
      gerbil-rs-root-string-char-ref-raw
      gerbil-rs-root-string->utf8-raw
      gerbil-rs-root-utf8->string-raw
      gerbil-rs-root-bytevector-length-raw
      gerbil-rs-root-bytevector-u8-ref-raw
      gerbil-rs-copy-u8vector-c
      gerbil-rs-fill-u8vector-c
      gerbil-rs-bytes->bytevector-root-raw
      gerbil-rs-root-bytevector-copy-raw
      gerbil-rs-scheme-object-bytevector-copy-raw
      gerbil-rs-root-release-raw
      gerbil-rs-scheme-object-pair-car-raw
      gerbil-rs-scheme-object-pair-cdr-raw))
    (c-declare
     "#ifndef ___HAVE_FFI_U8VECTOR\n#define ___HAVE_FFI_U8VECTOR\n#define U8_DATA(obj) ___CAST (___U8*, ___BODY_AS (obj, ___tSUBTYPED))\n#define U8_LEN(obj) ___U8VECTORSIZE(obj)\n#endif")
    (c-declare
     "#if defined(__has_builtin)\n#if __has_builtin(__builtin_memcpy_inline)\n#define GERBIL_RS_COPY_BLOCK(d, s) __builtin_memcpy_inline(d, s, 64)\n#endif\n#endif\n#if !defined(GERBIL_RS_COPY_BLOCK) && defined(__GNUC__)\n/* GCC's constant-size builtin is qualified by the canonical module link,\n   unlike the guaranteed-inline Clang intrinsic. Do not change linker policy\n   if a target emits an unresolved memcpy call. */\n#define GERBIL_RS_COPY_BLOCK(d, s) __builtin_memcpy(d, s, 64)\n#endif\nstatic void gerbil_rs_copy_bytes(___U8 *destination, const ___U8 *source,\n                                 ___U64 length) {\n#if defined(GERBIL_RS_COPY_BLOCK)\n  /* Local bulk-copy qualification does not imply concurrent admission.\n     Clang guarantees inlining; GCC must pass actual module qualification. */\n  while (length >= 64) {\n    GERBIL_RS_COPY_BLOCK(destination, source);\n    destination += 64;\n    source += 64;\n    length -= 64;\n  }\n#endif\n  for (___U64 index = 0; index < length; ++index) {\n    destination[index] = source[index];\n  }\n}")
    (define gerbil-rs-copy-u8vector-c
      (c-lambda
       (scheme-object (pointer unsigned-int8) unsigned-int64)
       int64
       "if (___arg3 > U8_LEN(___arg1) || (___arg3 > 0 && ___arg2 == NULL)) {\n  ___return(-1);\n}\nconst ___U8 *source = U8_DATA(___arg1);\ngerbil_rs_copy_bytes(___arg2, source, ___arg3);\n___return((___S64)___arg3);"))
    (define gerbil-rs-fill-u8vector-c
      (c-lambda
       (scheme-object (pointer unsigned-int8) unsigned-int64)
       int64
       "if (___arg3 != U8_LEN(___arg1) || (___arg3 > 0 && ___arg2 == NULL)) {\n  ___return(-1);\n}\n___U8 *destination = U8_DATA(___arg1);\ngerbil_rs_copy_bytes(destination, ___arg2, ___arg3);\n___return((___S64)___arg3);"))
    (c-define
     (gerbil-rs-bytes->bytevector-root-raw source length)
     ((pointer unsigned-int8) unsigned-int64)
     int64
     "gerbil_scheme_rust_bytes_to_bytevector_root_raw"
     "extern"
     (let ((value (make-u8vector length)))
       (if (= length
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-fill-u8vector-c
               value
               source
               length))
           (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-bytevector value)
           0)))
    (c-define
     (gerbil-rs-abi-version-native)
     ()
     unsigned-int32
     "gerbil_scheme_rust_abi_version"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-abi-version))
    (c-define
     (gerbil-rs-add-i64-native left right)
     (int64 int64)
     int64
     "gerbil_scheme_rust_add_i64"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-add-i64 left right))
    (c-define
     (gerbil-rs-is-even-i64-native value)
     (int64)
     int32
     "gerbil_scheme_rust_is_even_i64"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-is-even-i64 value))
    (c-define
     (gerbil-rs-compare-i64-native left right)
     (int64 int64)
     int32
     "gerbil_scheme_rust_compare_i64"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-compare-i64 left right))
    (c-define
     (gerbil-rs-scheme-null-value-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_scheme_null_value_raw"
     "extern"
     '())
    (c-define
     (gerbil-rs-fixture-void-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_void_raw"
     "extern"
     #!void)
    (c-define
     (gerbil-rs-fixture-pair-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_pair_raw"
     "extern"
     (cons 1 2))
    (c-define
     (gerbil-rs-fixture-proper-list-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_proper_list_raw"
     "extern"
     (list 1 2))
    (c-define
     (gerbil-rs-fixture-improper-list-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_improper_list_raw"
     "extern"
     (cons 1 2))
    (c-define
     (gerbil-rs-fixture-true-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_true_raw"
     "extern"
     #t)
    (c-define
     (gerbil-rs-fixture-false-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_false_raw"
     "extern"
     #f)
    (c-define
     (gerbil-rs-fixture-fixnum-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_fixnum_raw"
     "extern"
     42)
    (c-define
     (gerbil-rs-fixture-exact-integer-large-positive-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_exact_integer_large_positive_raw"
     "extern"
     (arithmetic-shift 1 80))
    (c-define
     (gerbil-rs-fixture-exact-integer-large-negative-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_exact_integer_large_negative_raw"
     "extern"
     (- (arithmetic-shift 1 80)))
    (c-define
     (gerbil-rs-fixture-char-ascii-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_char_ascii_raw"
     "extern"
     #\A)
    (c-define
     (gerbil-rs-fixture-char-bmp-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_char_bmp_raw"
     "extern"
     (integer->char 955))
    (c-define
     (gerbil-rs-fixture-char-non-bmp-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_char_non_bmp_raw"
     "extern"
     (integer->char 128578))
    (c-define
     (gerbil-rs-fixture-flonum-finite-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_flonum_finite_raw"
     "extern"
     42.5)
    (c-define
     (gerbil-rs-fixture-flonum-nan-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_flonum_nan_raw"
     "extern"
     +nan.0)
    (c-define
     (gerbil-rs-fixture-flonum-pos-inf-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_flonum_pos_inf_raw"
     "extern"
     +inf.0)
    (c-define
     (gerbil-rs-fixture-flonum-neg-inf-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_flonum_neg_inf_raw"
     "extern"
     -inf.0)
    (c-define
     (gerbil-rs-fixture-flonum-neg-zero-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_flonum_neg_zero_raw"
     "extern"
     -0.)
    (c-define
     (gerbil-rs-fixture-bytevector-raw)
     ()
     scheme-object
     "gerbil_scheme_rust_fixture_bytevector_raw"
     "extern"
     #u8(255 127 11 1 0))
    (c-define
     (gerbil-rs-scheme-object-null?-raw value)
     (scheme-object)
     int32
     "gerbil_scheme_rust_scheme_object_is_null_raw"
     "extern"
     (if (null? value) 1 0))
    (c-define
     (gerbil-rs-scheme-object-void?-raw value)
     (scheme-object)
     int32
     "gerbil_scheme_rust_scheme_object_is_void_raw"
     "extern"
     (if (eq? value #!void) 1 0))
    (c-define
     (gerbil-rs-scheme-object-bytevector?-raw value)
     (scheme-object)
     int32
     "gerbil_scheme_rust_scheme_object_is_bytevector_raw"
     "extern"
     (if (u8vector? value) 1 0))
    (c-define
     (gerbil-rs-scheme-object-pair?-raw value)
     (scheme-object)
     int32
     "gerbil_scheme_rust_scheme_object_is_pair_raw"
     "extern"
     (if (pair? value) 1 0))
    (c-define
     (gerbil-rs-scheme-object-list?-raw value)
     (scheme-object)
     int32
     "gerbil_scheme_rust_scheme_object_is_list_raw"
     "extern"
     (if (list? value) 1 0))
    (c-define
     (gerbil-rs-scheme-object-boolean?-raw value)
     (scheme-object)
     int32
     "gerbil_scheme_rust_scheme_object_is_boolean_raw"
     "extern"
     (if (boolean? value) 1 0))
    (c-define
     (gerbil-rs-scheme-object-boolean-value-raw value)
     (scheme-object)
     int32
     "gerbil_scheme_rust_scheme_object_boolean_value_raw"
     "extern"
     (if value 1 0))
    (c-define
     (gerbil-rs-scheme-object-fixnum?-raw value)
     (scheme-object)
     int32
     "gerbil_scheme_rust_scheme_object_is_fixnum_raw"
     "extern"
     (if (fixnum? value) 1 0))
    (c-define
     (gerbil-rs-scheme-object-fixnum-value-raw value)
     (scheme-object)
     long
     "gerbil_scheme_rust_scheme_object_fixnum_value_raw"
     "extern"
     value)
    (c-define
     (gerbil-rs-scheme-object-exact-integer?-raw value)
     (scheme-object)
     int32
     "gerbil_scheme_rust_scheme_object_is_exact_integer_raw"
     "extern"
     (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer? value)
         1
         0))
    (c-define
     (gerbil-rs-scheme-object-exact-integer-fits-i64?-raw value)
     (scheme-object)
     int32
     "gerbil_scheme_rust_scheme_object_exact_integer_fits_i64_raw"
     "extern"
     (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer-fits-i64?
          value)
         1
         0))
    (c-define
     (gerbil-rs-scheme-object-exact-integer-fits-u64?-raw value)
     (scheme-object)
     int32
     "gerbil_scheme_rust_scheme_object_exact_integer_fits_u64_raw"
     "extern"
     (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer-fits-u64?
          value)
         1
         0))
    (c-define
     (gerbil-rs-scheme-object-exact-integer-i64-value-raw value)
     (scheme-object)
     int64
     "gerbil_scheme_rust_scheme_object_exact_integer_i64_value_raw"
     "extern"
     value)
    (c-define
     (gerbil-rs-scheme-object-exact-integer-u64-value-raw value)
     (scheme-object)
     unsigned-int64
     "gerbil_scheme_rust_scheme_object_exact_integer_u64_value_raw"
     "extern"
     value)
    (c-define
     (gerbil-rs-scheme-object-char?-raw value)
     (scheme-object)
     int32
     "gerbil_scheme_rust_scheme_object_is_char_raw"
     "extern"
     (if (char? value) 1 0))
    (c-define
     (gerbil-rs-scheme-object-char-value-raw value)
     (scheme-object)
     int32
     "gerbil_scheme_rust_scheme_object_char_value_raw"
     "extern"
     (char->integer value))
    (c-define
     (gerbil-rs-scheme-object-flonum?-raw value)
     (scheme-object)
     int32
     "gerbil_scheme_rust_scheme_object_is_flonum_raw"
     "extern"
     (if (flonum? value) 1 0))
    (c-define
     (gerbil-rs-scheme-object-flonum-value-raw value)
     (scheme-object)
     double
     "gerbil_scheme_rust_scheme_object_flonum_value_raw"
     "extern"
     value)
    (c-define
     (gerbil-rs-scheme-object-bytevector-length-raw value)
     (scheme-object)
     int64
     "gerbil_scheme_rust_scheme_object_bytevector_length_raw"
     "extern"
     (if (u8vector? value) (u8vector-length value) -1))
    (c-define
     (gerbil-rs-scheme-object-bytevector-u8-ref-raw value index)
     (scheme-object int64)
     int32
     "gerbil_scheme_rust_scheme_object_bytevector_u8_ref_raw"
     "extern"
     (if (and (u8vector? value) (>= index 0) (< index (u8vector-length value)))
         (u8vector-ref value index)
         -1))
    (c-define
     (gerbil-rs-bytevector->bytestring-root-raw value delimiter-code)
     (scheme-object int32)
     int64
     "gerbil_scheme_rust_bytevector_to_bytestring_root_raw"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-bytevector->bytestring-root
      value
      delimiter-code))
    (c-define
     (gerbil-rs-bytestring->bytevector-root-raw bytestring delimiter-code)
     (char-string int32)
     int64
     "gerbil_scheme_rust_bytestring_to_bytevector_root_raw"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-bytestring->bytevector-root
      bytestring
      delimiter-code))
    (c-define
     (gerbil-rs-bytevector->uint-raw value byte-order size)
     (scheme-object int32 int64)
     unsigned-int64
     "gerbil_scheme_rust_bytevector_to_uint_raw"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-u8vector->uint
      value
      byte-order
      size))
    (c-define
     (gerbil-rs-bytevector->sint-raw value byte-order size)
     (scheme-object int32 int64)
     int64
     "gerbil_scheme_rust_bytevector_to_sint_raw"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-u8vector->sint
      value
      byte-order
      size))
    (c-define
     (gerbil-rs-root-bytevector->uint-raw root-id byte-order size)
     (int64 int32 int64)
     unsigned-int64
     "gerbil_scheme_rust_root_bytevector_to_uint_raw"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-bytevector->uint
      root-id
      byte-order
      size))
    (c-define
     (gerbil-rs-root-bytevector->sint-raw root-id byte-order size)
     (int64 int32 int64)
     int64
     "gerbil_scheme_rust_root_bytevector_to_sint_raw"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-bytevector->sint
      root-id
      byte-order
      size))
    (c-define
     (gerbil-rs-uint->bytevector-root-raw uint byte-order size)
     (unsigned-int64 int32 int64)
     int64
     "gerbil_scheme_rust_uint_to_bytevector_root_raw"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-uint->bytevector-root
      uint
      byte-order
      size))
    (c-define
     (gerbil-rs-sint->bytevector-root-raw sint byte-order size)
     (int64 int32 int64)
     int64
     "gerbil_scheme_rust_sint_to_bytevector_root_raw"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-sint->bytevector-root
      sint
      byte-order
      size))
    (c-define
     (gerbil-rs-i64->exact-integer-root-raw value)
     (int64)
     int64
     "gerbil_scheme_rust_i64_to_exact_integer_root_raw"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-i64->exact-integer-root
      value))
    (c-define
     (gerbil-rs-u64->exact-integer-root-raw value)
     (unsigned-int64)
     int64
     "gerbil_scheme_rust_u64_to_exact_integer_root_raw"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-u64->exact-integer-root
      value))
    (c-define
     (gerbil-rs-root-exact-integer?-raw root-id)
     (int64)
     int32
     "gerbil_scheme_rust_root_is_exact_integer_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer? value)
           1
           0)))
    (c-define
     (gerbil-rs-root-exact-integer-fits-i64?-raw root-id)
     (int64)
     int32
     "gerbil_scheme_rust_root_exact_integer_fits_i64_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer-fits-i64?
            value)
           1
           0)))
    (c-define
     (gerbil-rs-root-exact-integer-fits-u64?-raw root-id)
     (int64)
     int32
     "gerbil_scheme_rust_root_exact_integer_fits_u64_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer-fits-u64?
            value)
           1
           0)))
    (c-define
     (gerbil-rs-root-exact-integer-i64-value-raw root-id)
     (int64)
     int64
     "gerbil_scheme_rust_root_exact_integer_i64_value_raw"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root-id))
    (c-define
     (gerbil-rs-root-exact-integer-u64-value-raw root-id)
     (int64)
     unsigned-int64
     "gerbil_scheme_rust_root_exact_integer_u64_value_raw"
     "extern"
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root-id))
    (c-define
     (gerbil-rs-take-call-error-raw)
     ()
     int64
     "gerbil_scheme_rust_take_call_error_raw"
     "extern"
     (let ((message gerbil-scheme-rust/scheme/runtime#gerbil-rs-call-error))
       (set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-call-error #f)
       (if message
           (with-exception-catcher
            (lambda (_) 0)
            (lambda ()
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-bytevector
               (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8
                message))))
           0)))
    (c-define
     (gerbil-rs-root-string->utf8-raw root-id)
     (int64)
     int64
     "gerbil_scheme_rust_root_string_to_utf8_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (string? value)
           (with-exception-catcher
            (lambda (_) 0)
            (lambda ()
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
               (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8 value))))
           0)))
    (c-define
     (gerbil-rs-root-string-encode-into-raw root-id pointer capacity)
     (int64 (pointer void) unsigned-int64)
     int64
     "gerbil_scheme_rust_root_string_encode_into_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (string? value)
           (with-exception-catcher
            (lambda (_) -1)
            (lambda ()
              (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-into
               value
               pointer
               capacity)))
           -1)))
    (c-define
     (gerbil-rs-root-utf8->string-raw root-id)
     (int64)
     int64
     "gerbil_scheme_rust_root_utf8_to_string_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (u8vector? value)
           (with-exception-catcher
            (lambda (_) 0)
            (lambda ()
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
               (utf8->string value))))
           0)))
    (c-define
     (gerbil-rs-root-string-length-raw root-id)
     (int64)
     int64
     "gerbil_scheme_rust_root_string_length_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (string? value) (string-length value) -1)))
    (c-define
     (gerbil-rs-root-string-char-ref-raw root-id index)
     (int64 int64)
     int32
     "gerbil_scheme_rust_root_string_char_ref_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (and (string? value) (>= index 0) (< index (string-length value)))
           (char->integer (string-ref value index))
           -1)))
    (c-define
     (gerbil-rs-root-bytevector-length-raw root-id)
     (int64)
     int64
     "gerbil_scheme_rust_root_bytevector_length_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (u8vector? value) (u8vector-length value) -1)))
    (c-define
     (gerbil-rs-root-bytevector-u8-ref-raw root-id index)
     (int64 int64)
     int32
     "gerbil_scheme_rust_root_bytevector_u8_ref_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (and (u8vector? value)
                (>= index 0)
                (< index (u8vector-length value)))
           (u8vector-ref value index)
           -1)))
    (c-define
     (gerbil-rs-root-bytevector-copy-raw root-id destination length)
     (int64 (pointer unsigned-int8) unsigned-int64)
     int64
     "gerbil_scheme_rust_root_bytevector_copy_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (and (u8vector? value) (= length (u8vector-length value)))
           (gerbil-scheme-rust/scheme/runtime#gerbil-rs-copy-u8vector-c
            value
            destination
            length)
           -1)))
    (c-define
     (gerbil-rs-scheme-object-bytevector-copy-raw value destination length)
     (scheme-object (pointer unsigned-int8) unsigned-int64)
     int64
     "gerbil_scheme_rust_scheme_object_bytevector_copy_raw"
     "extern"
     (if (and (u8vector? value) (= length (u8vector-length value)))
         (gerbil-scheme-rust/scheme/runtime#gerbil-rs-copy-u8vector-c
          value
          destination
          length)
         -1))
    (c-define
     (gerbil-rs-root-release-raw root-id)
     (int64)
     int32
     "gerbil_scheme_rust_root_release_raw"
     "extern"
     (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-release!
          root-id)
         1
         0))
    (c-define
     (gerbil-rs-scheme-object-pair-car-raw value)
     (scheme-object)
     scheme-object
     "gerbil_scheme_rust_scheme_object_pair_car_raw"
     "extern"
     (if (pair? value) (car value) #f))
    (c-define
     (gerbil-rs-scheme-object-pair-cdr-raw value)
     (scheme-object)
     scheme-object
     "gerbil_scheme_rust_scheme_object_pair_cdr_raw"
     "extern"
     (if (pair? value) (cdr value) #f))))
