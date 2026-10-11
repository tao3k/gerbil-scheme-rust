;; gerbil-scheme-rust.generated-scm-provenance.v1 input-sha256=6e941a1456c827a8d3132bfc20efffc9db655458ee5384f5a26ee63b23de5e15 body-sha256=8891ed60706fbbdccaa423ca932d9dbcd3acf7fa1b18debf2deb3f6202af0e94
(declare (block) (standard-bindings) (extended-bindings))
(begin
  (define gerbil-scheme-rust/scheme/runtime::timestamp 1791689663)
  (begin
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-abi-version
      (lambda () '1))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-add-i64
      (lambda (_%left673%_ _%right674%_) (+ _%left673%_ _%right674%_)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-is-even-i64
      (lambda (_%value671%_) (if (even? _%value671%_) '1 '0)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-compare-i64
      (lambda (_%left665%_ _%right666%_)
        (if (< _%left665%_ _%right666%_)
            '-1
            (if (> _%left665%_ _%right666%_) '1 '0))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-next-root-id '1)
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values
      (make-table 'test: eqv? 'weak-keys: '#f 'weak-values: '#f))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-string
      (lambda (_%value663%_)
        (if (string? _%value663%_)
            (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
             _%value663%_)
            '0)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-bytevector
      (lambda (_%value661%_)
        (if (u8vector? _%value661%_)
            (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
             _%value661%_)
            '0)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
      (lambda (_%value657%_)
        (let ((_%root-id659%_
               gerbil-scheme-rust/scheme/runtime#gerbil-rs-next-root-id))
          (set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-next-root-id
                (+ _%root-id659%_ '1))
          (let ()
            (declare (not safe))
            (##table-set!
             gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values
             _%root-id659%_
             _%value657%_))
          _%root-id659%_)))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
      (lambda (_%root-id655%_)
        (let ()
          (declare (not safe))
          (##table-ref
           gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values
           _%root-id655%_
           '#f))))
    (define gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-release!
      (lambda (_%root-id653%_)
        (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
             _%root-id653%_)
            (begin
              (let ()
                (declare (not safe))
                (##table-set!
                 gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values
                 _%root-id653%_))
              '#t)
            '#f)))
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
