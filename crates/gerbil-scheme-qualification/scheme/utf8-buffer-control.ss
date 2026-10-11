;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Test-only caller-owned output, following c_intf.c's bounded buffer route.
package: gerbil-scheme-rust/qualification
(import :gerbil-scheme-rust/scheme/runtime)
(extern buffer-chunk)

(def (encode-buffer text pointer capacity)
 (let* ((length (string-length text)) (maximum (* 4 length)))
  (unless (and (fixnum? maximum) (fixnum? capacity) (>= capacity maximum))
   (error "buffer capacity cannot admit encoding"))
  (let loop ((start 0) (written 0))
   (if (##fx< start length)
    (let* ((end (if (##fx< (##fx- length start) 256) length (##fx+ start 256)))
           (next (checked-chunk text pointer capacity start end written)))
     (loop end next))
    written))))

(def (checked-chunk text pointer capacity start end written)
 (declare (not interrupts-enabled))
 (let (next (buffer-chunk text pointer capacity start end written))
  (if (##fx< next 0) (error "buffer scalar or bounds rejection") next)))

(begin-foreign
 (namespace ("gerbil-scheme-rust/qualification/utf8-buffer-control#" buffer-chunk))
 (define buffer-chunk
  (c-lambda (scheme-object (pointer void) scheme-object scheme-object scheme-object scheme-object) scheme-object
   #<<C-END
___SCMOBJ ___temp;
if (!___STRINGP(___arg1) || !___FIXNUMP(___arg3) || !___FIXNUMP(___arg4) ||
    !___FIXNUMP(___arg5) || !___FIXNUMP(___arg6) ||
    ___INT(___arg3) < 0 || ___INT(___arg4) < 0 || ___INT(___arg5) < 0 || ___INT(___arg6) < 0)
 ___return(___FIX(-2));
const ___U64 length = ___STRINGSIZE(___arg1), capacity = ___INT(___arg3);
const ___U64 start = ___INT(___arg4), end = ___INT(___arg5), position = ___INT(___arg6);
if (start > end || end > length || end - start > 256 || position > capacity ||
    4 * (end - start) > capacity - position || (end > start && !___arg2))
 ___return(___FIX(-2));
/* Reacquire the Scheme body per leaf. Only the Rust allocation stays stable.
   Neither body pointer survives this allocation-free, callback-free leaf. */
const ___C *source = ___CAST(const ___C*, ___BODY_AS(___arg1, ___tSTRING));
___U8 *output = ___CAST(___U8*, ___arg2);
___U64 j = position;
for (___U64 i = start; i < end; ++i) {
 const ___U32 c = source[i];
 if (c <= 0x7f) output[j++] = (___U8)c;
 else if (c <= 0x7ff) {
  output[j++] = (___U8)(0xc0 | (c >> 6)); output[j++] = (___U8)(0x80 | (c & 0x3f));
 } else if (c <= 0xffff) {
  if ((c >> 11) == 0x1b) ___return(___FIX(-1));
  output[j++] = (___U8)(0xe0 | (c >> 12)); output[j++] = (___U8)(0x80 | ((c >> 6) & 0x3f)); output[j++] = (___U8)(0x80 | (c & 0x3f));
 } else if (c <= 0x10ffff) {
  output[j++] = (___U8)(0xf0 | (c >> 18)); output[j++] = (___U8)(0x80 | ((c >> 12) & 0x3f));
  output[j++] = (___U8)(0x80 | ((c >> 6) & 0x3f)); output[j++] = (___U8)(0x80 | (c & 0x3f));
 } else ___return(___FIX(-1));
}
___return(___FIX(j));
C-END
 ))
 (c-define (buffer-encode root pointer capacity) (int64 (pointer void) unsigned-int64) int64
  "gerbil_utf8_buffer_encode" "extern"
  (let ((text (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root)))
   (if (string? text)
    (with-exception-catcher (lambda (_) -1)
     (lambda () (gerbil-scheme-rust/qualification/utf8-buffer-control#encode-buffer text pointer capacity)))
    -1)))
 ;; Test-only mutation, including raw surrogate values. Never a public API.
 (c-define (buffer-mutate root index codepoint) (int64 unsigned-int64 unsigned-int32) void
  "gerbil_utf8_buffer_mutate" "extern"
  (let ((text (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root)))
   (string-set! text index (##integer->char codepoint)))))
