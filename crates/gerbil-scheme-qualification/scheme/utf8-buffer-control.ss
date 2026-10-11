;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Frozen checked-inline control from fc4562c; qualification only.
package: gerbil-scheme-rust/qualification
(import :gerbil-scheme-rust/scheme/runtime)

(def (encode-buffer str pointer capacity)
  (let* ((length (string-length str)) (maximum (* 4 length)))
    (unless (and (fixnum? maximum) (fixnum? capacity) (>= capacity maximum))
      (error "UTF-8 caller capacity cannot admit encoding"))
    (let loop ((start 0) (written 0))
      (if (##fx< start length)
        (let* ((end (if (##fx< (##fx- length start) 256)
                       length (##fx+ start 256)))
               (next (control-encode-utf8-buffer-chunk str pointer capacity start end written)))
          (loop end next))
        written))))

(def (control-encode-utf8-buffer-chunk str pointer capacity start end written)
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

(begin-foreign
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
/* c_intf.c's output-cursor convention, within the checked local leaf only.
   Handle an empty null destination before any pointer arithmetic. */
if (start == end) return ___FIX((___S64)output_start);
___U8 *cursor = output + output_start;
for (___U64 i = start; i < end; ++i) {
  const ___U32 c = source[i];
  if (c <= 0x7f) *cursor++ = (___U8)c;
  else if (c <= 0x7ff) {
    *cursor++ = (___U8)(0xc0 | (c >> 6));
    *cursor++ = (___U8)(0x80 | (c & 0x3f));
  } else if (c <= 0xffff) {
    if ((c >> 11) == 0x1b) return ___FIX(-1);
    *cursor++ = (___U8)(0xe0 | (c >> 12));
    *cursor++ = (___U8)(0x80 | ((c >> 6) & 0x3f));
    *cursor++ = (___U8)(0x80 | (c & 0x3f));
  } else if (c <= 0x10ffff) {
    *cursor++ = (___U8)(0xf0 | (c >> 18));
    *cursor++ = (___U8)(0x80 | ((c >> 12) & 0x3f));
    *cursor++ = (___U8)(0x80 | ((c >> 6) & 0x3f));
    *cursor++ = (___U8)(0x80 | (c & 0x3f));
  } else return ___FIX(-1);
}
return ___FIX((___S64)(cursor - output));
}
END-C
 ))

(begin-foreign
 (c-define (buffer-encode root pointer capacity) (int64 (pointer void) unsigned-int64) int64
  "gerbil_utf8_buffer_encode" "extern"
  (let ((text (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root)))
   (if (string? text)
    (with-exception-catcher (lambda (_) -1)
     (lambda () (gerbil-scheme-rust/qualification/utf8-buffer-control#encode-buffer text pointer capacity)))
    -1)))
 (c-define (buffer-mutate root index codepoint) (int64 unsigned-int64 unsigned-int32) void
  "gerbil_utf8_buffer_mutate" "extern"
  (let ((text (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root)))
   (string-set! text index (##integer->char codepoint)))))
