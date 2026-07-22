;; gerbil-scheme-rust.generated-scm-provenance.v1 input-sha256=d6b52040ac4a678efbb50d66f9e91134ec3119040c8302459aa55802c649aa8a body-sha256=ab55887d7561b749db58bdffa677a7b8d7a1615ecae34bf8ee13a68c7699d973
(declare (block) (standard-bindings) (extended-bindings))
(begin
  (define gerbil-scheme-rust/scheme/native::timestamp 1784678614)
  (begin
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-next-root-id '1)
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values '())
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
      (lambda (_%value294%_)
        (let ((_%root-id296%_
               gerbil-scheme-rust/scheme/native#gerbil-rs-next-root-id))
          (set! gerbil-scheme-rust/scheme/native#gerbil-rs-next-root-id
                (+ _%root-id296%_ '1))
          (set! gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values
                (cons (cons _%root-id296%_ _%value294%_)
                      gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values))
          _%root-id296%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
      (lambda (_%root-id285%_)
        (let _%lp287%_ ((_%rest289%_
                         gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values))
          (if (null? _%rest289%_)
              '#f
              (if (= (caar _%rest289%_) _%root-id285%_)
                  (cdar _%rest289%_)
                  (_%lp287%_ (cdr _%rest289%_)))))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values-remove
      (lambda (_%rest275%_ _%root-id276%_)
        (if (null? _%rest275%_)
            (values _%rest275%_ '#f)
            (if (= (caar _%rest275%_) _%root-id276%_)
                (values (cdr _%rest275%_) '#t)
                (let ((__tmp3263
                       (lambda ()
                         (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values-remove
                          (cdr _%rest275%_)
                          _%root-id276%_)))
                      (__tmp3262
                       (lambda (_%tail282%_ _%found?283%_)
                         (values (if _%found?283%_
                                     (cons (car _%rest275%_) _%tail282%_)
                                     _%rest275%_)
                                 _%found?283%_))))
                  (declare (not safe))
                  (##call-with-values __tmp3263 __tmp3262))))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-release!
      (lambda (_%root-id269%_)
        (let ((__tmp3265
               (lambda ()
                 (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values-remove
                  gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values
                  _%root-id269%_)))
              (__tmp3264
               (lambda (_%rooted-values272%_ _%found?273%_)
                 (if _%found?273%_
                     (set! gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-values
                           _%rooted-values272%_)
                     '#!void)
                 _%found?273%_)))
          (declare (not safe))
          (##call-with-values __tmp3265 __tmp3264))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring-delimiter
      (lambda (_%code264%_)
        (if (= _%code264%_ '-1)
            '#f
            (if (and (>= _%code264%_ '0)
                     (<= _%code264%_ '1114111)
                     (not (<= '55296 _%code264%_ '57343)))
                (integer->char _%code264%_)
                '#!void))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digits
      '"0123456789ABCDEF")
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digit
      (lambda (_%value262%_)
        (string-ref
         gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digits
         _%value262%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-hex-value
      (lambda (_%character256%_)
        (if (char<=? '#\0 _%character256%_ '#\9)
            (- (char->integer _%character256%_)
               (let () (declare (not safe)) (##char->integer '#\0)))
            (if (char<=? '#\A _%character256%_ '#\F)
                (+ '10
                   (- (char->integer _%character256%_)
                      (let () (declare (not safe)) (##char->integer '#\A))))
                (if (char<=? '#\a _%character256%_ '#\f)
                    (+ '10
                       (- (char->integer _%character256%_)
                          (let ()
                            (declare (not safe))
                            (##char->integer '#\a))))
                    '-1)))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->bytestring
      (lambda (_%value236%_ _%delimiter237%_)
        (if (and (u8vector? _%value236%_)
                 (or (not _%delimiter237%_) (char? _%delimiter237%_)))
            (let* ((_%length242%_ (u8vector-length _%value236%_))
                   (_%delimiter-count244%_
                    (if (and (> _%length242%_ '0) _%delimiter237%_)
                        (- _%length242%_ '1)
                        '0))
                   (_%bytestring246%_
                    (make-string
                     (+ (* _%length242%_ '2) _%delimiter-count244%_))))
              (let _%lp249%_ ((_%index251%_ '0) (_%offset252%_ '0))
                (if (< _%index251%_ _%length242%_)
                    (let ((_%byte254%_
                           (u8vector-ref _%value236%_ _%index251%_)))
                      (if (and (> _%index251%_ '0) _%delimiter237%_)
                          (begin
                            (string-set!
                             _%bytestring246%_
                             _%offset252%_
                             _%delimiter237%_)
                            (set! _%offset252%_ (+ _%offset252%_ '1)))
                          '#!void)
                      (string-set!
                       _%bytestring246%_
                       _%offset252%_
                       (gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digit
                        (arithmetic-shift _%byte254%_ '-4)))
                      (string-set!
                       _%bytestring246%_
                       (+ _%offset252%_ '1)
                       (gerbil-scheme-rust/scheme/native#gerbil-rs-upper-hex-digit
                        (bitwise-and _%byte254%_ '15)))
                      (_%lp249%_ (+ _%index251%_ '1) (+ _%offset252%_ '2)))
                    '#!void))
              _%bytestring246%_)
            '#f)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-hex-byte
      (lambda (_%bytestring230%_ _%offset231%_)
        (let ((_%high233%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-hex-value
                (string-ref _%bytestring230%_ _%offset231%_)))
              (_%low234%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-hex-value
                (string-ref _%bytestring230%_ (+ _%offset231%_ '1)))))
          (if (and (>= _%high233%_ '0) (>= _%low234%_ '0))
              (+ (arithmetic-shift _%high233%_ '4) _%low234%_)
              '-1))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring->u8vector
      (lambda (_%bytestring195%_ _%delimiter196%_)
        (if (and (string? _%bytestring195%_)
                 (or (not _%delimiter196%_) (char? _%delimiter196%_)))
            (let* ((_%length201%_ (string-length _%bytestring195%_))
                   (_%valid-length?206%_
                    (if _%delimiter196%_
                        (let ((_%$e203%_ (zero? _%length201%_)))
                          (if _%$e203%_
                              _%$e203%_
                              (zero? (modulo (+ _%length201%_ '1) '3))))
                        (zero? (modulo _%length201%_ '2))))
                   (_%byte-count208%_
                    (if _%delimiter196%_
                        (quotient (+ _%length201%_ '1) '3)
                        (quotient _%length201%_ '2)))
                   (_%value210%_
                    (if _%valid-length?206%_
                        (make-u8vector _%byte-count208%_)
                        '#f)))
              (if _%value210%_
                  (let _%lp213%_ ((_%index215%_ '0))
                    (if (< _%index215%_ _%byte-count208%_)
                        (let* ((_%offset217%_
                                (if _%delimiter196%_
                                    (* _%index215%_ '3)
                                    (* _%index215%_ '2)))
                               (_%delimiter-valid?225%_
                                (let ((_%$e219%_ (not _%delimiter196%_)))
                                  (if _%$e219%_
                                      _%$e219%_
                                      (let ((_%$e222%_ (zero? _%index215%_)))
                                        (if _%$e222%_
                                            _%$e222%_
                                            (eq? _%delimiter196%_
                                                 (string-ref
                                                  _%bytestring195%_
                                                  (- _%offset217%_ '1))))))))
                               (_%byte227%_
                                (if _%delimiter-valid?225%_
                                    (gerbil-scheme-rust/scheme/native#gerbil-rs-hex-byte
                                     _%bytestring195%_
                                     _%offset217%_)
                                    '#f)))
                          (if (and _%byte227%_ (>= _%byte227%_ '0))
                              (begin
                                (u8vector-set!
                                 _%value210%_
                                 _%index215%_
                                 _%byte227%_)
                                (_%lp213%_ (+ _%index215%_ '1)))
                              '#f))
                        _%value210%_))
                  '#f))
            '#f)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-bytevector->bytestring-root
      (lambda (_%value190%_ _%delimiter-code191%_)
        (let ((_%bytestring193%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->bytestring
                _%value190%_
                (gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring-delimiter
                 _%delimiter-code191%_))))
          (if _%bytestring193%_
              (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
               _%bytestring193%_)
              '0))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring->bytevector-root
      (lambda (_%bytestring185%_ _%delimiter-code186%_)
        (let ((_%bytevector188%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring->u8vector
                _%bytestring185%_
                (gerbil-scheme-rust/scheme/native#gerbil-rs-bytestring-delimiter
                 _%delimiter-code186%_))))
          (if _%bytevector188%_
              (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
               _%bytevector188%_)
              '0))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->uint
      (lambda (_%value176%_ _%byte-order177%_ _%size178%_)
        (let _%lp180%_ ((_%index182%_
                         (if (= _%byte-order177%_ '0) '0 (- _%size178%_ '1)))
                        (_%result183%_ '0))
          (if (if (= _%byte-order177%_ '0)
                  (< _%index182%_ _%size178%_)
                  (>= _%index182%_ '0))
              (_%lp180%_
               (if (= _%byte-order177%_ '0)
                   (+ _%index182%_ '1)
                   (- _%index182%_ '1))
               (bitwise-ior
                (arithmetic-shift _%result183%_ '8)
                (u8vector-ref _%value176%_ _%index182%_)))
              _%result183%_))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->sint
      (lambda (_%value165%_ _%byte-order166%_ _%size167%_)
        (if (zero? _%size167%_)
            '0
            (let* ((_%uint169%_
                    (gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->uint
                     _%value165%_
                     _%byte-order166%_
                     _%size167%_))
                   (_%bits171%_ (* _%size167%_ '8))
                   (_%sign-bit173%_ (arithmetic-shift '1 (- _%bits171%_ '1))))
              (if (zero? (bitwise-and _%uint169%_ _%sign-bit173%_))
                  _%uint169%_
                  (- _%uint169%_ (arithmetic-shift '1 _%bits171%_)))))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-uint->u8vector
      (lambda (_%uint154%_ _%byte-order155%_ _%size156%_)
        (let ((_%value158%_ (make-u8vector _%size156%_)))
          (let _%lp160%_ ((_%index162%_ '0) (_%rest163%_ _%uint154%_))
            (if (< _%index162%_ _%size156%_)
                (begin
                  (u8vector-set!
                   _%value158%_
                   (if (= _%byte-order155%_ '0)
                       (- _%size156%_ _%index162%_ '1)
                       _%index162%_)
                   (bitwise-and _%rest163%_ '255))
                  (_%lp160%_
                   (+ _%index162%_ '1)
                   (arithmetic-shift _%rest163%_ '-8)))
                '#!void))
          _%value158%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-sint->u8vector
      (lambda (_%sint150%_ _%byte-order151%_ _%size152%_)
        (gerbil-scheme-rust/scheme/native#gerbil-rs-uint->u8vector
         (if (< _%sint150%_ '0)
             (+ _%sint150%_ (arithmetic-shift '1 (* _%size152%_ '8)))
             _%sint150%_)
         _%byte-order151%_
         _%size152%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-root-bytevector->uint
      (lambda (_%root-id144%_ _%byte-order145%_ _%size146%_)
        (let ((_%value148%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
                _%root-id144%_)))
          (if (u8vector? _%value148%_)
              (gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->uint
               _%value148%_
               _%byte-order145%_
               _%size146%_)
              '0))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-root-bytevector->sint
      (lambda (_%root-id138%_ _%byte-order139%_ _%size140%_)
        (let ((_%value142%_
               (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-ref
                _%root-id138%_)))
          (if (u8vector? _%value142%_)
              (gerbil-scheme-rust/scheme/native#gerbil-rs-u8vector->sint
               _%value142%_
               _%byte-order139%_
               _%size140%_)
              '0))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-uint->bytevector-root
      (lambda (_%uint134%_ _%byte-order135%_ _%size136%_)
        (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
         (gerbil-scheme-rust/scheme/native#gerbil-rs-uint->u8vector
          _%uint134%_
          _%byte-order135%_
          _%size136%_))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-sint->bytevector-root
      (lambda (_%sint130%_ _%byte-order131%_ _%size132%_)
        (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
         (gerbil-scheme-rust/scheme/native#gerbil-rs-sint->u8vector
          _%sint130%_
          _%byte-order131%_
          _%size132%_))))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-i64-min
      (- (arithmetic-shift '1 '63)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-i64-max
      (- (arithmetic-shift '1 '63) '1))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-u64-max
      (- (arithmetic-shift '1 '64) '1))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-exact-integer?
      (lambda (_%value128%_)
        (if (integer? _%value128%_) (exact? _%value128%_) '#f)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-exact-integer-fits-i64?
      (lambda (_%value126%_)
        (if (gerbil-scheme-rust/scheme/native#gerbil-rs-exact-integer?
             _%value126%_)
            (<= gerbil-scheme-rust/scheme/native#gerbil-rs-i64-min
                _%value126%_
                gerbil-scheme-rust/scheme/native#gerbil-rs-i64-max)
            '#f)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-exact-integer-fits-u64?
      (lambda (_%value124%_)
        (if (gerbil-scheme-rust/scheme/native#gerbil-rs-exact-integer?
             _%value124%_)
            (<= '0
                _%value124%_
                gerbil-scheme-rust/scheme/native#gerbil-rs-u64-max)
            '#f)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-i64->exact-integer-root
      (lambda (_%value122%_)
        (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
         _%value122%_)))
    (define gerbil-scheme-rust/scheme/native#gerbil-rs-u64->exact-integer-root
      (lambda (_%value120%_)
        (gerbil-scheme-rust/scheme/native#gerbil-rs-rooted-value-store!
         _%value120%_)))
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
      gerbil-rs-root-exact-integer-u64-value-raw
      gerbil-rs-root-exact-integer-i64-value-raw
      gerbil-rs-root-exact-integer-fits-u64?-raw
      gerbil-rs-root-exact-integer-fits-i64?-raw
      gerbil-rs-root-exact-integer?-raw
      gerbil-rs-u64->exact-integer-root-raw
      gerbil-rs-i64->exact-integer-root-raw
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
      gerbil-rs-scheme-object-exact-integer-u64-value-raw
      gerbil-rs-scheme-object-exact-integer-i64-value-raw
      gerbil-rs-scheme-object-exact-integer-fits-u64?-raw
      gerbil-rs-scheme-object-exact-integer-fits-i64?-raw
      gerbil-rs-scheme-object-exact-integer?-raw
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
      gerbil-rs-fixture-exact-integer-large-negative-raw
      gerbil-rs-fixture-exact-integer-large-positive-raw
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
     (if (pair? value) (cdr value) #f))
    (c-declare
     "#ifndef ___HAVE_FFI_FREE\n#define ___HAVE_FFI_FREE\n___SCMOBJ ffi_free (void *ptr)\n{\n free (ptr);\n return ___FIX (___NO_ERR);\n}\n#endif")))
