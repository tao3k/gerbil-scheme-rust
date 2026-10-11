;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Test-only historical control and allocation candidate, not runtime choices.
package: gerbil-scheme-rust/qualification
(import :gerbil-scheme-rust/scheme/runtime "utf8-inline-control")
(export utf8-conformance)
(extern namespace: #f
 gerbil-scheme-rust/qualification/utf8-inline-control#inline-control-encode-chunk-c
 gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref)
(extern historical-chunk)

;; Frozen pre-inline/pre-no-fill encoder. Only the qualification graph links it.
(def (historical-checked text bytes start end written)
 (declare (not interrupts-enabled))
 (let (next (historical-chunk text bytes start end written))
  (if (##fx< next 0) (error "historical scalar rejection") next)))
(def (historical text)
 (let* ((length (string-length text)) (capacity (* 4 length)))
  (unless (fixnum? capacity) (error "historical capacity overflow"))
  (let (bytes (make-u8vector capacity))
  (let loop ((start 0) (written 0))
   (if (##fx< start length)
    (let* ((end (if (##fx< (##fx- length start) 256) length (##fx+ start 256)))
           (next (historical-checked text bytes start end written)))
     (loop end next))
    (begin (when (##fx< written capacity) (##u8vector-shrink! bytes written)) bytes))))))

(def (count-chunk text start end)
 (declare (not interrupts-enabled))
 (##c-code "___RESULT = comparison_utf8_count(___ARG1, ___ARG2, ___ARG3);" text start end))

;; Count every current character, not a stable snapshot. Reserve one bounded
;; chunk's worst-case space so the unchanged encoding leaf's preflight remains
;; valid even for the final partial chunk; shrink only after all writes finish.
(def (sized text)
 (let* ((length (string-length text)) (maximum (* 4 length)))
  (unless (fixnum? maximum) (error "candidate capacity overflow"))
  (let* ((size
          (let count ((start 0) (total 0))
           (if (##fx< start length)
            (let* ((end (if (##fx< (##fx- length start) 256) length (##fx+ start 256))) (next (count-chunk text start end)))
             (when (##fx< next 0) (error "candidate scalar rejection"))
             (count end (##fx+ total next)))
            total)))
         (capacity (min maximum (+ size 1024)))
         (bytes (##make-u8vector capacity)))
   (let loop ((start 0) (written 0))
    (if (##fx< start length)
     (let* ((end (if (##fx< (##fx- length start) 256) length (##fx+ start 256)))
            (next (gerbil-scheme-rust/qualification/utf8-inline-control#inline-control-encode-chunk-c
                   text bytes start end written)))
      (when (##fx< next 0) (error "candidate bounds or scalar rejection"))
      (loop end next))
     (begin (##u8vector-shrink! bytes written) bytes))))))

(def (encode mode text)
 (case mode ((0) (historical text)) ((1) (inline-control-encode text))
       ((2) (sized text)) (else (error "unknown encoding control" mode))))

(def (utf8-conformance)
 (let loop ((start 0))
  (when (< start #x110000)
   (let* ((end (min (+ start 4096) #x110000))
          (chars (let collect ((c start) (out '()))
                   (cond ((= c end) (reverse out))
                         ((<= #xd800 c #xdfff) (collect (+ c 1) out))
                         (else (collect (+ c 1) (cons (integer->char c) out))))))
          (text (list->string chars)) (expected (string->utf8 text)))
    (for-each (lambda (mode) (unless (equal? (encode mode text) expected)
                              (error "comparison Unicode mismatch" mode start))) '(0 1 2))
    (loop end))))
 (for-each
  (lambda (mode)
   (for-each (lambda (text)
              (unless (equal? (encode mode text) (string->utf8 text))
                (error "comparison short input mismatch" mode)))
             (list "" "a" (make-string 8192 #\a) "\x00;汉😀"))
   (let ((text (make-string 4096 #\a)))
    (for-each (lambda (c) (string-set! text 2047 (integer->char c))
                (unless (equal? (encode mode text) (string->utf8 text))
                  (error "comparison mutable input mismatch" mode)))
              '(0 127 128 2047 2048 55295 57344 65535 65536 1114111)))
   (let reject ((c #xd800))
    (when (<= c #xdfff)
     (unless (with-exception-catcher (lambda (_) #t)
               (lambda () (encode mode (string (##integer->char c))) #f))
       (error "comparison accepted surrogate" mode c))
     (reject (+ c 1))))) '(0 1 2))
 (displayln "UTF8-MATCHED-CONFORMANCE modes=3 Unicode=1112064 surrogates=2048 mutation=OK"))

(def snapshots (make-vector 2 #f))
(begin-foreign
(c-define (comparison-encode root mode) (int64 int32) int64
 "gerbil_utf8_comparison_encode" "extern"
 (let ((text (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root)))
  (if (string? text)
   (with-exception-catcher (lambda (_) 0)
    (lambda () (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-bytevector
                (gerbil-scheme-rust/qualification/utf8-controls#encode mode text)))) 0)))
(c-define (comparison-snapshot slot) (int32) void
 "gerbil_utf8_comparison_snapshot" "extern"
 (vector-set! gerbil-scheme-rust/qualification/utf8-controls#snapshots slot (##process-statistics)))
(c-define (comparison-stat index) (int32) double
 "gerbil_utf8_comparison_stat" "extern"
 (- (f64vector-ref (vector-ref gerbil-scheme-rust/qualification/utf8-controls#snapshots 1) index)
    (f64vector-ref (vector-ref gerbil-scheme-rust/qualification/utf8-controls#snapshots 0) index))))

(begin-foreign
 (c-declare #<<C-END
static ___SCMOBJ comparison_utf8_count(___SCMOBJ text, ___SCMOBJ first, ___SCMOBJ last) {
 ___SCMOBJ ___temp;
 if (!___STRINGP(text) || !___FIXNUMP(first) || !___FIXNUMP(last) ||
     ___INT(first) < 0 || ___INT(last) < ___INT(first)) return ___FIX(-1);
 const ___U64 start = ___INT(first), end = ___INT(last);
 if (end > ___STRINGSIZE(text) || end - start > 256) return ___FIX(-1);
 const ___C *source = ___CAST(const ___C*, ___BODY_AS(text, ___tSTRING));
 ___U32 invalid = 0;
 ___U64 bytes = 0;
 for (___U64 i = start; i < end; ++i) {
  const ___U32 c = source[i];
  invalid |= (c > 0x10ffff) | ((c >> 11) == 0x1b);
  bytes += 1 + (c > 0x7f) + (c > 0x7ff) + (c > 0xffff);
 }
 return invalid ? ___FIX(-1) : ___FIX(bytes);
}
C-END
 )
 (namespace ("gerbil-scheme-rust/qualification/utf8-controls#" historical-chunk))
 (define historical-chunk
  (c-lambda (scheme-object scheme-object unsigned-int64 unsigned-int64 unsigned-int64) int64
   #<<C-END
___SCMOBJ ___temp;
if (!___STRINGP(___arg1) || !___U8VECTORP(___arg2)) ___return(-2);
const ___U64 length = ___STRINGSIZE(___arg1), capacity = ___U8VECTORSIZE(___arg2);
const ___U64 start = ___arg3, end = ___arg4, output_start = ___arg5;
if (start > end || end > length || end - start > 256 || output_start > capacity ||
    4 * (end - start) > capacity - output_start) ___return(-2);
const ___C *source = ___CAST(const ___C*, ___BODY_AS(___arg1, ___tSTRING));
___U8 *output = ___CAST(___U8*, ___BODY_AS(___arg2, ___tU8VECTOR));
___U64 j = output_start;
for (___U64 i = start; i < end; ++i) {
 const ___U32 c = source[i];
 if (c <= 0x7f) output[j++] = (___U8)c;
 else if (c <= 0x7ff) { output[j++] = (___U8)(0xc0 | (c >> 6)); output[j++] = (___U8)(0x80 | (c & 0x3f)); }
 else if (c <= 0xffff) {
  if ((c >> 11) == 0x1b) ___return(-1);
  output[j++] = (___U8)(0xe0 | (c >> 12)); output[j++] = (___U8)(0x80 | ((c >> 6) & 0x3f)); output[j++] = (___U8)(0x80 | (c & 0x3f));
 } else if (c <= 0x10ffff) {
  output[j++] = (___U8)(0xf0 | (c >> 18)); output[j++] = (___U8)(0x80 | ((c >> 12) & 0x3f));
  output[j++] = (___U8)(0x80 | ((c >> 6) & 0x3f)); output[j++] = (___U8)(0x80 | (c & 0x3f));
 } else ___return(-1);
}
___return((___S64)j);
C-END
  )))
