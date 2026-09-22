;; gerbil-scheme-rust.generated-scm-provenance.v1 input-sha256=0c39c366a3a17141249becb779711695ec64cf62647af876e5adb0e189cb1f66 body-sha256=7ea0158966aa619e90cd23a37517c370b4aec2629e97a191f07aedcb92e7f47a
(declare (block) (standard-bindings) (extended-bindings))
(begin
  (define gerbil-scheme-rust/scheme/native::timestamp 1790086637)
  (begin
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-abi-version
      (lambda () '1))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-add-i64
      (lambda (_%left635%_ _%right636%_) (+ _%left635%_ _%right636%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-is-even-i64
      (lambda (_%value633%_) (if (even? _%value633%_) '1 '0)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-compare-i64
      (lambda (_%left627%_ _%right628%_)
        (if (< _%left627%_ _%right628%_)
            '-1
            (if (> _%left627%_ _%right628%_) '1 '0))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-next-root-id '1)
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values '())
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-root-string
      (lambda (_%value625%_)
        (if (string? _%value625%_)
            (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
             _%value625%_)
            '0)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
      (lambda (_%value621%_)
        (let ((_%root-id623%_
               gerbil-scheme-rust/scheme/native#gerbil-rs-next-root-id))
          (set! gerbil-scheme-rust/scheme/native#gerbil-rs-next-root-id
                (+ _%root-id623%_ '1))
          (set! gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values
                (cons (cons _%root-id623%_ _%value621%_)
                      gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values))
          _%root-id623%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
      (lambda (_%root-id612%_)
        (let _%lp614%_ ((_%rest616%_
                         gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values))
          (if (null? _%rest616%_)
              '#f
              (if (= (caar _%rest616%_) _%root-id612%_)
                  (cdar _%rest616%_)
                  (_%lp614%_ (cdr _%rest616%_)))))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values-remove
      (lambda (_%rest602%_ _%root-id603%_)
        (if (null? _%rest602%_)
            (values _%rest602%_ '#f)
            (if (= (caar _%rest602%_) _%root-id603%_)
                (values (cdr _%rest602%_) '#t)
                (let ((__tmp4935
                       (lambda ()
                         (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values-remove
                          (cdr _%rest602%_)
                          _%root-id603%_)))
                      (__tmp4934
                       (lambda (_%tail609%_ _%found?610%_)
                         (values (if _%found?610%_
                                     (cons (car _%rest602%_) _%tail609%_)
                                     _%rest602%_)
                                 _%found?610%_))))
                  (declare (not safe))
                  (##call-with-values __tmp4935 __tmp4934))))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-release!
      (lambda (_%root-id596%_)
        (let ((__tmp4937
               (lambda ()
                 (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values-remove
                  gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values
                  _%root-id596%_)))
              (__tmp4936
               (lambda (_%rooted-values599%_ _%found?600%_)
                 (if _%found?600%_
                     (set! gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values
                           _%rooted-values599%_)
                     '#!void)
                 _%found?600%_)))
          (declare (not safe))
          (##call-with-values __tmp4937 __tmp4936))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring-delimiter
      (lambda (_%code591%_)
        (if (= _%code591%_ '-1)
            '#f
            (if (and (>= _%code591%_ '0)
                     (<= _%code591%_ '1114111)
                     (not (<= '55296 _%code591%_ '57343)))
                (integer->char _%code591%_)
                '#!void))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digits
      '"0123456789ABCDEF")
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digit
      (lambda (_%value589%_)
        (string-ref
         gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digits
         _%value589%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-hex-value
      (lambda (_%character583%_)
        (if (char<=? '#\0 _%character583%_ '#\9)
            (- (char->integer _%character583%_)
               (let () (declare (not safe)) (##char->integer '#\0)))
            (if (char<=? '#\A _%character583%_ '#\F)
                (+ '10
                   (- (char->integer _%character583%_)
                      (let () (declare (not safe)) (##char->integer '#\A))))
                (if (char<=? '#\a _%character583%_ '#\f)
                    (+ '10
                       (- (char->integer _%character583%_)
                          (let ()
                            (declare (not safe))
                            (##char->integer '#\a))))
                    '-1)))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->bytestring
      (lambda (_%value563%_ _%delimiter564%_)
        (if (and (u8vector? _%value563%_)
                 (or (not _%delimiter564%_) (char? _%delimiter564%_)))
            (let* ((_%length569%_ (u8vector-length _%value563%_))
                   (_%delimiter-count571%_
                    (if (and (> _%length569%_ '0) _%delimiter564%_)
                        (- _%length569%_ '1)
                        '0))
                   (_%bytestring573%_
                    (make-string
                     (+ (* _%length569%_ '2) _%delimiter-count571%_))))
              (let _%lp576%_ ((_%index578%_ '0) (_%offset579%_ '0))
                (if (< _%index578%_ _%length569%_)
                    (let ((_%byte581%_
                           (u8vector-ref _%value563%_ _%index578%_)))
                      (if (and (> _%index578%_ '0) _%delimiter564%_)
                          (begin
                            (string-set!
                             _%bytestring573%_
                             _%offset579%_
                             _%delimiter564%_)
                            (set! _%offset579%_ (+ _%offset579%_ '1)))
                          '#!void)
                      (string-set!
                       _%bytestring573%_
                       _%offset579%_
                       (gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digit
                        (arithmetic-shift _%byte581%_ '-4)))
                      (string-set!
                       _%bytestring573%_
                       (+ _%offset579%_ '1)
                       (gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digit
                        (bitwise-and _%byte581%_ '15)))
                      (_%lp576%_ (+ _%index578%_ '1) (+ _%offset579%_ '2)))
                    '#!void))
              _%bytestring573%_)
            '#f)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-hex-byte
      (lambda (_%bytestring557%_ _%offset558%_)
        (let ((_%high560%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-hex-value
                (string-ref _%bytestring557%_ _%offset558%_)))
              (_%low561%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-hex-value
                (string-ref _%bytestring557%_ (+ _%offset558%_ '1)))))
          (if (and (>= _%high560%_ '0) (>= _%low561%_ '0))
              (+ (arithmetic-shift _%high560%_ '4) _%low561%_)
              '-1))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring->u8vector
      (lambda (_%bytestring522%_ _%delimiter523%_)
        (if (and (string? _%bytestring522%_)
                 (or (not _%delimiter523%_) (char? _%delimiter523%_)))
            (let* ((_%length528%_ (string-length _%bytestring522%_))
                   (_%valid-length?533%_
                    (if _%delimiter523%_
                        (let ((_%$e530%_ (zero? _%length528%_)))
                          (if _%$e530%_
                              _%$e530%_
                              (zero? (modulo (+ _%length528%_ '1) '3))))
                        (zero? (modulo _%length528%_ '2))))
                   (_%byte-count535%_
                    (if _%delimiter523%_
                        (quotient (+ _%length528%_ '1) '3)
                        (quotient _%length528%_ '2)))
                   (_%value537%_
                    (if _%valid-length?533%_
                        (make-u8vector _%byte-count535%_)
                        '#f)))
              (if _%value537%_
                  (let _%lp540%_ ((_%index542%_ '0))
                    (if (< _%index542%_ _%byte-count535%_)
                        (let* ((_%offset544%_
                                (if _%delimiter523%_
                                    (* _%index542%_ '3)
                                    (* _%index542%_ '2)))
                               (_%delimiter-valid?552%_
                                (let ((_%$e546%_ (not _%delimiter523%_)))
                                  (if _%$e546%_
                                      _%$e546%_
                                      (let ((_%$e549%_ (zero? _%index542%_)))
                                        (if _%$e549%_
                                            _%$e549%_
                                            (eq? _%delimiter523%_
                                                 (string-ref
                                                  _%bytestring522%_
                                                  (- _%offset544%_ '1))))))))
                               (_%byte554%_
                                (if _%delimiter-valid?552%_
                                    (gerbil-scheme-rust/scheme/native#gerbil-rs-hex-byte
                                     _%bytestring522%_
                                     _%offset544%_)
                                    '#f)))
                          (if (and _%byte554%_ (>= _%byte554%_ '0))
                              (begin
                                (u8vector-set!
                                 _%value537%_
                                 _%index542%_
                                 _%byte554%_)
                                (_%lp540%_ (+ _%index542%_ '1)))
                              '#f))
                        _%value537%_))
                  '#f))
            '#f)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-bytevector->bytestring-root
      (lambda (_%value517%_ _%delimiter-code518%_)
        (let ((_%bytestring520%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->bytestring
                _%value517%_
                (gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring-delimiter
                 _%delimiter-code518%_))))
          (if _%bytestring520%_
              (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
               _%bytestring520%_)
              '0))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring->bytevector-root
      (lambda (_%bytestring512%_ _%delimiter-code513%_)
        (let ((_%bytevector515%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring->u8vector
                _%bytestring512%_
                (gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring-delimiter
                 _%delimiter-code513%_))))
          (if _%bytevector515%_
              (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
               _%bytevector515%_)
              '0))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->uint
      (lambda (_%value503%_ _%byte-order504%_ _%size505%_)
        (let _%lp507%_ ((_%index509%_
                         (if (= _%byte-order504%_ '0) '0 (- _%size505%_ '1)))
                        (_%result510%_ '0))
          (if (if (= _%byte-order504%_ '0)
                  (< _%index509%_ _%size505%_)
                  (>= _%index509%_ '0))
              (_%lp507%_
               (if (= _%byte-order504%_ '0)
                   (+ _%index509%_ '1)
                   (- _%index509%_ '1))
               (bitwise-ior
                (arithmetic-shift _%result510%_ '8)
                (u8vector-ref _%value503%_ _%index509%_)))
              _%result510%_))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->sint
      (lambda (_%value492%_ _%byte-order493%_ _%size494%_)
        (if (zero? _%size494%_)
            '0
            (let* ((_%uint496%_
                    (gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->uint
                     _%value492%_
                     _%byte-order493%_
                     _%size494%_))
                   (_%bits498%_ (* _%size494%_ '8))
                   (_%sign-bit500%_ (arithmetic-shift '1 (- _%bits498%_ '1))))
              (if (zero? (bitwise-and _%uint496%_ _%sign-bit500%_))
                  _%uint496%_
                  (- _%uint496%_ (arithmetic-shift '1 _%bits498%_)))))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-uint->u8vector
      (lambda (_%uint481%_ _%byte-order482%_ _%size483%_)
        (let ((_%value485%_ (make-u8vector _%size483%_)))
          (let _%lp487%_ ((_%index489%_ '0) (_%rest490%_ _%uint481%_))
            (if (< _%index489%_ _%size483%_)
                (begin
                  (u8vector-set!
                   _%value485%_
                   (if (= _%byte-order482%_ '0)
                       (- _%size483%_ _%index489%_ '1)
                       _%index489%_)
                   (bitwise-and _%rest490%_ '255))
                  (_%lp487%_
                   (+ _%index489%_ '1)
                   (arithmetic-shift _%rest490%_ '-8)))
                '#!void))
          _%value485%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-sint->u8vector
      (lambda (_%sint477%_ _%byte-order478%_ _%size479%_)
        (gerbil-scheme-rust/scheme/native#gerbil-rs-uint->u8vector
         (if (< _%sint477%_ '0)
             (+ _%sint477%_ (arithmetic-shift '1 (* _%size479%_ '8)))
             _%sint477%_)
         _%byte-order478%_
         _%size479%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-root-bytevector->uint
      (lambda (_%root-id471%_ _%byte-order472%_ _%size473%_)
        (let ((_%value475%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
                _%root-id471%_)))
          (if (u8vector? _%value475%_)
              (gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->uint
               _%value475%_
               _%byte-order472%_
               _%size473%_)
              '0))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-root-bytevector->sint
      (lambda (_%root-id465%_ _%byte-order466%_ _%size467%_)
        (let ((_%value469%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
                _%root-id465%_)))
          (if (u8vector? _%value469%_)
              (gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->sint
               _%value469%_
               _%byte-order466%_
               _%size467%_)
              '0))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-uint->bytevector-root
      (lambda (_%uint461%_ _%byte-order462%_ _%size463%_)
        (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
         (gerbil-scheme-rust/scheme/native#gerbil-rs-uint->u8vector
          _%uint461%_
          _%byte-order462%_
          _%size463%_))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-sint->bytevector-root
      (lambda (_%sint457%_ _%byte-order458%_ _%size459%_)
        (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
         (gerbil-scheme-rust/scheme/native#gerbil-rs-sint->u8vector
          _%sint457%_
          _%byte-order458%_
          _%size459%_))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-i64-min
      (- (arithmetic-shift '1 '63)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-i64-max
      (- (arithmetic-shift '1 '63) '1))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-u64-max
      (- (arithmetic-shift '1 '64) '1))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-exact-integer?
      (lambda (_%value455%_)
        (if (integer? _%value455%_) (exact? _%value455%_) '#f)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-exact-integer-fits-i64?
      (lambda (_%value453%_)
        (if (gerbil-scheme-rust/scheme/native#gerbil-rs-exact-integer?
             _%value453%_)
            (<= gerbil-scheme-rust/scheme/native#gerbil-rs-i64-min
                _%value453%_
                gerbil-scheme-rust/scheme/native#gerbil-rs-i64-max)
            '#f)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-exact-integer-fits-u64?
      (lambda (_%value451%_)
        (if (gerbil-scheme-rust/scheme/native#gerbil-rs-exact-integer?
             _%value451%_)
            (<= '0
                _%value451%_
                gerbil-scheme-rust/scheme/native#gerbil-rs-u64-max)
            '#f)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-i64->exact-integer-root
      (lambda (_%value449%_)
        (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
         _%value449%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-u64->exact-integer-root
      (lambda (_%value447%_)
        (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
         _%value447%_)))
    (namespace
     ("gerbil-scheme-rust/scheme/native#"
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
      gerbil-rs-root-bytevector-length-raw
      gerbil-rs-root-bytevector-u8-ref-raw
      gerbil-rs-root-release-raw
      gerbil-rs-scheme-object-pair-car-raw
      gerbil-rs-scheme-object-pair-cdr-raw))
    (c-define
     (gerbil-rs-abi-version-native)
     ()
     unsigned-int32
     "gerbil_scheme_rust_abi_version"
     "extern"
     (gerbil-scheme-rust/scheme/native#gerbil-rs-abi-version))
    (c-define
     (gerbil-rs-add-i64-native left right)
     (int64 int64)
     int64
     "gerbil_scheme_rust_add_i64"
     "extern"
     (gerbil-scheme-rust/scheme/native#gerbil-rs-add-i64 left right))
    (c-define
     (gerbil-rs-is-even-i64-native value)
     (int64)
     int32
     "gerbil_scheme_rust_is_even_i64"
     "extern"
     (gerbil-scheme-rust/scheme/native#gerbil-rs-is-even-i64 value))
    (c-define
     (gerbil-rs-compare-i64-native left right)
     (int64 int64)
     int32
     "gerbil_scheme_rust_compare_i64"
     "extern"
     (gerbil-scheme-rust/scheme/native#gerbil-rs-compare-i64 left right))
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
     (if (gerbil-scheme-rust/scheme/native#gerbil-rs-exact-integer? value)
         1
         0))
    (c-define
     (gerbil-rs-scheme-object-exact-integer-fits-i64?-raw value)
     (scheme-object)
     int32
     "gerbil_scheme_rust_scheme_object_exact_integer_fits_i64_raw"
     "extern"
     (if (gerbil-scheme-rust/scheme/native#gerbil-rs-exact-integer-fits-i64?
          value)
         1
         0))
    (c-define
     (gerbil-rs-scheme-object-exact-integer-fits-u64?-raw value)
     (scheme-object)
     int32
     "gerbil_scheme_rust_scheme_object_exact_integer_fits_u64_raw"
     "extern"
     (if (gerbil-scheme-rust/scheme/native#gerbil-rs-exact-integer-fits-u64?
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
     (gerbil-scheme-rust/scheme/native#gerbil-rs-bytevector->bytestring-root
      value
      delimiter-code))
    (c-define
     (gerbil-rs-bytestring->bytevector-root-raw bytestring delimiter-code)
     (char-string int32)
     int64
     "gerbil_scheme_rust_bytestring_to_bytevector_root_raw"
     "extern"
     (gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring->bytevector-root
      bytestring
      delimiter-code))
    (c-define
     (gerbil-rs-bytevector->uint-raw value byte-order size)
     (scheme-object int32 int64)
     unsigned-int64
     "gerbil_scheme_rust_bytevector_to_uint_raw"
     "extern"
     (gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->uint
      value
      byte-order
      size))
    (c-define
     (gerbil-rs-bytevector->sint-raw value byte-order size)
     (scheme-object int32 int64)
     int64
     "gerbil_scheme_rust_bytevector_to_sint_raw"
     "extern"
     (gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->sint
      value
      byte-order
      size))
    (c-define
     (gerbil-rs-root-bytevector->uint-raw root-id byte-order size)
     (int64 int32 int64)
     unsigned-int64
     "gerbil_scheme_rust_root_bytevector_to_uint_raw"
     "extern"
     (gerbil-scheme-rust/scheme/native#gerbil-rs-root-bytevector->uint
      root-id
      byte-order
      size))
    (c-define
     (gerbil-rs-root-bytevector->sint-raw root-id byte-order size)
     (int64 int32 int64)
     int64
     "gerbil_scheme_rust_root_bytevector_to_sint_raw"
     "extern"
     (gerbil-scheme-rust/scheme/native#gerbil-rs-root-bytevector->sint
      root-id
      byte-order
      size))
    (c-define
     (gerbil-rs-uint->bytevector-root-raw uint byte-order size)
     (unsigned-int64 int32 int64)
     int64
     "gerbil_scheme_rust_uint_to_bytevector_root_raw"
     "extern"
     (gerbil-scheme-rust/scheme/native#gerbil-rs-uint->bytevector-root
      uint
      byte-order
      size))
    (c-define
     (gerbil-rs-sint->bytevector-root-raw sint byte-order size)
     (int64 int32 int64)
     int64
     "gerbil_scheme_rust_sint_to_bytevector_root_raw"
     "extern"
     (gerbil-scheme-rust/scheme/native#gerbil-rs-sint->bytevector-root
      sint
      byte-order
      size))
    (c-define
     (gerbil-rs-i64->exact-integer-root-raw value)
     (int64)
     int64
     "gerbil_scheme_rust_i64_to_exact_integer_root_raw"
     "extern"
     (gerbil-scheme-rust/scheme/native#gerbil-rs-i64->exact-integer-root
      value))
    (c-define
     (gerbil-rs-u64->exact-integer-root-raw value)
     (unsigned-int64)
     int64
     "gerbil_scheme_rust_u64_to_exact_integer_root_raw"
     "extern"
     (gerbil-scheme-rust/scheme/native#gerbil-rs-u64->exact-integer-root
      value))
    (c-define
     (gerbil-rs-root-exact-integer?-raw root-id)
     (int64)
     int32
     "gerbil_scheme_rust_root_is_exact_integer_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (gerbil-scheme-rust/scheme/native#gerbil-rs-exact-integer? value)
           1
           0)))
    (c-define
     (gerbil-rs-root-exact-integer-fits-i64?-raw root-id)
     (int64)
     int32
     "gerbil_scheme_rust_root_exact_integer_fits_i64_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (gerbil-scheme-rust/scheme/native#gerbil-rs-exact-integer-fits-i64?
            value)
           1
           0)))
    (c-define
     (gerbil-rs-root-exact-integer-fits-u64?-raw root-id)
     (int64)
     int32
     "gerbil_scheme_rust_root_exact_integer_fits_u64_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (gerbil-scheme-rust/scheme/native#gerbil-rs-exact-integer-fits-u64?
            value)
           1
           0)))
    (c-define
     (gerbil-rs-root-exact-integer-i64-value-raw root-id)
     (int64)
     int64
     "gerbil_scheme_rust_root_exact_integer_i64_value_raw"
     "extern"
     (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref root-id))
    (c-define
     (gerbil-rs-root-exact-integer-u64-value-raw root-id)
     (int64)
     unsigned-int64
     "gerbil_scheme_rust_root_exact_integer_u64_value_raw"
     "extern"
     (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref root-id))
    (c-define
     (gerbil-rs-root-string-length-raw root-id)
     (int64)
     int64
     "gerbil_scheme_rust_root_string_length_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (string? value) (string-length value) -1)))
    (c-define
     (gerbil-rs-root-string-char-ref-raw root-id index)
     (int64 int64)
     int32
     "gerbil_scheme_rust_root_string_char_ref_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
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
     (let ((value (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (u8vector? value) (u8vector-length value) -1)))
    (c-define
     (gerbil-rs-root-bytevector-u8-ref-raw root-id index)
     (int64 int64)
     int32
     "gerbil_scheme_rust_root_bytevector_u8_ref_raw"
     "extern"
     (let ((value (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
                   root-id)))
       (if (and (u8vector? value)
                (>= index 0)
                (< index (u8vector-length value)))
           (u8vector-ref value index)
           -1)))
    (c-define
     (gerbil-rs-root-release-raw root-id)
     (int64)
     int32
     "gerbil_scheme_rust_root_release_raw"
     "extern"
     (if (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-release!
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
