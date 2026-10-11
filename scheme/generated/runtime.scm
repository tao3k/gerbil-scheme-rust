;; gerbil-scheme-rust.generated-scm-provenance.v1 input-sha256=52a10c883c6270b3c8cda93f91803a3c60c6312a5330b0688bcbdd1b9d665aff body-sha256=0a03bd1bbb0a884d00848a05ff30efecd5a99202f3b8a245bfb546b37f157d0b
(declare (block) (standard-bindings) (extended-bindings))
(begin
  (define gerbil-scheme-rust/scheme/runtime::timestamp 1791677034)
  (begin
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-abi-version
      (lambda () '1))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-add-i64
      (lambda (_%left644%_ _%right645%_) (+ _%left644%_ _%right645%_)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-is-even-i64
      (lambda (_%value642%_) (if (even? _%value642%_) '1 '0)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-compare-i64
      (lambda (_%left636%_ _%right637%_)
        (if (< _%left636%_ _%right637%_)
            '-1
            (if (> _%left636%_ _%right637%_) '1 '0))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-next-root-id '1)
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values
      (make-table 'test: eqv? 'weak-keys: '#f 'weak-values: '#f))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-string
      (lambda (_%value634%_)
        (if (string? _%value634%_)
            (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
             _%value634%_)
            '0)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-bytevector
      (lambda (_%value632%_)
        (if (u8vector? _%value632%_)
            (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
             _%value632%_)
            '0)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
      (lambda (_%value628%_)
        (let ((_%root-id630%_
               gerbil-scheme-rust/scheme/runtime#gerbil-rs-next-root-id))
          (set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-next-root-id
                (+ _%root-id630%_ '1))
          (let ()
            (declare (not safe))
            (##table-set!
             gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values
             _%root-id630%_
             _%value628%_))
          _%root-id630%_)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
      (lambda (_%root-id626%_)
        (let ()
          (declare (not safe))
          (##table-ref
           gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values
           _%root-id626%_
           '#f))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-release!
      (lambda (_%root-id624%_)
        (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
             _%root-id624%_)
            (begin
              (let ()
                (declare (not safe))
                (##table-set!
                 gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values
                 _%root-id624%_))
              '#t)
            '#f)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-bytestring-delimiter
      (lambda (_%code619%_)
        (if (= _%code619%_ '-1)
            '#f
            (if (and (>= _%code619%_ '0)
                     (<= _%code619%_ '1114111)
                     (not (<= '55296 _%code619%_ '57343)))
                (integer->char _%code619%_)
                '#!void))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-upper-hex-digits
      '"0123456789ABCDEF")
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-upper-hex-digit
      (lambda (_%value617%_)
        (string-ref
         gerbil-scheme-rust/scheme/runtime#gerbil-rs-upper-hex-digits
         _%value617%_)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-hex-value
      (lambda (_%character611%_)
        (if (char<=? '#\0 _%character611%_ '#\9)
            (- (char->integer _%character611%_)
               (let () (declare (not safe)) (##char->integer '#\0)))
            (if (char<=? '#\A _%character611%_ '#\F)
                (+ '10
                   (- (char->integer _%character611%_)
                      (let () (declare (not safe)) (##char->integer '#\A))))
                (if (char<=? '#\a _%character611%_ '#\f)
                    (+ '10
                       (- (char->integer _%character611%_)
                          (let ()
                            (declare (not safe))
                            (##char->integer '#\a))))
                    '-1)))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-u8vector->bytestring
      (lambda (_%value591%_ _%delimiter592%_)
        (if (and (u8vector? _%value591%_)
                 (or (not _%delimiter592%_) (char? _%delimiter592%_)))
            (let* ((_%length597%_ (u8vector-length _%value591%_))
                   (_%delimiter-count599%_
                    (if (and (> _%length597%_ '0) _%delimiter592%_)
                        (- _%length597%_ '1)
                        '0))
                   (_%bytestring601%_
                    (make-string
                     (+ (* _%length597%_ '2) _%delimiter-count599%_))))
              (let _%lp604%_ ((_%index606%_ '0) (_%offset607%_ '0))
                (if (< _%index606%_ _%length597%_)
                    (let ((_%byte609%_
                           (u8vector-ref _%value591%_ _%index606%_)))
                      (if (and (> _%index606%_ '0) _%delimiter592%_)
                          (begin
                            (string-set!
                             _%bytestring601%_
                             _%offset607%_
                             _%delimiter592%_)
                            (set! _%offset607%_ (+ _%offset607%_ '1)))
                          '#!void)
                      (string-set!
                       _%bytestring601%_
                       _%offset607%_
                       (gerbil-scheme-rust/scheme/runtime#gerbil-rs-upper-hex-digit
                        (arithmetic-shift _%byte609%_ '-4)))
                      (string-set!
                       _%bytestring601%_
                       (+ _%offset607%_ '1)
                       (gerbil-scheme-rust/scheme/runtime#gerbil-rs-upper-hex-digit
                        (bitwise-and _%byte609%_ '15)))
                      (_%lp604%_ (+ _%index606%_ '1) (+ _%offset607%_ '2)))
                    '#!void))
              _%bytestring601%_)
            '#f)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-hex-byte
      (lambda (_%bytestring585%_ _%offset586%_)
        (let ((_%high588%_
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-hex-value
                (string-ref _%bytestring585%_ _%offset586%_)))
              (_%low589%_
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-hex-value
                (string-ref _%bytestring585%_ (+ _%offset586%_ '1)))))
          (if (and (>= _%high588%_ '0) (>= _%low589%_ '0))
              (+ (arithmetic-shift _%high588%_ '4) _%low589%_)
              '-1))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-bytestring->u8vector
      (lambda (_%bytestring550%_ _%delimiter551%_)
        (if (and (string? _%bytestring550%_)
                 (or (not _%delimiter551%_) (char? _%delimiter551%_)))
            (let* ((_%length556%_ (string-length _%bytestring550%_))
                   (_%valid-length?561%_
                    (if _%delimiter551%_
                        (let ((_%$e558%_ (zero? _%length556%_)))
                          (if _%$e558%_
                              _%$e558%_
                              (zero? (modulo (+ _%length556%_ '1) '3))))
                        (zero? (modulo _%length556%_ '2))))
                   (_%byte-count563%_
                    (if _%delimiter551%_
                        (quotient (+ _%length556%_ '1) '3)
                        (quotient _%length556%_ '2)))
                   (_%value565%_
                    (if _%valid-length?561%_
                        (make-u8vector _%byte-count563%_)
                        '#f)))
              (if _%value565%_
                  (let _%lp568%_ ((_%index570%_ '0))
                    (if (< _%index570%_ _%byte-count563%_)
                        (let* ((_%offset572%_
                                (if _%delimiter551%_
                                    (* _%index570%_ '3)
                                    (* _%index570%_ '2)))
                               (_%delimiter-valid?580%_
                                (let ((_%$e574%_ (not _%delimiter551%_)))
                                  (if _%$e574%_
                                      _%$e574%_
                                      (let ((_%$e577%_ (zero? _%index570%_)))
                                        (if _%$e577%_
                                            _%$e577%_
                                            (eq? _%delimiter551%_
                                                 (string-ref
                                                  _%bytestring550%_
                                                  (- _%offset572%_ '1))))))))
                               (_%byte582%_
                                (if _%delimiter-valid?580%_
                                    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-hex-byte
                                     _%bytestring550%_
                                     _%offset572%_)
                                    '#f)))
                          (if (and _%byte582%_ (>= _%byte582%_ '0))
                              (begin
                                (u8vector-set!
                                 _%value565%_
                                 _%index570%_
                                 _%byte582%_)
                                (_%lp568%_ (+ _%index570%_ '1)))
                              '#f))
                        _%value565%_))
                  '#f))
            '#f)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-bytevector->bytestring-root
      (lambda (_%value545%_ _%delimiter-code546%_)
        (let ((_%bytestring548%_
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-u8vector->bytestring
                _%value545%_
                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-bytestring-delimiter
                 _%delimiter-code546%_))))
          (if _%bytestring548%_
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
               _%bytestring548%_)
              '0))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-bytestring->bytevector-root
      (lambda (_%bytestring540%_ _%delimiter-code541%_)
        (let ((_%bytevector543%_
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-bytestring->u8vector
                _%bytestring540%_
                (gerbil-scheme-rust/scheme/runtime#gerbil-rs-bytestring-delimiter
                 _%delimiter-code541%_))))
          (if _%bytevector543%_
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
               _%bytevector543%_)
              '0))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-u8vector->uint
      (lambda (_%value531%_ _%byte-order532%_ _%size533%_)
        (let _%lp535%_ ((_%index537%_
                         (if (= _%byte-order532%_ '0) '0 (- _%size533%_ '1)))
                        (_%result538%_ '0))
          (if (if (= _%byte-order532%_ '0)
                  (< _%index537%_ _%size533%_)
                  (>= _%index537%_ '0))
              (_%lp535%_
               (if (= _%byte-order532%_ '0)
                   (+ _%index537%_ '1)
                   (- _%index537%_ '1))
               (bitwise-ior
                (arithmetic-shift _%result538%_ '8)
                (u8vector-ref _%value531%_ _%index537%_)))
              _%result538%_))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-u8vector->sint
      (lambda (_%value520%_ _%byte-order521%_ _%size522%_)
        (if (zero? _%size522%_)
            '0
            (let* ((_%uint524%_
                    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-u8vector->uint
                     _%value520%_
                     _%byte-order521%_
                     _%size522%_))
                   (_%bits526%_ (* _%size522%_ '8))
                   (_%sign-bit528%_ (arithmetic-shift '1 (- _%bits526%_ '1))))
              (if (zero? (bitwise-and _%uint524%_ _%sign-bit528%_))
                  _%uint524%_
                  (- _%uint524%_ (arithmetic-shift '1 _%bits526%_)))))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-uint->u8vector
      (lambda (_%uint509%_ _%byte-order510%_ _%size511%_)
        (let ((_%value513%_ (make-u8vector _%size511%_)))
          (let _%lp515%_ ((_%index517%_ '0) (_%rest518%_ _%uint509%_))
            (if (< _%index517%_ _%size511%_)
                (begin
                  (u8vector-set!
                   _%value513%_
                   (if (= _%byte-order510%_ '0)
                       (- _%size511%_ _%index517%_ '1)
                       _%index517%_)
                   (bitwise-and _%rest518%_ '255))
                  (_%lp515%_
                   (+ _%index517%_ '1)
                   (arithmetic-shift _%rest518%_ '-8)))
                '#!void))
          _%value513%_)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-sint->u8vector
      (lambda (_%sint505%_ _%byte-order506%_ _%size507%_)
        (gerbil-scheme-rust/scheme/runtime#gerbil-rs-uint->u8vector
         (if (< _%sint505%_ '0)
             (+ _%sint505%_ (arithmetic-shift '1 (* _%size507%_ '8)))
             _%sint505%_)
         _%byte-order506%_
         _%size507%_)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-bytevector->uint
      (lambda (_%root-id499%_ _%byte-order500%_ _%size501%_)
        (let ((_%value503%_
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                _%root-id499%_)))
          (if (u8vector? _%value503%_)
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-u8vector->uint
               _%value503%_
               _%byte-order500%_
               _%size501%_)
              '0))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-bytevector->sint
      (lambda (_%root-id493%_ _%byte-order494%_ _%size495%_)
        (let ((_%value497%_
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                _%root-id493%_)))
          (if (u8vector? _%value497%_)
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-u8vector->sint
               _%value497%_
               _%byte-order494%_
               _%size495%_)
              '0))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-uint->bytevector-root
      (lambda (_%uint489%_ _%byte-order490%_ _%size491%_)
        (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
         (gerbil-scheme-rust/scheme/runtime#gerbil-rs-uint->u8vector
          _%uint489%_
          _%byte-order490%_
          _%size491%_))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-sint->bytevector-root
      (lambda (_%sint485%_ _%byte-order486%_ _%size487%_)
        (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
         (gerbil-scheme-rust/scheme/runtime#gerbil-rs-sint->u8vector
          _%sint485%_
          _%byte-order486%_
          _%size487%_))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-i64-min
      (- (arithmetic-shift '1 '63)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-i64-max
      (- (arithmetic-shift '1 '63) '1))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-u64-max
      (- (arithmetic-shift '1 '64) '1))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer?
      (lambda (_%value483%_)
        (if (integer? _%value483%_) (exact? _%value483%_) '#f)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer-fits-i64?
      (lambda (_%value481%_)
        (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer?
             _%value481%_)
            (<= gerbil-scheme-rust/scheme/runtime#gerbil-rs-i64-min
                _%value481%_
                gerbil-scheme-rust/scheme/runtime#gerbil-rs-i64-max)
            '#f)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer-fits-u64?
      (lambda (_%value479%_)
        (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer?
             _%value479%_)
            (<= '0
                _%value479%_
                gerbil-scheme-rust/scheme/runtime#gerbil-rs-u64-max)
            '#f)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-i64->exact-integer-root
      (lambda (_%value477%_)
        (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
         _%value477%_)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-u64->exact-integer-root
      (lambda (_%value475%_)
        (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
         _%value475%_)))
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
