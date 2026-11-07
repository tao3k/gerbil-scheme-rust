package: gerbil-scheme-rust/qualification
;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

(export inline-control-encode)

;; Bridge-local encoder: Gambit's gambit/string/string.scm codepoint rules,
;; with Gerbil std/encoding/utf16.ss's bounded allocation / single-pass shrink
;; algorithm. No SDK code is replaced. Input must be a checked Scheme string.
;; Scheme owns capacity, chunk bounds, polling, shrink and errors. The private
;; no-allocation C leaf uses Gambit's official typed-body access and UTF-8
;; shifts (gambit.h and lib/c_intf.c), restricted to Unicode scalar values.
(def (inline-control-encode str)
  (let* ((length (string-length str))
         (capacity (* 4 length)))
    (unless (fixnum? capacity)
      (error "UTF-8 capacity exceeds fixnum range" length))
    ;; Official ##make-u8vector without a fill skips initialization. Every
    ;; published byte is written by the leaf; unused capacity is shrunk away
    ;; before returning. A failed leaf never publishes the partial buffer.
    (let (bytes (##make-u8vector capacity))
      (let loop ((start 0) (j 0))
        (if (##fx< start length)
          (let* ((end (if (##fx< (##fx- length start) 256)
                        length (##fx+ start 256)))
                 (next (inline-control-encode-chunk str bytes start end j)))
            (loop end next))
          (begin
            (when (##fx< j capacity) (##u8vector-shrink! bytes j))
            bytes))))))

;; One bulk call per bounded chunk, never one foreign call per character.
;; Body pointers exist only inside this synchronous, allocation-free leaf;
;; no pointer survives a return to Scheme or a possible GC/poll boundary.
(def (inline-control-encode-chunk str bytes start end output-start)
  (declare (not interrupts-enabled))
  (let (next (inline-control-encode-chunk-c str bytes start end output-start))
    (if (##fx< next 0)
      (error "Illegal Unicode scalar or UTF-8 chunk bounds" start end)
      next)))

(def (inline-control-encode-chunk-c str bytes start end output-start)
  (declare (not interrupts-enabled))
  ;; std/ffi's def-C uses this same allocation-free ##c-code mechanism.
  ;; Keep full object/range checks in the helper, without c-lambda's generic
  ;; numeric conversions and C/Scheme call-return frame on every chunk.
  (##c-code
   "___RESULT = comparison_inline_encode_chunk(___ARG1, ___ARG2, ___ARG3, ___ARG4, ___ARG5);"
   str bytes start end output-start))

(begin-foreign
 (c-declare #<<END-C
static ___SCMOBJ comparison_inline_encode_chunk(___SCMOBJ str, ___SCMOBJ bytes,
                                        ___SCMOBJ first, ___SCMOBJ last,
                                        ___SCMOBJ position) {
/* Checked private leaf: source and destination are distinct typed objects. */
___SCMOBJ ___temp; /* Gambit's subtype predicates use this scratch word. */
if (!___STRINGP(str) || !___U8VECTORP(bytes) ||
    !___FIXNUMP(first) || !___FIXNUMP(last) || !___FIXNUMP(position) ||
    ___INT(first) < 0 || ___INT(last) < 0 || ___INT(position) < 0)
  return ___FIX(-2);
const ___U64 length = ___STRINGSIZE(str);
const ___U64 capacity = ___U8VECTORSIZE(bytes);
const ___U64 start = ___INT(first), end = ___INT(last), output_start = ___INT(position);
if (start > end || end > length || end - start > 256 ||
    output_start > capacity || 4 * (end - start) > capacity - output_start)
  return ___FIX(-2);
const ___C *source = ___CAST(const ___C*, ___BODY_AS(str, ___tSTRING));
___U8 *output = ___CAST(___U8*, ___BODY_AS(bytes, ___tU8VECTOR));
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
END-C
 ))
