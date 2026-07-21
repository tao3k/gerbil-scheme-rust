;; gerbil-scheme-rust.generated-scm-provenance.v1 input-sha256=74ce5e5d9d8757e30fc523422c81acbf7e4d7ce6ca2f84a167aac8c7f616539f body-sha256=a44889adc4484001c01ab398db0cbd9a4b15a3c33ed1a501e9bdae1da09ae02b
(declare (block) (standard-bindings) (extended-bindings))
(begin
  (define gerbil-scheme-rust/scheme/native::timestamp 1784670766)
  (begin
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-next-root-id '1)
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values '())
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
      (lambda (_%value284%_)
        (let ((_%root-id286%_
               gerbil-scheme-rust/scheme/native#gerbil-rs-next-root-id))
          (set! gerbil-scheme-rust/scheme/native#gerbil-rs-next-root-id
                (+ _%root-id286%_ '1))
          (set! gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values
                (cons (cons _%root-id286%_ _%value284%_)
                      gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values))
          _%root-id286%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
      (lambda (_%root-id275%_)
        (let _%lp277%_ ((_%rest279%_
                         gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values))
          (if (null? _%rest279%_)
              '#f
              (if (= (caar _%rest279%_) _%root-id275%_)
                  (cdar _%rest279%_)
                  (_%lp277%_ (cdr _%rest279%_)))))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values-remove
      (lambda (_%rest265%_ _%root-id266%_)
        (if (null? _%rest265%_)
            (values _%rest265%_ '#f)
            (if (= (caar _%rest265%_) _%root-id266%_)
                (values (cdr _%rest265%_) '#t)
                (let ((__tmp3253
                       (lambda ()
                         (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values-remove
                          (cdr _%rest265%_)
                          _%root-id266%_)))
                      (__tmp3252
                       (lambda (_%tail272%_ _%found?273%_)
                         (values (if _%found?273%_
                                     (cons (car _%rest265%_) _%tail272%_)
                                     _%rest265%_)
                                 _%found?273%_))))
                  (declare (not safe))
                  (##call-with-values __tmp3253 __tmp3252))))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-release!
      (lambda (_%root-id259%_)
        (let ((__tmp3255
               (lambda ()
                 (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values-remove
                  gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values
                  _%root-id259%_)))
              (__tmp3254
               (lambda (_%rooted-values262%_ _%found?263%_)
                 (if _%found?263%_
                     (set! gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values
                           _%rooted-values262%_)
                     '#!void)
                 _%found?263%_)))
          (declare (not safe))
          (##call-with-values __tmp3255 __tmp3254))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring-delimiter
      (lambda (_%code254%_)
        (if (= _%code254%_ '-1)
            '#f
            (if (and (>= _%code254%_ '0)
                     (<= _%code254%_ '1114111)
                     (not (<= '55296 _%code254%_ '57343)))
                (integer->char _%code254%_)
                '#!void))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digits
      '"0123456789ABCDEF")
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digit
      (lambda (_%value252%_)
        (string-ref
         gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digits
         _%value252%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-hex-value
      (lambda (_%character246%_)
        (if (char<=? '#\0 _%character246%_ '#\9)
            (- (char->integer _%character246%_)
               (let () (declare (not safe)) (##char->integer '#\0)))
            (if (char<=? '#\A _%character246%_ '#\F)
                (+ '10
                   (- (char->integer _%character246%_)
                      (let () (declare (not safe)) (##char->integer '#\A))))
                (if (char<=? '#\a _%character246%_ '#\f)
                    (+ '10
                       (- (char->integer _%character246%_)
                          (let ()
                            (declare (not safe))
                            (##char->integer '#\a))))
                    '-1)))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->bytestring
      (lambda (_%value226%_ _%delimiter227%_)
        (if (and (u8vector? _%value226%_)
                 (or (not _%delimiter227%_) (char? _%delimiter227%_)))
            (let* ((_%length232%_ (u8vector-length _%value226%_))
                   (_%delimiter-count234%_
                    (if (and (> _%length232%_ '0) _%delimiter227%_)
                        (- _%length232%_ '1)
                        '0))
                   (_%bytestring236%_
                    (make-string
                     (+ (* _%length232%_ '2) _%delimiter-count234%_))))
              (let _%lp239%_ ((_%index241%_ '0) (_%offset242%_ '0))
                (if (< _%index241%_ _%length232%_)
                    (let ((_%byte244%_
                           (u8vector-ref _%value226%_ _%index241%_)))
                      (if (and (> _%index241%_ '0) _%delimiter227%_)
                          (begin
                            (string-set!
                             _%bytestring236%_
                             _%offset242%_
                             _%delimiter227%_)
                            (set! _%offset242%_ (+ _%offset242%_ '1)))
                          '#!void)
                      (string-set!
                       _%bytestring236%_
                       _%offset242%_
                       (gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digit
                        (arithmetic-shift _%byte244%_ '-4)))
                      (string-set!
                       _%bytestring236%_
                       (+ _%offset242%_ '1)
                       (gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digit
                        (bitwise-and _%byte244%_ '15)))
                      (_%lp239%_ (+ _%index241%_ '1) (+ _%offset242%_ '2)))
                    '#!void))
              _%bytestring236%_)
            '#f)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-hex-byte
      (lambda (_%bytestring220%_ _%offset221%_)
        (let ((_%high223%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-hex-value
                (string-ref _%bytestring220%_ _%offset221%_)))
              (_%low224%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-hex-value
                (string-ref _%bytestring220%_ (+ _%offset221%_ '1)))))
          (if (and (>= _%high223%_ '0) (>= _%low224%_ '0))
              (+ (arithmetic-shift _%high223%_ '4) _%low224%_)
              '-1))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring->u8vector
      (lambda (_%bytestring185%_ _%delimiter186%_)
        (if (and (string? _%bytestring185%_)
                 (or (not _%delimiter186%_) (char? _%delimiter186%_)))
            (let* ((_%length191%_ (string-length _%bytestring185%_))
                   (_%valid-length?196%_
                    (if _%delimiter186%_
                        (let ((_%$e193%_ (zero? _%length191%_)))
                          (if _%$e193%_
                              _%$e193%_
                              (zero? (modulo (+ _%length191%_ '1) '3))))
                        (zero? (modulo _%length191%_ '2))))
                   (_%byte-count198%_
                    (if _%delimiter186%_
                        (quotient (+ _%length191%_ '1) '3)
                        (quotient _%length191%_ '2)))
                   (_%value200%_
                    (if _%valid-length?196%_
                        (make-u8vector _%byte-count198%_)
                        '#f)))
              (if _%value200%_
                  (let _%lp203%_ ((_%index205%_ '0))
                    (if (< _%index205%_ _%byte-count198%_)
                        (let* ((_%offset207%_
                                (if _%delimiter186%_
                                    (* _%index205%_ '3)
                                    (* _%index205%_ '2)))
                               (_%delimiter-valid?215%_
                                (let ((_%$e209%_ (not _%delimiter186%_)))
                                  (if _%$e209%_
                                      _%$e209%_
                                      (let ((_%$e212%_ (zero? _%index205%_)))
                                        (if _%$e212%_
                                            _%$e212%_
                                            (eq? _%delimiter186%_
                                                 (string-ref
                                                  _%bytestring185%_
                                                  (- _%offset207%_ '1))))))))
                               (_%byte217%_
                                (if _%delimiter-valid?215%_
                                    (gerbil-scheme-rust/scheme/native#gerbil-rs-hex-byte
                                     _%bytestring185%_
                                     _%offset207%_)
                                    '#f)))
                          (if (and _%byte217%_ (>= _%byte217%_ '0))
                              (begin
                                (u8vector-set!
                                 _%value200%_
                                 _%index205%_
                                 _%byte217%_)
                                (_%lp203%_ (+ _%index205%_ '1)))
                              '#f))
                        _%value200%_))
                  '#f))
            '#f)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-bytevector->bytestring-root
      (lambda (_%value180%_ _%delimiter-code181%_)
        (let ((_%bytestring183%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->bytestring
                _%value180%_
                (gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring-delimiter
                 _%delimiter-code181%_))))
          (if _%bytestring183%_
              (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
               _%bytestring183%_)
              '0))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring->bytevector-root
      (lambda (_%bytestring175%_ _%delimiter-code176%_)
        (let ((_%bytevector178%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring->u8vector
                _%bytestring175%_
                (gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring-delimiter
                 _%delimiter-code176%_))))
          (if _%bytevector178%_
              (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
               _%bytevector178%_)
              '0))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->uint
      (lambda (_%value166%_ _%byte-order167%_ _%size168%_)
        (let _%lp170%_ ((_%index172%_
                         (if (= _%byte-order167%_ '0) '0 (- _%size168%_ '1)))
                        (_%result173%_ '0))
          (if (if (= _%byte-order167%_ '0)
                  (< _%index172%_ _%size168%_)
                  (>= _%index172%_ '0))
              (_%lp170%_
               (if (= _%byte-order167%_ '0)
                   (+ _%index172%_ '1)
                   (- _%index172%_ '1))
               (bitwise-ior
                (arithmetic-shift _%result173%_ '8)
                (u8vector-ref _%value166%_ _%index172%_)))
              _%result173%_))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->sint
      (lambda (_%value155%_ _%byte-order156%_ _%size157%_)
        (if (zero? _%size157%_)
            '0
            (let* ((_%uint159%_
                    (gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->uint
                     _%value155%_
                     _%byte-order156%_
                     _%size157%_))
                   (_%bits161%_ (* _%size157%_ '8))
                   (_%sign-bit163%_ (arithmetic-shift '1 (- _%bits161%_ '1))))
              (if (zero? (bitwise-and _%uint159%_ _%sign-bit163%_))
                  _%uint159%_
                  (- _%uint159%_ (arithmetic-shift '1 _%bits161%_)))))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-uint->u8vector
      (lambda (_%uint144%_ _%byte-order145%_ _%size146%_)
        (let ((_%value148%_ (make-u8vector _%size146%_)))
          (let _%lp150%_ ((_%index152%_ '0) (_%rest153%_ _%uint144%_))
            (if (< _%index152%_ _%size146%_)
                (begin
                  (u8vector-set!
                   _%value148%_
                   (if (= _%byte-order145%_ '0)
                       (- _%size146%_ _%index152%_ '1)
                       _%index152%_)
                   (bitwise-and _%rest153%_ '255))
                  (_%lp150%_
                   (+ _%index152%_ '1)
                   (arithmetic-shift _%rest153%_ '-8)))
                '#!void))
          _%value148%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-sint->u8vector
      (lambda (_%sint140%_ _%byte-order141%_ _%size142%_)
        (gerbil-scheme-rust/scheme/native#gerbil-rs-uint->u8vector
         (if (< _%sint140%_ '0)
             (+ _%sint140%_ (arithmetic-shift '1 (* _%size142%_ '8)))
             _%sint140%_)
         _%byte-order141%_
         _%size142%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-root-bytevector->uint
      (lambda (_%root-id134%_ _%byte-order135%_ _%size136%_)
        (let ((_%value138%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
                _%root-id134%_)))
          (if (u8vector? _%value138%_)
              (gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->uint
               _%value138%_
               _%byte-order135%_
               _%size136%_)
              '0))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-root-bytevector->sint
      (lambda (_%root-id128%_ _%byte-order129%_ _%size130%_)
        (let ((_%value132%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
                _%root-id128%_)))
          (if (u8vector? _%value132%_)
              (gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->sint
               _%value132%_
               _%byte-order129%_
               _%size130%_)
              '0))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-uint->bytevector-root
      (lambda (_%uint124%_ _%byte-order125%_ _%size126%_)
        (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
         (gerbil-scheme-rust/scheme/native#gerbil-rs-uint->u8vector
          _%uint124%_
          _%byte-order125%_
          _%size126%_))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-sint->bytevector-root
      (lambda (_%sint120%_ _%byte-order121%_ _%size122%_)
        (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
         (gerbil-scheme-rust/scheme/native#gerbil-rs-sint->u8vector
          _%sint120%_
          _%byte-order121%_
          _%size122%_))))
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
      gerbil-rs-sint->bytevector-root-raw
      gerbil-rs-uint->bytevector-root-raw
      gerbil-rs-root-bytevector->sint-raw
      gerbil-rs-root-bytevector->uint-raw
      gerbil-rs-bytevector->sint-raw
      gerbil-rs-bytevector->uint-raw
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
