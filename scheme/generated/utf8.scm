(declare (block) (standard-bindings) (extended-bindings))
(begin
  (define gerbil-scheme-rust/scheme/utf8::timestamp 1793610014)
  (begin
    (define gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8
      (lambda (_%str488%_)
        (let* ((_%length490%_ (string-length _%str488%_))
               (_%capacity492%_ (* '4 _%length490%_)))
          (if (fixnum? _%capacity492%_)
              '#!void
              (let ()
                (declare (not safe))
                (error '"UTF-8 capacity exceeds fixnum range" _%length490%_)))
          (let ((_%bytes495%_ (make-u8vector _%capacity492%_)))
            (let _%loop497%_ ((_%start499%_ '0) (_%j500%_ '0))
              (if (let ()
                    (declare (not safe))
                    (##fx< _%start499%_ _%length490%_))
                  (let* ((_%end502%_
                          (if (let ((__tmp9030
                                     (let ()
                                       (declare (not safe))
                                       (##fx- _%length490%_ _%start499%_))))
                                (declare (not safe))
                                (##fx< __tmp9030 '256))
                              _%length490%_
                              (let ()
                                (declare (not safe))
                                (##fx+ _%start499%_ '256))))
                         (_%next504%_
                          (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-chunk
                           _%str488%_
                           _%bytes495%_
                           _%start499%_
                           _%end502%_
                           _%j500%_)))
                    (_%loop497%_ _%end502%_ _%next504%_))
                  (begin
                    (if (let ()
                          (declare (not safe))
                          (##fx< _%j500%_ _%capacity492%_))
                        (let ()
                          (declare (not safe))
                          (##u8vector-shrink! _%bytes495%_ _%j500%_))
                        '#!void)
                    _%bytes495%_)))))))
    (define gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-into
      (lambda (_%str469%_ _%pointer470%_ _%capacity471%_)
        (let* ((_%length473%_ (string-length _%str469%_))
               (_%maximum475%_ (* '4 _%length473%_)))
          (if (and (fixnum? _%maximum475%_)
                   (fixnum? _%capacity471%_)
                   (>= _%capacity471%_ _%maximum475%_))
              '#!void
              (let ()
                (declare (not safe))
                (error '"UTF-8 caller capacity cannot admit encoding")))
          (let _%loop478%_ ((_%start480%_ '0) (_%written481%_ '0))
            (if (let ()
                  (declare (not safe))
                  (##fx< _%start480%_ _%length473%_))
                (let* ((_%end483%_
                        (if (let ((__tmp9031
                                   (let ()
                                     (declare (not safe))
                                     (##fx- _%length473%_ _%start480%_))))
                              (declare (not safe))
                              (##fx< __tmp9031 '256))
                            _%length473%_
                            (let ()
                              (declare (not safe))
                              (##fx+ _%start480%_ '256))))
                       (_%next485%_
                        (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-buffer-chunk
                         _%str469%_
                         _%pointer470%_
                         _%capacity471%_
                         _%start480%_
                         _%end483%_
                         _%written481%_)))
                  (_%loop478%_ _%end483%_ _%next485%_))
                _%written481%_)))))
    (define gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-buffer-chunk
      (lambda (_%str459%_
               _%pointer460%_
               _%capacity461%_
               _%start462%_
               _%end463%_
               _%written464%_)
        (declare (not interrupts-enabled))
        (let ((_%next467%_
               (let ()
                 (declare (not safe))
                 (##c-code
                  '"void *destination = 0;\nif (___EXT(___SCMOBJ_to_POINTER) (___PSP ___ARG2, &destination, ___ARG7, 2) != ___FIX(___NO_ERR))\n  ___RESULT = ___FIX(-2);\nelse\n  ___RESULT = gerbil_utf8_encode_into(___ARG1, ___CAST(___U8*, destination),\n                                    ___ARG3, ___ARG4, ___ARG5, ___ARG6);"
                  _%str459%_
                  _%pointer460%_
                  _%capacity461%_
                  _%start462%_
                  _%end463%_
                  _%written464%_
                  '(void*)))))
          (if (let () (declare (not safe)) (##fx< _%next467%_ '0))
              (let ()
                (declare (not safe))
                (error '"Illegal Unicode scalar or caller bounds"))
              _%next467%_))))
    (define gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-chunk
      (lambda (_%str450%_
               _%bytes451%_
               _%start452%_
               _%end453%_
               _%output-start454%_)
        (declare (not interrupts-enabled))
        (let ((_%next457%_
               (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-chunk-c
                _%str450%_
                _%bytes451%_
                _%start452%_
                _%end453%_
                _%output-start454%_)))
          (if (let () (declare (not safe)) (##fx< _%next457%_ '0))
              (let ()
                (declare (not safe))
                (error '"Illegal Unicode scalar or UTF-8 chunk bounds"
                       _%start452%_
                       _%end453%_))
              _%next457%_))))
    (namespace
     ("gerbil-scheme-rust/scheme/utf8#"
      gerbil-rs-encode-utf8-chunk-c
      gerbil-rs-encode-utf8-buffer-chunk-c))
    (c-declare
     "static ___SCMOBJ gerbil_utf8_encode_into(___SCMOBJ str, ___U8 *output,\n                                       ___SCMOBJ size, ___SCMOBJ first,\n                                       ___SCMOBJ last, ___SCMOBJ position) {\n/* Checked private leaf: source and destination are distinct typed objects. */\n___SCMOBJ ___temp; /* Gambit's subtype predicates use this scratch word. */\nif (!___STRINGP(str) || !___FIXNUMP(size) || ___INT(size) < 0 ||\n    !___FIXNUMP(first) || !___FIXNUMP(last) || !___FIXNUMP(position) ||\n    ___INT(first) < 0 || ___INT(last) < 0 || ___INT(position) < 0)\n  return ___FIX(-2);\nconst ___U64 length = ___STRINGSIZE(str);\nconst ___U64 capacity = ___INT(size);\nconst ___U64 start = ___INT(first), end = ___INT(last), output_start = ___INT(position);\nif (start > end || end > length || end - start > 256 ||\n    output_start > capacity || 4 * (end - start) > capacity - output_start ||\n    (end > start && !output))\n  return ___FIX(-2);\nconst ___C *source = ___CAST(const ___C*, ___BODY_AS(str, ___tSTRING));\n/* c_intf.c's output-cursor convention, within the checked local leaf only.\n   Handle an empty null destination before any pointer arithmetic. */\nif (start == end) return ___FIX((___S64)output_start);\n___U8 *cursor = output + output_start;\nfor (___U64 i = start; i < end; ++i) {\n  const ___U32 c = source[i];\n  if (c <= 0x7f) *cursor++ = (___U8)c;\n  else if (c <= 0x7ff) {\n    *cursor++ = (___U8)(0xc0 | (c >> 6));\n    *cursor++ = (___U8)(0x80 | (c & 0x3f));\n  } else if (c <= 0xffff) {\n    if ((c >> 11) == 0x1b) return ___FIX(-1);\n    *cursor++ = (___U8)(0xe0 | (c >> 12));\n    *cursor++ = (___U8)(0x80 | ((c >> 6) & 0x3f));\n    *cursor++ = (___U8)(0x80 | (c & 0x3f));\n  } else if (c <= 0x10ffff) {\n    *cursor++ = (___U8)(0xf0 | (c >> 18));\n    *cursor++ = (___U8)(0x80 | ((c >> 12) & 0x3f));\n    *cursor++ = (___U8)(0x80 | ((c >> 6) & 0x3f));\n    *cursor++ = (___U8)(0x80 | (c & 0x3f));\n  } else return ___FIX(-1);\n}\nreturn ___FIX((___S64)(cursor - output));\n}\nstatic ___SCMOBJ gerbil_utf8_encode_chunk(___SCMOBJ str, ___SCMOBJ bytes,\n                                        ___SCMOBJ first, ___SCMOBJ last,\n                                        ___SCMOBJ position) {\n  ___SCMOBJ ___temp;\n  if (!___U8VECTORP(bytes)) return ___FIX(-2);\n  return gerbil_utf8_encode_into(str,\n    ___CAST(___U8*, ___BODY_AS(bytes, ___tU8VECTOR)),\n    ___FIX(___U8VECTORSIZE(bytes)), first, last, position);\n}")
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
