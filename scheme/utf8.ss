;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

(export gerbil-rs-encode-utf8 gerbil-rs-encode-utf8-into)

;; Bridge-local encoder: Gambit's gambit/string/string.scm codepoint rules,
;; with Gerbil std/encoding/utf16.ss's bounded allocation / single-pass shrink
;; algorithm. No SDK code is replaced. Input must be a checked Scheme string.
;; Scheme owns capacity, chunk bounds, polling, shrink and errors. The private
;; no-allocation C leaf uses Gambit's official typed-body access and UTF-8
;; shifts (gambit.h and lib/c_intf.c), restricted to Unicode scalar values.
(def (gerbil-rs-encode-utf8 str)
  (let* ((length (string-length str))
         (capacity (* 4 length)))
    (unless (fixnum? capacity)
      (error "UTF-8 capacity exceeds fixnum range" length))
    ;; Conservative rollback: filled capacity and generic bulk call framing.
    ;; Keep the strict type/range guards; never restore per-character FFI.
    (let (bytes (make-u8vector capacity))
      (let loop ((start 0) (j 0))
        (if (##fx< start length)
          (let* ((end (if (##fx< (##fx- length start) 256)
                        length (##fx+ start 256)))
                 (next (gerbil-rs-encode-utf8-chunk str bytes start end j)))
            (loop end next))
          (begin
            (when (##fx< j capacity) (##u8vector-shrink! bytes j))
            bytes))))))

;; Caller-owned storage follows c_intf.c's bounded buffer convention. Rust's
;; allocation is stable across Scheme polls; no Scheme body pointer is retained.
(def (gerbil-rs-encode-utf8-into str pointer capacity)
  (let* ((length (string-length str)) (maximum (* 4 length)))
    (unless (and (fixnum? maximum) (fixnum? capacity) (>= capacity maximum))
      (error "UTF-8 caller capacity cannot admit encoding"))
    (let loop ((start 0) (written 0))
      (if (##fx< start length)
        (let* ((end (if (##fx< (##fx- length start) 256)
                       length (##fx+ start 256)))
               (next (gerbil-rs-encode-utf8-buffer-chunk str pointer capacity start end written)))
          (loop end next))
        written))))

(def (gerbil-rs-encode-utf8-buffer-chunk str pointer capacity start end written)
  (declare (not interrupts-enabled))
  ;; c_intf.c's converter is allocation-free and retains the exact void* tag
  ;; check. Keep foreign bodies local to this interrupt-disabled leaf.
  (let (next (##c-code #<<C-END
void *destination = 0;
if (___EXT(___SCMOBJ_to_POINTER) (___PSP ___ARG2, &destination, ___ARG7, 2) != ___FIX(___NO_ERR))
  ___RESULT = ___FIX(-2);
else
  ___RESULT = gerbil_utf8_encode_into(___ARG1, ___CAST(___U8*, destination),
                                    ___ARG3, ___ARG4, ___ARG5, ___ARG6);
C-END
              str pointer capacity start end written '(void*)))
    (if (##fx< next 0) (error "Illegal Unicode scalar or caller bounds") next)))

;; One bulk call per bounded chunk, never one foreign call per character.
;; Body pointers exist only inside this synchronous, allocation-free leaf;
;; no pointer survives a return to Scheme or a possible GC/poll boundary.
(def (gerbil-rs-encode-utf8-chunk str bytes start end output-start)
  (declare (not interrupts-enabled))
  (let (next (gerbil-rs-encode-utf8-chunk-c str bytes start end output-start))
    (if (##fx< next 0)
      (error "Illegal Unicode scalar or UTF-8 chunk bounds" start end)
      next)))

(extern gerbil-rs-encode-utf8-chunk-c gerbil-rs-encode-utf8-buffer-chunk-c)

(begin-foreign
 (namespace ("gerbil-scheme-rust/scheme/utf8#" gerbil-rs-encode-utf8-chunk-c gerbil-rs-encode-utf8-buffer-chunk-c))
 (c-declare #<<END-C
static ___SCMOBJ gerbil_utf8_encode_into(___SCMOBJ str, ___U8 *output,
                                       ___SCMOBJ size, ___SCMOBJ first,
                                       ___SCMOBJ last, ___SCMOBJ position) {
/* Checked private leaf: source and destination are distinct typed objects. */
___SCMOBJ ___temp; /* Gambit's subtype predicates use this scratch word. */
if (!___STRINGP(str) || !___FIXNUMP(size) || ___INT(size) < 0 ||
    !___FIXNUMP(first) || !___FIXNUMP(last) || !___FIXNUMP(position) ||
    ___INT(first) < 0 || ___INT(last) < 0 || ___INT(position) < 0)
  return ___FIX(-2);
const ___U64 length = ___STRINGSIZE(str);
const ___U64 capacity = ___INT(size);
const ___U64 start = ___INT(first), end = ___INT(last), output_start = ___INT(position);
if (start > end || end > length || end - start > 256 ||
    output_start > capacity || 4 * (end - start) > capacity - output_start ||
    (end > start && !output))
  return ___FIX(-2);
const ___C *source = ___CAST(const ___C*, ___BODY_AS(str, ___tSTRING));
___U64 j = output_start;
for (___U64 i = start; i < end; ++i) {
  const ___U32 c = source[i];
  if (c <= 0x7f) output[j++] = (___U8)c;
  else if (c <= 0x7ff) {
    output[j++] = (___U8)(0xc0 | (c >> 6));
    output[j++] = (___U8)(0x80 | (c & 0x3f));
  } else if (c <= 0xffff) {
    if ((c >> 11) == 0x1b) return ___FIX(-1);
    output[j++] = (___U8)(0xe0 | (c >> 12));
    output[j++] = (___U8)(0x80 | ((c >> 6) & 0x3f));
    output[j++] = (___U8)(0x80 | (c & 0x3f));
  } else if (c <= 0x10ffff) {
    output[j++] = (___U8)(0xf0 | (c >> 18));
    output[j++] = (___U8)(0x80 | ((c >> 12) & 0x3f));
    output[j++] = (___U8)(0x80 | ((c >> 6) & 0x3f));
    output[j++] = (___U8)(0x80 | (c & 0x3f));
  } else return ___FIX(-1);
}
return ___FIX((___S64)j);
}
static ___SCMOBJ gerbil_utf8_encode_chunk(___SCMOBJ str, ___SCMOBJ bytes,
                                        ___SCMOBJ first, ___SCMOBJ last,
                                        ___SCMOBJ position) {
  ___SCMOBJ ___temp;
  if (!___U8VECTORP(bytes)) return ___FIX(-2);
  return gerbil_utf8_encode_into(str,
    ___CAST(___U8*, ___BODY_AS(bytes, ___tU8VECTOR)),
    ___FIX(___U8VECTORSIZE(bytes)), first, last, position);
}
END-C
 )
 (define gerbil-rs-encode-utf8-chunk-c
  (c-lambda (scheme-object scheme-object scheme-object scheme-object scheme-object) scheme-object
   "___return(gerbil_utf8_encode_chunk(___arg1, ___arg2, ___arg3, ___arg4, ___arg5));"))
 (define gerbil-rs-encode-utf8-buffer-chunk-c
  (c-lambda (scheme-object (pointer void) scheme-object scheme-object scheme-object scheme-object) scheme-object
   "___return(gerbil_utf8_encode_into(___arg1, ___CAST(___U8*, ___arg2), ___arg3, ___arg4, ___arg5, ___arg6));")))
