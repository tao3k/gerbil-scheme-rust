;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Included in the runtime module: the strong root registry has one owner.

(def (gerbil-rs-collection-view value kind)
  (case kind
    ((0) (and (vector? value) value))
    ((1) (and (list? value) (list->vector value)))
    ((2) (and (pair? value) (vector (car value) (cdr value))))
    (else #f)))

(def (gerbil-rs-indexed-value root index)
  (let (value (gerbil-rs-rooted-value-ref root))
    (cond
      ((= index -1) value)
      ((and (>= index 0) (vector? value) (< index (vector-length value)))
       (vector-ref value index))
      (else gerbil-rs-root-missing))))

(begin-foreign
 (namespace ("gerbil-scheme-rust/scheme/runtime#"
   gerbil-rs-root-id-at gerbil-rs-write-root-id gerbil-rs-scalar-copy-c gerbil-rs-object-scalar-raw
   gerbil-rs-root-count-raw gerbil-rs-force-gc-raw
   gerbil-rs-read-limb gerbil-rs-write-limb
   gerbil-rs-read-small-integer gerbil-rs-copy-small-integer gerbil-rs-copy-integer-value
   gerbil-rs-object-root-raw gerbil-rs-object-word-raw
   gerbil-rs-collection-create-raw gerbil-rs-collection-length-raw
   gerbil-rs-collection-element-raw
   gerbil-rs-collection-snapshot-raw gerbil-rs-object-scalar-indexed
   gerbil-rs-bigint-size-indexed gerbil-rs-bigint-copy-indexed
   gerbil-rs-roots-release-raw gerbil-rs-bigint-create-raw
   gerbil-rs-bigint-size-raw gerbil-rs-bigint-copy-raw
   gerbil-rs-bigint-create-byte-control gerbil-rs-bigint-copy-byte-control))
 (namespace ("gerbil-scheme-rust/scheme/runtime#"
   gerbil-rs-numeric-span gerbil-rs-numeric-create gerbil-rs-numeric-size gerbil-rs-numeric-copy))
 ;; Official storage macros remain inside a nonallocating foreign leaf. No
 ;; Scheme body pointer is returned or retained; memcpy accepts unaligned spans.
 (c-declare "#include <string.h>\n#include <stdint.h>")
 (define gerbil-rs-numeric-span
   (c-lambda (scheme-object int32 (pointer unsigned-int8) unsigned-int64 unsigned-int64 int32) int64
     #<<END-C
___SCMOBJ ___temp;
void *body = 0; ___U64 count = 0, width = 0;
#define GRS_NUMERIC(K,T,W) case K: if (!___##T##VECTORP(___arg1)) ___return(-1); count=___##T##VECTORSIZE(___arg1); width=W; body=___BODY_AS(___arg1,___t##T##VECTOR); break
switch (___arg2) {
  GRS_NUMERIC(0,U16,2); GRS_NUMERIC(1,U32,4); GRS_NUMERIC(2,U64,8);
  GRS_NUMERIC(3,F32,4); GRS_NUMERIC(4,F64,8);
  default: ___return(-1);
}
#undef GRS_NUMERIC
if (___arg6 == 2) ___return(count);
if (___arg4 > count || ___arg5 > count-___arg4 ||
    ___arg5 > SIZE_MAX/width || (___arg5 && !___arg3)) ___return(-1);
if (___arg5) {
  unsigned char *start = (unsigned char*)body + ___arg4*width;
  unsigned char *destination, *source;
  if (___arg6 == 0) { destination=___arg3; source=start; }
  else if (___arg6 == 1) { destination=start; source=___arg3; }
  else ___return(-1);
  /* Constant-size copies allow unaligned spans without aliasing casts. */
  ___U64 bytes=___arg5*width, offset=0;
  for (; bytes-offset >= 8; offset+=8) {
    ___U64 word; memcpy(&word,source+offset,8); memcpy(destination+offset,&word,8);
  }
  for (; offset < bytes; offset++) destination[offset]=source[offset];
}
___return(0);
END-C
))
 (c-define (gerbil-rs-numeric-create kind bytes size)
   (int32 (pointer unsigned-int8) unsigned-int64) int64
   "gerbil_scheme_rust_numeric_create" "extern"
   (with-exception-catcher
     (lambda (_exception) 0)
     (lambda ()
       (let ((value (case kind
                      ((0) (make-u16vector size)) ((1) (make-u32vector size))
                      ((2) (make-u64vector size)) ((3) (make-f32vector size))
                      ((4) (make-f64vector size)) (else #f))))
         (if (and value (= 0 (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-span
                               value kind bytes 0 size 1)))
           (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store! value) 0)))))
 (c-define (gerbil-rs-numeric-size root index kind) (int64 int64 int32) int64
   "gerbil_scheme_rust_numeric_size" "extern"
   (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-span
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value root index) kind #f 0 0 2))
 (c-define (gerbil-rs-numeric-copy root index kind output offset size)
   (int64 int64 int32 (pointer unsigned-int8) unsigned-int64 unsigned-int64) int64
   "gerbil_scheme_rust_numeric_copy" "extern"
   (gerbil-scheme-rust/scheme/runtime#gerbil-rs-numeric-span
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value root index) kind output offset size 0))
 (define gerbil-rs-root-id-at
   (c-lambda ((pointer int64) unsigned-int64) int64 "___return(___arg1[___arg2]);"))
 (define gerbil-rs-write-root-id
   (c-lambda ((pointer int64) unsigned-int64 int64) void "___arg1[___arg2] = ___arg3;"))
 (c-declare "#include <string.h>")
 ;; Keep GC-managed object inspection inside the Gambit foreign-call boundary.
 (define gerbil-rs-scalar-copy-c
   (c-lambda (scheme-object (pointer unsigned-int8)) int32
     #<<END-C
___FLONUMP_DECL
___U32 tag=0, reserved=0;
___U64 bits=0;
if (___arg1 == ___FAL || ___arg1 == ___TRU) { tag=1; bits=___arg1 == ___TRU; }
else if (___FIXNUMP(___arg1)) { tag=2; bits=(___U64)(___S64)___INT(___arg1); }
else if (___CHARP(___arg1)) {
  bits=___ORD(___arg1);
  if (bits <= 0x10ffff && !(bits >= 0xd800 && bits <= 0xdfff)) tag=3;
} else if (___FLONUMP(___arg1)) { tag=4; double n=___F64UNBOX(___arg1); memcpy(&bits,&n,8); }
if (tag == 0) ___return(3);
memcpy(___arg2,&tag,4); memcpy(___arg2+4,&reserved,4); memcpy(___arg2+8,&bits,8);
___return(0);
END-C
))
 (c-define (gerbil-rs-object-scalar-raw root output) (int64 (pointer unsigned-int8)) int32
   "gerbil_scheme_rust_object_scalar_raw" "extern"
   (gerbil-scheme-rust/scheme/runtime#gerbil-rs-scalar-copy-c
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root) output))
 (c-define (gerbil-rs-object-scalar-indexed root index output)
   (int64 int64 (pointer unsigned-int8)) int32
   "gerbil_scheme_rust_object_scalar_indexed" "extern"
   (gerbil-scheme-rust/scheme/runtime#gerbil-rs-scalar-copy-c
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value root index) output))
 (c-define (gerbil-rs-collection-snapshot-raw root kind size)
   (int64 int32 unsigned-int64) int64
   "gerbil_scheme_rust_collection_snapshot_raw" "extern"
   (with-exception-catcher
     (lambda (_exception) 0)
     (lambda ()
       (let ((values (gerbil-scheme-rust/scheme/runtime#gerbil-rs-collection-view
                      (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root) kind)))
         (if (not (and values (= size (vector-length values)))) 0
           (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
             ;; Lists/pairs already produced fresh vectors; vectors need a copy.
             (if (= kind 0) (##vector-copy values) values)))))))
 ;; Each leaf transfers a complete 64-bit limb, never one Rust call per byte.
 (define gerbil-rs-read-limb
   (c-lambda ((pointer unsigned-int8) unsigned-int64 unsigned-int64) unsigned-int64
     "___U64 n=0; for (___U64 i=0;i<___arg3;i++) n |= (___U64)___arg1[___arg2+i] << (i*8); ___return(n);"))
 (define gerbil-rs-write-limb
   (c-lambda ((pointer unsigned-int8) unsigned-int64 unsigned-int64 unsigned-int64) void
     "for (___U64 i=0;i<___arg3;i++) ___arg1[___arg2+i] = (___U8)(___arg4 >> (i*8));"))
 ;; Canonical zero/one-byte magnitudes fit official immediate fixnums. Keep
 ;; representation inspection inside the foreign boundary, not Rust word code.
 (define gerbil-rs-read-small-integer
   (c-lambda ((pointer unsigned-int8) unsigned-int64 int32) scheme-object
     "___S64 n = ___arg2 == 0 ? 0 : ___arg1[0]; ___return(___FIX(___arg3 == 0 ? n : -n));"))
 (define gerbil-rs-copy-small-integer
   (c-lambda (scheme-object (pointer unsigned-int8) unsigned-int64) int32
     #<<END-C
if (!___FIXNUMP(___arg1)) ___return(-2);
___S64 n = ___INT(___arg1);
___U64 magnitude = n < 0 ? (___U64)(-n) : (___U64)n;
if ((___arg3 == 0 && magnitude != 0) ||
    (___arg3 == 1 && (magnitude == 0 || magnitude > 255))) ___return(-1);
if (___arg3 == 1) ___arg2[0] = (___U8)magnitude;
___return(n < 0 ? 1 : 0);
END-C
))
 (define (gerbil-rs-copy-integer-value value output size)
   (let ((small (if (<= size 1)
                  (gerbil-scheme-rust/scheme/runtime#gerbil-rs-copy-small-integer value output size) -2)))
     (if (not (= small -2)) small
       (if (not (and (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer? value)
                 (= size (quotient (+ (integer-length (abs value)) 7) 8)))) -1
         (begin
           (let lp ((offset 0) (rest (abs value)))
             (when (< offset size)
               (let ((count (min 8 (- size offset))))
                 (gerbil-scheme-rust/scheme/runtime#gerbil-rs-write-limb output offset count (bitwise-and rest #xffffffffffffffff))
                 (lp (+ offset count) (arithmetic-shift rest (* -8 count))))))
           (if (< value 0) 1 0))))))
 (c-define (gerbil-rs-object-root-raw value) (scheme-object) int64
   "gerbil_scheme_rust_object_root_raw" "extern"
   (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store! value))
 (c-define (gerbil-rs-object-word-raw root) (int64) scheme-object
   "gerbil_scheme_rust_object_word_raw" "extern"
   (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root))
 (c-define (gerbil-rs-root-count-raw) () int64 "gerbil_scheme_rust_root_count_raw" "extern"
   (table-length gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values))
 (c-define (gerbil-rs-force-gc-raw) () int32 "gerbil_scheme_rust_force_gc_raw" "extern"
   (##gc) 0)
 (c-define (gerbil-rs-collection-create-raw roots size kind)
   ((pointer int64) unsigned-int64 int32) int64
   "gerbil_scheme_rust_collection_create_raw" "extern"
   (with-exception-catcher
    (lambda (_exception) 0)
    (lambda ()
   (if (or (not (<= 0 kind 2)) (and (= kind 2) (not (= size 2)))) 0
     (let ((values (make-vector size)))
       (let lp ((index 0))
         (if (= index size)
           (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
             (case kind ((0) values) ((1) (vector->list values))
               ((2) (cons (vector-ref values 0) (vector-ref values 1)))))
           (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
                         (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at roots index))))
             (if (eq? value gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing) 0
               (begin (vector-set! values index value) (lp (+ index 1))))))))))))
 (c-define (gerbil-rs-collection-length-raw root kind) (int64 int32) int64
   "gerbil_scheme_rust_collection_length_raw" "extern"
   (let ((values (gerbil-scheme-rust/scheme/runtime#gerbil-rs-collection-view
                 (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root) kind)))
     (if values (vector-length values) -1)))
 (c-define (gerbil-rs-collection-element-raw root kind index)
   (int64 int32 unsigned-int64) int64
   "gerbil_scheme_rust_collection_element_raw" "extern"
   (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root)))
     ;; The scalar control does not allocate a temporary vector per element.
     (case kind
       ((0) (if (and (vector? value) (< index (vector-length value)))
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store! (vector-ref value index)) 0))
       ((1) (if (and (list? value) (< index (length value)))
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store! (list-ref value index)) 0))
       ((2) (if (and (pair? value) (< index 2))
              (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
                (if (= index 0) (car value) (cdr value))) 0))
       (else 0))))
 (c-define (gerbil-rs-roots-release-raw roots size)
   ((pointer int64) unsigned-int64) int32
   "gerbil_scheme_rust_roots_release_raw" "extern"
   (if (= size 2)
     ;; Immutable short spans need two leaf reads, not four reads over two walks.
     (let ((first (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at roots 0))
           (second (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at roots 1)))
       (if (and (> first 0) (> second first)
             (not (eq? (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref first)
                   gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing))
             (not (eq? (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref second)
                   gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing)))
         (begin
           (table-set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values first)
           (table-set! gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-values second) 0) 3))
   (let validate ((index 0) (previous 0))
     (if (= index size)
       (begin
         (let release ((index 0))
           (when (< index size)
             (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-release!
               (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at roots index))
             (release (+ index 1)))) 0)
       (let ((id (gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-id-at roots index)))
         (if (and (> id previous)
               (not (eq? (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref id)
                     gerbil-scheme-rust/scheme/runtime#gerbil-rs-root-missing)))
           (validate (+ index 1) id) 3))))))
 (c-define (gerbil-rs-bigint-create-raw source size negative)
   ((pointer unsigned-int8) unsigned-int64 int32) int64
   "gerbil_scheme_rust_bigint_create_raw" "extern"
   (if (<= size 1)
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store!
       (gerbil-scheme-rust/scheme/runtime#gerbil-rs-read-small-integer source size negative))
   (let lp ((end size) (number 0))
     (if (zero? end)
       (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store! (if (= negative 0) number (- number)))
       (let* ((start (max 0 (- end 8))) (count (- end start)))
         (lp start (bitwise-ior (arithmetic-shift number (* count 8))
                     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-read-limb source start count))))))))
 (c-define (gerbil-rs-bigint-size-raw root) (int64) int64
   "gerbil_scheme_rust_bigint_size_raw" "extern"
   (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root)))
     (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer? value)
       (quotient (+ (integer-length (abs value)) 7) 8) -1)))
 (c-define (gerbil-rs-bigint-size-indexed root index) (int64 int64) int64
   "gerbil_scheme_rust_bigint_size_indexed" "extern"
   (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value root index)))
     (if (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer? value)
       (quotient (+ (integer-length (abs value)) 7) 8) -1)))
 ;; Executable reference algorithm, not a per-byte Rust FFI implementation.
 (c-define (gerbil-rs-bigint-create-byte-control source size negative)
   ((pointer unsigned-int8) unsigned-int64 int32) int64
   "gerbil_scheme_rust_bigint_create_byte_control" "extern"
   (let lp ((end size) (number 0))
     (if (zero? end)
       (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-store! (if (= negative 0) number (- number)))
       (lp (- end 1) (bitwise-ior (arithmetic-shift number 8)
         (gerbil-scheme-rust/scheme/runtime#gerbil-rs-read-limb source (- end 1) 1))))))
 (c-define (gerbil-rs-bigint-copy-byte-control root output size)
   (int64 (pointer unsigned-int8) unsigned-int64) int32
   "gerbil_scheme_rust_bigint_copy_byte_control" "extern"
   (let ((value (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root)))
     (if (not (and (gerbil-scheme-rust/scheme/runtime#gerbil-rs-exact-integer? value)
               (= size (quotient (+ (integer-length (abs value)) 7) 8)))) -1
       (begin
         (let lp ((offset 0) (rest (abs value)))
           (when (< offset size)
             (gerbil-scheme-rust/scheme/runtime#gerbil-rs-write-limb output offset 1 (bitwise-and rest #xff))
             (lp (+ offset 1) (arithmetic-shift rest -8))))
         (if (< value 0) 1 0)))))
 (c-define (gerbil-rs-bigint-copy-raw root output size)
   (int64 (pointer unsigned-int8) unsigned-int64) int32
   "gerbil_scheme_rust_bigint_copy_raw" "extern"
   (gerbil-scheme-rust/scheme/runtime#gerbil-rs-copy-integer-value
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref root) output size))
 (c-define (gerbil-rs-bigint-copy-indexed root index output size)
   (int64 int64 (pointer unsigned-int8) unsigned-int64) int32
   "gerbil_scheme_rust_bigint_copy_indexed" "extern"
   (gerbil-scheme-rust/scheme/runtime#gerbil-rs-copy-integer-value
     (gerbil-scheme-rust/scheme/runtime#gerbil-rs-indexed-value root index) output size)))
