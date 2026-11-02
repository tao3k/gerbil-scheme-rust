(declare (block) (standard-bindings) (extended-bindings))
(begin
  (define gerbil-scheme-rust/scheme/utf8::timestamp 1793626237)
  (begin
    (define gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8
      (lambda (_%str513%_)
        (let* ((_%length515%_ (string-length _%str513%_))
               (_%capacity517%_ (* '4 _%length515%_)))
          (if (fixnum? _%capacity517%_)
              '#!void
              (let ()
                (declare (not safe))
                (error '"UTF-8 capacity exceeds fixnum range" _%length515%_)))
          (let ((_%bytes520%_ (make-u8vector _%capacity517%_)))
            (let _%loop522%_ ((_%start524%_ '0) (_%j525%_ '0))
              (if (let ()
                    (declare (not safe))
                    (##fx< _%start524%_ _%length515%_))
                  (let* ((_%end527%_
                          (if (let ((__tmp9145
                                     (let ()
                                       (declare (not safe))
                                       (##fx- _%length515%_ _%start524%_))))
                                (declare (not safe))
                                (##fx< __tmp9145 '256))
                              _%length515%_
                              (let ()
                                (declare (not safe))
                                (##fx+ _%start524%_ '256))))
                         (_%next529%_
                          (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-chunk
                           _%str513%_
                           _%bytes520%_
                           _%start524%_
                           _%end527%_
                           _%j525%_)))
                    (_%loop522%_ _%end527%_ _%next529%_))
                  (begin
                    (if (let ()
                          (declare (not safe))
                          (##fx< _%j525%_ _%capacity517%_))
                        (let ()
                          (declare (not safe))
                          (##u8vector-shrink! _%bytes520%_ _%j525%_))
                        '#!void)
                    _%bytes520%_)))))))
    (define gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-into
      (lambda (_%str497%_ _%pointer498%_ _%capacity499%_)
        (let ((_%length501%_ (string-length _%str497%_)))
          (if (and (fixnum? _%capacity499%_) (>= _%capacity499%_ '0))
              '#!void
              (let ()
                (declare (not safe))
                (error '"UTF-8 caller capacity cannot admit encoding")))
          (let _%loop503%_ ((_%start505%_ '0) (_%written506%_ '0))
            (if (let ()
                  (declare (not safe))
                  (##fx< _%start505%_ _%length501%_))
                (let* ((_%end508%_
                        (if (let ((__tmp9146
                                   (let ()
                                     (declare (not safe))
                                     (##fx- _%length501%_ _%start505%_))))
                              (declare (not safe))
                              (##fx< __tmp9146 '256))
                            _%length501%_
                            (let ()
                              (declare (not safe))
                              (##fx+ _%start505%_ '256))))
                       (_%next510%_
                        (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-buffer-chunk
                         _%str497%_
                         _%pointer498%_
                         _%capacity499%_
                         _%start505%_
                         _%end508%_
                         _%written506%_)))
                  (_%loop503%_ _%end508%_ _%next510%_))
                _%written506%_)))))
    (define gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-buffer-chunk
      (lambda (_%str487%_
               _%pointer488%_
               _%capacity489%_
               _%start490%_
               _%end491%_
               _%written492%_)
        (declare (not interrupts-enabled))
        (let ((_%next495%_
               (let ()
                 (declare (not safe))
                 (##c-code
                  '"void *destination = 0;\nif (___EXT(___SCMOBJ_to_POINTER) (___PSP ___ARG2, &destination, ___ARG7, 2) != ___FIX(___NO_ERR))\n  ___RESULT = ___FIX(-2);\nelse\n  ___RESULT = gerbil_utf8_encode_into(___ARG1, ___CAST(___U8*, destination),\n                                    ___ARG3, ___ARG4, ___ARG5, ___ARG6);"
                  _%str487%_
                  _%pointer488%_
                  _%capacity489%_
                  _%start490%_
                  _%end491%_
                  _%written492%_
                  '(void*)))))
          (if (let () (declare (not safe)) (##fx< _%next495%_ '0))
              (let ()
                (declare (not safe))
                (error '"Illegal Unicode scalar or caller bounds"))
              _%next495%_))))
    (define gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-chunk
      (lambda (_%str478%_
               _%bytes479%_
               _%start480%_
               _%end481%_
               _%output-start482%_)
        (declare (not interrupts-enabled))
        (let ((_%next485%_
               (gerbil-scheme-rust/scheme/utf8#gerbil-rs-encode-utf8-chunk-c
                _%str478%_
                _%bytes479%_
                _%start480%_
                _%end481%_
                _%output-start482%_)))
          (if (let () (declare (not safe)) (##fx< _%next485%_ '0))
              (let ()
                (declare (not safe))
                (error '"Illegal Unicode scalar or UTF-8 chunk bounds"
                       _%start480%_
                       _%end481%_))
              _%next485%_))))
    (namespace
     ("gerbil-scheme-rust/scheme/utf8#"
      gerbil-rs-encode-utf8-chunk-c
      gerbil-rs-encode-utf8-buffer-chunk-c))
    (c-declare
     "static ___SCMOBJ gerbil_utf8_encode_into(___SCMOBJ str, ___U8 *output,\n                                       ___SCMOBJ size, ___SCMOBJ first,\n                                       ___SCMOBJ last, ___SCMOBJ position) {\n/* Checked private leaf: source and destination are distinct typed objects. */\n___SCMOBJ ___temp; /* Gambit's subtype predicates use this scratch word. */\nif (!___STRINGP(str) || !___FIXNUMP(size) || ___INT(size) < 0 ||\n    !___FIXNUMP(first) || !___FIXNUMP(last) || !___FIXNUMP(position) ||\n    ___INT(first) < 0 || ___INT(last) < 0 || ___INT(position) < 0)\n  return ___FIX(-2);\nconst ___U64 length = ___STRINGSIZE(str);\nconst ___U64 capacity = ___INT(size);\nconst ___U64 start = ___INT(first), end = ___INT(last), output_start = ___INT(position);\nif (start > end || end > length || end - start > 256 ||\n    output_start > capacity ||\n    (end > start && !output))\n  return ___FIX(-2);\nconst ___C *source = ___CAST(const ___C*, ___BODY_AS(str, ___tSTRING));\n/* Preflight at most 256 scalars before writing this chunk. Keep required <=\n   available, so subtraction cannot underflow; required is at most 1024 bytes.\n   A caller needs the actual byte capacity, not the four-byte upper bound. */\nconst ___U64 available = capacity - output_start;\n___U64 required = 0;\nfor (___U64 i = start; i < end; ++i) {\n  const ___U32 c = source[i];\n  ___U64 width;\n  if (c <= 0x7f) width = 1;\n  else if (c <= 0x7ff) width = 2;\n  else if (c <= 0xffff) {\n    if ((c >> 11) == 0x1b) return ___FIX(-1);\n    width = 3;\n  } else if (c <= 0x10ffff) width = 4;\n  else return ___FIX(-1);\n  if (width > available - required) return ___FIX(-2);\n  required += width;\n}\n/* c_intf.c's output-cursor convention, within the checked local leaf only.\n   Handle an empty null destination before any pointer arithmetic. */\nif (start == end) return ___FIX((___S64)output_start);\n___U8 *cursor = output + output_start;\nfor (___U64 i = start; i < end; ++i) {\n  const ___U32 c = source[i];\n  if (c <= 0x7f) *cursor++ = (___U8)c;\n  else if (c <= 0x7ff) {\n    *cursor++ = (___U8)(0xc0 | (c >> 6));\n    *cursor++ = (___U8)(0x80 | (c & 0x3f));\n  } else if (c <= 0xffff) {\n    if ((c >> 11) == 0x1b) return ___FIX(-1);\n    *cursor++ = (___U8)(0xe0 | (c >> 12));\n    *cursor++ = (___U8)(0x80 | ((c >> 6) & 0x3f));\n    *cursor++ = (___U8)(0x80 | (c & 0x3f));\n  } else if (c <= 0x10ffff) {\n    *cursor++ = (___U8)(0xf0 | (c >> 18));\n    *cursor++ = (___U8)(0x80 | ((c >> 12) & 0x3f));\n    *cursor++ = (___U8)(0x80 | ((c >> 6) & 0x3f));\n    *cursor++ = (___U8)(0x80 | (c & 0x3f));\n  } else return ___FIX(-1);\n}\nreturn ___FIX((___S64)(cursor - output));\n}\nstatic ___SCMOBJ gerbil_utf8_encode_chunk(___SCMOBJ str, ___SCMOBJ bytes,\n                                        ___SCMOBJ first, ___SCMOBJ last,\n                                        ___SCMOBJ position) {\n  ___SCMOBJ ___temp;\n  if (!___U8VECTORP(bytes)) return ___FIX(-2);\n  return gerbil_utf8_encode_into(str,\n    ___CAST(___U8*, ___BODY_AS(bytes, ___tU8VECTOR)),\n    ___FIX(___U8VECTORSIZE(bytes)), first, last, position);\n}")
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
