;; gerbil-scheme-rust.generated-scm-provenance.v1 input-sha256=6e941a1456c827a8d3132bfc20efffc9db655458ee5384f5a26ee63b23de5e15 body-sha256=d679de9f519b5243bc037db5a1daba15de99a719f85c81242aa73930d3d18ba1
(declare (block) (standard-bindings) (extended-bindings))
(begin
  (define gerbil-scheme-rust/scheme/utf8::timestamp 1791689652)
  (begin
    (define gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8
      (lambda (_%str485%_)
        (let* ((_%length487%_ (string-length _%str485%_))
               (_%capacity489%_ (* '4 _%length487%_)))
          (if (fixnum? _%capacity489%_)
              '#!void
              (let ()
                (declare (not safe))
                (error '"UTF-8 capacity exceeds fixnum range" _%length487%_)))
          (let ((_%bytes492%_ (make-u8vector _%capacity489%_)))
            (let _%loop494%_ ((_%start496%_ '0) (_%j497%_ '0))
              (if (let ()
                    (declare (not safe))
                    (##fx< _%start496%_ _%length487%_))
                  (let* ((_%end499%_
                          (if (let ((__tmp4972
                                     (let ()
                                       (declare (not safe))
                                       (##fx- _%length487%_ _%start496%_))))
                                (declare (not safe))
                                (##fx< __tmp4972 '256))
                              _%length487%_
                              (let ()
                                (declare (not safe))
                                (##fx+ _%start496%_ '256))))
                         (_%next501%_
                          (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-chunk
                           _%str485%_
                           _%bytes492%_
                           _%start496%_
                           _%end499%_
                           _%j497%_)))
                    (_%loop494%_ _%end499%_ _%next501%_))
                  (begin
                    (if (let ()
                          (declare (not safe))
                          (##fx< _%j497%_ _%capacity489%_))
                        (let ()
                          (declare (not safe))
                          (##u8vector-shrink! _%bytes492%_ _%j497%_))
                        '#!void)
                    _%bytes492%_)))))))
    (define gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-into
      (lambda (_%str466%_ _%pointer467%_ _%capacity468%_)
        (let* ((_%length470%_ (string-length _%str466%_))
               (_%maximum472%_ (* '4 _%length470%_)))
          (if (and (fixnum? _%maximum472%_)
                   (fixnum? _%capacity468%_)
                   (>= _%capacity468%_ _%maximum472%_))
              '#!void
              (let ()
                (declare (not safe))
                (error '"UTF-8 caller capacity cannot admit encoding")))
          (let _%loop475%_ ((_%start477%_ '0) (_%written478%_ '0))
            (if (let ()
                  (declare (not safe))
                  (##fx< _%start477%_ _%length470%_))
                (let* ((_%end480%_
                        (if (let ((__tmp4973
                                   (let ()
                                     (declare (not safe))
                                     (##fx- _%length470%_ _%start477%_))))
                              (declare (not safe))
                              (##fx< __tmp4973 '256))
                            _%length470%_
                            (let ()
                              (declare (not safe))
                              (##fx+ _%start477%_ '256))))
                       (_%next482%_
                        (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-buffer-chunk
                         _%str466%_
                         _%pointer467%_
                         _%capacity468%_
                         _%start477%_
                         _%end480%_
                         _%written478%_)))
                  (_%loop475%_ _%end480%_ _%next482%_))
                _%written478%_)))))
    (define gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-buffer-chunk
      (lambda (_%str456%_
               _%pointer457%_
               _%capacity458%_
               _%start459%_
               _%end460%_
               _%written461%_)
        (declare (not interrupts-enabled))
        (let ((_%next464%_
               (let ()
                 (declare (not safe))
                 (##c-code
                  '"void *destination = 0;\nif (___EXT(___SCMOBJ_to_POINTER) (___PSP ___ARG2, &destination, ___ARG7, 2) != ___FIX(___NO_ERR))\n  ___RESULT = ___FIX(-2);\nelse\n  ___RESULT = gerbil_utf8_encode_into(___ARG1, ___CAST(___U8*, destination),\n                                    ___ARG3, ___ARG4, ___ARG5, ___ARG6);"
                  _%str456%_
                  _%pointer457%_
                  _%capacity458%_
                  _%start459%_
                  _%end460%_
                  _%written461%_
                  '(void*)))))
          (if (let () (declare (not safe)) (##fx< _%next464%_ '0))
              (let ()
                (declare (not safe))
                (error '"Illegal Unicode scalar or caller bounds"))
              _%next464%_))))
    (define gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-chunk
      (lambda (_%str447%_
               _%bytes448%_
               _%start449%_
               _%end450%_
               _%output-start451%_)
        (declare (not interrupts-enabled))
        (let ((_%next454%_
               (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-chunk-c
                _%str447%_
                _%bytes448%_
                _%start449%_
                _%end450%_
                _%output-start451%_)))
          (if (let () (declare (not safe)) (##fx< _%next454%_ '0))
              (let ()
                (declare (not safe))
                (error '"Illegal Unicode scalar or UTF-8 chunk bounds"
                       _%start449%_
                       _%end450%_))
              _%next454%_))))
    (namespace
     ("gerbil-scheme-rust/scheme/utf8#"
      gerbil-rs-encode-utf8-chunk-c
      gerbil-rs-encode-utf8-buffer-chunk-c))
    (c-declare
     "static ___SCMOBJ gerbil_utf8_encode_into(___SCMOBJ str, ___U8 *output,\n                                       ___SCMOBJ size, ___SCMOBJ first,\n                                       ___SCMOBJ last, ___SCMOBJ position) {\n/* Checked private leaf: source and destination are distinct typed objects. */\n___SCMOBJ ___temp; /* Gambit's subtype predicates use this scratch word. */\nif (!___STRINGP(str) || !___FIXNUMP(size) || ___INT(size) < 0 ||\n    !___FIXNUMP(first) || !___FIXNUMP(last) || !___FIXNUMP(position) ||\n    ___INT(first) < 0 || ___INT(last) < 0 || ___INT(position) < 0)\n  return ___FIX(-2);\nconst ___U64 length = ___STRINGSIZE(str);\nconst ___U64 capacity = ___INT(size);\nconst ___U64 start = ___INT(first), end = ___INT(last), output_start = ___INT(position);\nif (start > end || end > length || end - start > 256 ||\n    output_start > capacity || 4 * (end - start) > capacity - output_start ||\n    (end > start && !output))\n  return ___FIX(-2);\nconst ___C *source = ___CAST(const ___C*, ___BODY_AS(str, ___tSTRING));\n___U64 j = output_start;\nfor (___U64 i = start; i < end; ++i) {\n  const ___U32 c = source[i];\n  if (c <= 0x7f) output[j++] = (___U8)c;\n  else if (c <= 0x7ff) {\n    output[j++] = (___U8)(0xc0 | (c >> 6));\n    output[j++] = (___U8)(0x80 | (c & 0x3f));\n  } else if (c <= 0xffff) {\n    if ((c >> 11) == 0x1b) return ___FIX(-1);\n    output[j++] = (___U8)(0xe0 | (c >> 12));\n    output[j++] = (___U8)(0x80 | ((c >> 6) & 0x3f));\n    output[j++] = (___U8)(0x80 | (c & 0x3f));\n  } else if (c <= 0x10ffff) {\n    output[j++] = (___U8)(0xf0 | (c >> 18));\n    output[j++] = (___U8)(0x80 | ((c >> 12) & 0x3f));\n    output[j++] = (___U8)(0x80 | ((c >> 6) & 0x3f));\n    output[j++] = (___U8)(0x80 | (c & 0x3f));\n  } else return ___FIX(-1);\n}\nreturn ___FIX((___S64)j);\n}\nstatic ___SCMOBJ gerbil_utf8_encode_chunk(___SCMOBJ str, ___SCMOBJ bytes,\n                                        ___SCMOBJ first, ___SCMOBJ last,\n                                        ___SCMOBJ position) {\n  ___SCMOBJ ___temp;\n  if (!___U8VECTORP(bytes)) return ___FIX(-2);\n  return gerbil_utf8_encode_into(str,\n    ___CAST(___U8*, ___BODY_AS(bytes, ___tU8VECTOR)),\n    ___FIX(___U8VECTORSIZE(bytes)), first, last, position);\n}")
    (define gerbil-rs-encode-utf8-chunk-c
      (c-lambda
       (scheme-object scheme-object scheme-object scheme-object scheme-object)
       scheme-object
       "___return(gerbil_utf8_encode_chunk(___arg1, ___arg2, ___arg3, ___arg4, ___arg5));"))
    (define gerbil-rs-encode-utf8-buffer-chunk-c
      (c-lambda
       (scheme-object
        (pointer void)
        scheme-object
        scheme-object
        scheme-object
        scheme-object)
       scheme-object
       "___return(gerbil_utf8_encode_into(___arg1, ___CAST(___U8*, ___arg2), ___arg3, ___arg4, ___arg5, ___arg6));"))))
