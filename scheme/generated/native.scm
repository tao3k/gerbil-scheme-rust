(declare (block) (standard-bindings) (extended-bindings))
(begin
  (define gerbil-scheme-rust/scheme/native::timestamp 1784664718)
  (begin
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-next-root-id '1)
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values '())
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
      (lambda (_%value229%_)
        (let ((_%root-id231%_
               gerbil-scheme-rust/scheme/native#gerbil-rs-next-root-id))
          (set! gerbil-scheme-rust/scheme/native#gerbil-rs-next-root-id
                (+ _%root-id231%_ '1))
          (set! gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values
                (cons (cons _%root-id231%_ _%value229%_)
                      gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values))
          _%root-id231%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
      (lambda (_%root-id220%_)
        (let _%lp222%_ ((_%rest224%_
                         gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values))
          (if (null? _%rest224%_)
              '#f
              (if (= (caar _%rest224%_) _%root-id220%_)
                  (cdar _%rest224%_)
                  (_%lp222%_ (cdr _%rest224%_)))))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values-remove
      (lambda (_%rest210%_ _%root-id211%_)
        (if (null? _%rest210%_)
            (values _%rest210%_ '#f)
            (if (= (caar _%rest210%_) _%root-id211%_)
                (values (cdr _%rest210%_) '#t)
                (let ((__tmp3198
                       (lambda ()
                         (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values-remove
                          (cdr _%rest210%_)
                          _%root-id211%_)))
                      (__tmp3197
                       (lambda (_%tail217%_ _%found?218%_)
                         (values (if _%found?218%_
                                     (cons (car _%rest210%_) _%tail217%_)
                                     _%rest210%_)
                                 _%found?218%_))))
                  (declare (not safe))
                  (##call-with-values __tmp3198 __tmp3197))))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-release!
      (lambda (_%root-id204%_)
        (let ((__tmp3200
               (lambda ()
                 (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values-remove
                  gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values
                  _%root-id204%_)))
              (__tmp3199
               (lambda (_%rooted-values207%_ _%found?208%_)
                 (if _%found?208%_
                     (set! gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values
                           _%rooted-values207%_)
                     '#!void)
                 _%found?208%_)))
          (declare (not safe))
          (##call-with-values __tmp3200 __tmp3199))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring-delimiter
      (lambda (_%code199%_)
        (if (= _%code199%_ '-1)
            '#f
            (if (and (>= _%code199%_ '0)
                     (<= _%code199%_ '1114111)
                     (not (<= '55296 _%code199%_ '57343)))
                (integer->char _%code199%_)
                '#!void))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digits
      '"0123456789ABCDEF")
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digit
      (lambda (_%value197%_)
        (string-ref
         gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digits
         _%value197%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-hex-value
      (lambda (_%character191%_)
        (if (char<=? '#\0 _%character191%_ '#\9)
            (- (char->integer _%character191%_)
               (let () (declare (not safe)) (##char->integer '#\0)))
            (if (char<=? '#\A _%character191%_ '#\F)
                (+ '10
                   (- (char->integer _%character191%_)
                      (let () (declare (not safe)) (##char->integer '#\A))))
                (if (char<=? '#\a _%character191%_ '#\f)
                    (+ '10
                       (- (char->integer _%character191%_)
                          (let ()
                            (declare (not safe))
                            (##char->integer '#\a))))
                    '-1)))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->bytestring
      (lambda (_%value171%_ _%delimiter172%_)
        (if (and (u8vector? _%value171%_)
                 (or (not _%delimiter172%_) (char? _%delimiter172%_)))
            (let* ((_%length177%_ (u8vector-length _%value171%_))
                   (_%delimiter-count179%_
                    (if (and (> _%length177%_ '0) _%delimiter172%_)
                        (- _%length177%_ '1)
                        '0))
                   (_%bytestring181%_
                    (make-string
                     (+ (* _%length177%_ '2) _%delimiter-count179%_))))
              (let _%lp184%_ ((_%index186%_ '0) (_%offset187%_ '0))
                (if (< _%index186%_ _%length177%_)
                    (let ((_%byte189%_
                           (u8vector-ref _%value171%_ _%index186%_)))
                      (if (and (> _%index186%_ '0) _%delimiter172%_)
                          (begin
                            (string-set!
                             _%bytestring181%_
                             _%offset187%_
                             _%delimiter172%_)
                            (set! _%offset187%_ (+ _%offset187%_ '1)))
                          '#!void)
                      (string-set!
                       _%bytestring181%_
                       _%offset187%_
                       (gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digit
                        (arithmetic-shift _%byte189%_ '-4)))
                      (string-set!
                       _%bytestring181%_
                       (+ _%offset187%_ '1)
                       (gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digit
                        (bitwise-and _%byte189%_ '15)))
                      (_%lp184%_ (+ _%index186%_ '1) (+ _%offset187%_ '2)))
                    '#!void))
              _%bytestring181%_)
            '#f)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-hex-byte
      (lambda (_%bytestring165%_ _%offset166%_)
        (let ((_%high168%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-hex-value
                (string-ref _%bytestring165%_ _%offset166%_)))
              (_%low169%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-hex-value
                (string-ref _%bytestring165%_ (+ _%offset166%_ '1)))))
          (if (and (>= _%high168%_ '0) (>= _%low169%_ '0))
              (+ (arithmetic-shift _%high168%_ '4) _%low169%_)
              '-1))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring->u8vector
      (lambda (_%bytestring130%_ _%delimiter131%_)
        (if (and (string? _%bytestring130%_)
                 (or (not _%delimiter131%_) (char? _%delimiter131%_)))
            (let* ((_%length136%_ (string-length _%bytestring130%_))
                   (_%valid-length?141%_
                    (if _%delimiter131%_
                        (let ((_%$e138%_ (zero? _%length136%_)))
                          (if _%$e138%_
                              _%$e138%_
                              (zero? (modulo (+ _%length136%_ '1) '3))))
                        (zero? (modulo _%length136%_ '2))))
                   (_%byte-count143%_
                    (if _%delimiter131%_
                        (quotient (+ _%length136%_ '1) '3)
                        (quotient _%length136%_ '2)))
                   (_%value145%_
                    (if _%valid-length?141%_
                        (make-u8vector _%byte-count143%_)
                        '#f)))
              (if _%value145%_
                  (let _%lp148%_ ((_%index150%_ '0))
                    (if (< _%index150%_ _%byte-count143%_)
                        (let* ((_%offset152%_
                                (if _%delimiter131%_
                                    (* _%index150%_ '3)
                                    (* _%index150%_ '2)))
                               (_%delimiter-valid?160%_
                                (let ((_%$e154%_ (not _%delimiter131%_)))
                                  (if _%$e154%_
                                      _%$e154%_
                                      (let ((_%$e157%_ (zero? _%index150%_)))
                                        (if _%$e157%_
                                            _%$e157%_
                                            (eq? _%delimiter131%_
                                                 (string-ref
                                                  _%bytestring130%_
                                                  (- _%offset152%_ '1))))))))
                               (_%byte162%_
                                (if _%delimiter-valid?160%_
                                    (gerbil-scheme-rust/scheme/native#gerbil-rs-hex-byte
                                     _%bytestring130%_
                                     _%offset152%_)
                                    '#f)))
                          (if (and _%byte162%_ (>= _%byte162%_ '0))
                              (begin
                                (u8vector-set!
                                 _%value145%_
                                 _%index150%_
                                 _%byte162%_)
                                (_%lp148%_ (+ _%index150%_ '1)))
                              '#f))
                        _%value145%_))
                  '#f))
            '#f)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-bytevector->bytestring-root
      (lambda (_%value125%_ _%delimiter-code126%_)
        (let ((_%bytestring128%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->bytestring
                _%value125%_
                (gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring-delimiter
                 _%delimiter-code126%_))))
          (if _%bytestring128%_
              (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
               _%bytestring128%_)
              '0))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring->bytevector-root
      (lambda (_%bytestring120%_ _%delimiter-code121%_)
        (let ((_%bytevector123%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring->u8vector
                _%bytestring120%_
                (gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring-delimiter
                 _%delimiter-code121%_))))
          (if _%bytevector123%_
              (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
               _%bytevector123%_)
              '0))))
    (define-macro (define-guard guard defn)
      (if (eval `(cond-expand
                  (gerbil-separate-compilation #f)
                  (,guard #t)
                  (else #f)))
          '(begin)
          (begin (eval `(define-cond-expand-feature ,guard)) defn)))
    (define-macro (define-c-lambda id args ret #!optional (name #f))
      (let ((name (or name (symbol->string id))))
        `(define ,id (c-lambda ,args ,ret ,name))))
    (define-macro (define-const symbol)
      (let* ((str (symbol->string symbol))
             (ref (string-append "___return (" str ");")))
        `(define ,symbol ((c-lambda () int ,ref)))))
    (define-macro (define-const* symbol #!optional (ccond #f))
      (let* ((str (symbol->string symbol))
             (code (string-append
                    "#if "
                    (or ccond (string-append "defined(" str ")"))
                    "\n"
                    "___return (___FIX ("
                    str
                    "));\n"
                    "#else \n"
                    "___return (___FAL);\n"
                    "#endif")))
        `(define ,symbol ((c-lambda () scheme-object ,code)))))
    (define-macro (define-with-errno symbol ffi-symbol args)
      `(define (,symbol ,@args)
         (declare (not interrupts-enabled))
         (let ((r (,ffi-symbol ,@args)))
           (if (##fx< r 0)
               (##fx- (##c-code "___RESULT = ___FIX (errno);"))
               r))))
    (define-macro (define-c-struct
                   struct
                   #!optional
                   (members '())
                   release-function
                   compatible-tags
                   as-typedef)
      (let* ((struct-str (symbol->string struct))
             (struct-ptr (string->symbol (string-append struct-str "*")))
             (shallow-ptr
              (string->symbol (string-append struct-str "-shallow-ptr*")))
             (borrowed-ptr
              (string->symbol (string-append struct-str "-borrowed-ptr*")))
             (struct-keyword? (if as-typedef "" "struct "))
             (string-types
              '(char-string
                nonull-char-string
                UTF-8-string
                nonnull-UTF-8-string
                UTF-16-string
                nonnull-UTF16-string))
             (string-compat-required?
              (let loop ((m members))
                (cond ((null? m) #f)
                      ((member (cdr (car m)) string-types) #t)
                      (else (loop (cdr m))))))
             (string-setter-body
              (lambda (member-name)
                (let ((m (string-append "___arg1->" member-name)))
                  (string-append
                   "if("
                   m
                   " == NULL)"
                   "\n"
                   m
                   "= strdup(___arg2);"
                   "\n"
                   "else if (strcmp("
                   m
                   ", ___arg2) != 0) {"
                   "\n"
                   "free("
                   m
                   ");"
                   "\n"
                   m
                   "= strdup(___arg2);"
                   "\n"
                   "}"
                   "\n"
                   "___return;"
                   "\n"))))
             (default-free-body
              (and string-compat-required?
                   (string-append
                    "___SCMOBJ "
                    struct-str
                    "_ffi_free (void *ptr) {"
                    "\n"
                    struct-keyword?
                    struct-str
                    " *obj = ("
                    struct-keyword?
                    struct-str
                    "*) ptr;"
                    "\n"
                    (apply string-append
                           (map (lambda (m)
                                  (cond ((memq (cdr m) string-types)
                                         (let ((mem-name
                                                (symbol->string (car m))))
                                           (string-append
                                            "if(obj->"
                                            mem-name
                                            ") "
                                            "free(obj->"
                                            mem-name
                                            ");"
                                            "\n")))
                                        (else "")))
                                members))
                    "free(obj);"
                    "\n"
                    "return ___FIX (___NO_ERR);"
                    "\n"
                    "}")))
             (release-function
              (or release-function
                  (if string-compat-required?
                      (string-append struct-str "_ffi_free")
                      "ffi_free")))
             (string-compat-types
              (if string-compat-required?
                  `((c-declare ,default-free-body)
                    (c-define-type
                     ,shallow-ptr
                     (pointer ,struct (,struct-ptr) "ffi_free")))
                  '()))
             (compatible-tags (or compatible-tags '()))
             (ptr-tags
              (map (lambda (t)
                     (string->symbol (string-append (symbol->string t) "*")))
                   compatible-tags)))
        `(begin
           (c-define-type
            ,struct
            (,(if as-typedef 'type 'struct)
             ,struct-str
             (,struct ,@compatible-tags)))
           (c-define-type
            ,struct-ptr
            (pointer ,struct (,struct-ptr ,@ptr-tags) ,release-function))
           (c-define-type ,borrowed-ptr (pointer ,struct (,struct-ptr)))
           ,@string-compat-types
           (define ,(string->symbol (string-append struct-str "-ptr?"))
             (lambda (obj)
               (and (foreign? obj) (member ',struct-ptr (foreign-tags obj)))))
           ,@(apply append
                    (map (lambda (m)
                           (let* ((member-name (symbol->string (car m)))
                                  (member-type (cdr m))
                                  (getter-name
                                   (string-append struct-str "-" member-name))
                                  (setter-body
                                   (cond ((member member-type string-types)
                                          (string-setter-body member-name))
                                         (else
                                          (string-append
                                           "___arg1->"
                                           member-name
                                           " = ___arg2;"
                                           "\n"
                                           "___return;"
                                           "\n")))))
                             `((define ,(string->symbol getter-name)
                                 (c-lambda
                                  (,struct-ptr)
                                  ,member-type
                                  ,(string-append
                                    "___return(___arg1->"
                                    member-name
                                    ");")))
                               (define ,(string->symbol
                                         (string-append getter-name "-set!"))
                                 (c-lambda
                                  (,struct-ptr ,member-type)
                                  void
                                  ,setter-body)))))
                         members))
           (define ,(string->symbol (string-append "malloc-" struct-str))
             (c-lambda
              ()
              ,struct-ptr
              ,(string-append
                struct-keyword?
                struct-str
                "* var = ("
                struct-keyword?
                struct-str
                " *) malloc(sizeof("
                struct-keyword?
                struct-str
                "));"
                "\n"
                "if (var == NULL)"
                "\n"
                "    ___return (NULL);"
                "\n"
                "memset(var, 0, sizeof("
                struct-keyword?
                struct-str
                "));"
                "___return(var);")))
           (define ,(string->symbol (string-append "ptr->" struct-str))
             (c-lambda (,struct-ptr) ,struct "___return(*___arg1);"))
           (define ,(string->symbol
                     (string-append "malloc-" struct-str "-array"))
             (c-lambda
              (unsigned-int32)
              ,(if string-compat-required? shallow-ptr struct-ptr)
              ,(string-append
                struct-keyword?
                struct-str
                " *arr_var=("
                struct-keyword?
                struct-str
                " *) malloc(___arg1*sizeof("
                struct-keyword?
                struct-str
                "));"
                "\n"
                "if (arr_var == NULL)"
                "\n"
                "    ___return (NULL);"
                "\n"
                "memset(arr_var, 0, ___arg1*sizeof("
                struct-keyword?
                struct-str
                "));"
                "\n"
                "___return(arr_var);")))
           (define ,(string->symbol (string-append struct-str "-array-ref"))
             (c-lambda
              (,struct-ptr unsigned-int32)
              ,borrowed-ptr
              "___return (___arg1 + ___arg2);"))
           (define ,(string->symbol (string-append struct-str "-array-set!"))
             (c-lambda
              (,struct-ptr unsigned-int32 ,struct-ptr)
              void
              "*(___arg1 + ___arg2) = *___arg3; ___return;")))))
    (c-declare "#include <stdlib.h>")
    (c-declare "#include <string.h>")
    (c-declare "#include <errno.h>")
    (c-declare "static ___SCMOBJ ffi_free (void *ptr);")
    (c-declare
     "#ifndef ___HAVE_FFI_U8VECTOR\n#define ___HAVE_FFI_U8VECTOR\n#define U8_DATA(obj) ___CAST (___U8*, ___BODY_AS (obj, ___tSUBTYPED))\n#define U8_LEN(obj) ___HD_BYTES (___HEADER (obj))\n#endif")
    (namespace
     ("gerbil-scheme-rust/scheme/native#"
      gerbil-rs-scheme-object-pair-cdr-raw
      gerbil-rs-scheme-object-pair-car-raw
      gerbil-rs-root-release-raw
      gerbil-rs-root-bytevector-u8-ref-raw
      gerbil-rs-root-bytevector-length-raw
      gerbil-rs-root-string-char-ref-raw
      gerbil-rs-root-string-length-raw
      gerbil-rs-bytestring->bytevector-root-raw
      gerbil-rs-bytevector->bytestring-root-raw
      gerbil-rs-scheme-object-bytevector-u8-ref-raw
      gerbil-rs-scheme-object-bytevector-length-raw
      gerbil-rs-scheme-object-flonum-value-raw
      gerbil-rs-scheme-object-flonum?-raw
      gerbil-rs-scheme-object-char-value-raw
      gerbil-rs-scheme-object-char?-raw
      gerbil-rs-scheme-object-fixnum-value-raw
      gerbil-rs-scheme-object-fixnum?-raw
      gerbil-rs-scheme-object-boolean-value-raw
      gerbil-rs-scheme-object-boolean?-raw
      gerbil-rs-scheme-object-list?-raw
      gerbil-rs-scheme-object-pair?-raw
      gerbil-rs-scheme-object-bytevector?-raw
      gerbil-rs-scheme-object-void?-raw
      gerbil-rs-scheme-object-null?-raw
      gerbil-rs-fixture-bytevector-raw
      gerbil-rs-fixture-flonum-neg-zero-raw
      gerbil-rs-fixture-flonum-neg-inf-raw
      gerbil-rs-fixture-flonum-pos-inf-raw
      gerbil-rs-fixture-flonum-nan-raw
      gerbil-rs-fixture-flonum-finite-raw
      gerbil-rs-fixture-char-non-bmp-raw
      gerbil-rs-fixture-char-bmp-raw
      gerbil-rs-fixture-char-ascii-raw
      gerbil-rs-fixture-fixnum-raw
      gerbil-rs-fixture-false-raw
      gerbil-rs-fixture-true-raw
      gerbil-rs-fixture-improper-list-raw
      gerbil-rs-fixture-proper-list-raw
      gerbil-rs-fixture-pair-raw
      gerbil-rs-fixture-void-raw
      gerbil-rs-scheme-null-value-raw
      gerbil-rs-compare-i64
      gerbil-rs-is-even-i64
      gerbil-rs-add-i64
      gerbil-rs-abi-version))
    (c-define
     (gerbil-rs-abi-version)
     ()
     unsigned-int32
     "gerbil_scheme_rust_abi_version"
     "extern"
     1)
    (c-define
     (gerbil-rs-add-i64 left right)
     (int64 int64)
     int64
     "gerbil_scheme_rust_add_i64"
     "extern"
     (+ left right))
    (c-define
     (gerbil-rs-is-even-i64 value)
     (int64)
     int32
     "gerbil_scheme_rust_is_even_i64"
     "extern"
     (if (even? value) 1 0))
    (c-define
     (gerbil-rs-compare-i64 left right)
     (int64 int64)
     int32
     "gerbil_scheme_rust_compare_i64"
     "extern"
     (cond ((< left right) -1) ((> left right) 1) (else 0)))
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
     (if (pair? value) (cdr value) #f))
    (c-declare
     "#ifndef ___HAVE_FFI_FREE\n#define ___HAVE_FFI_FREE\n___SCMOBJ ffi_free (void *ptr)\n{\n free (ptr);\n return ___FIX (___NO_ERR);\n}\n#endif")))
