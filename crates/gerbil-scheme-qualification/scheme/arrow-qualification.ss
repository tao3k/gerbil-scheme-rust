;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
package: gerbil-scheme-rust/qualification
(import :std/ffi :gerbil-scheme-rust/scheme/arrow :gerbil-scheme-rust/scheme/runtime)
(export main)

(def (main . _) (void))

(extern produce-i64)
(begin-foreign
  (namespace ("gerbil-scheme-rust/qualification/arrow-qualification#" produce-i64))
  (c-declare #<<C
#include <stdint.h>
#include <stdlib.h>
#include <stdatomic.h>

struct ArrowArray {
  int64_t length, null_count, offset, n_buffers, n_children;
  const void **buffers;
  struct ArrowArray **children, *dictionary;
  void (*release)(struct ArrowArray *);
  void *private_data;
};
struct ArrowSchema {
  const char *format, *name, *metadata;
  int64_t flags, n_children;
  struct ArrowSchema **children, *dictionary;
  void (*release)(struct ArrowSchema *);
  void *private_data;
};
struct sample_storage {
  const void *buffers[2];
  int64_t value;
};

static _Atomic int releases = 0;
int gerbil_arrow_qualification_releases(void) { return atomic_load(&releases); }

static void release_array(struct ArrowArray *array) {
  free(array->private_data);
  array->release = NULL;
  atomic_fetch_add(&releases, 1);
}

static void release_schema(struct ArrowSchema *schema) {
  schema->release = NULL;
  atomic_fetch_add(&releases, 1);
}

static int produce_i64(struct ArrowArray *array, struct ArrowSchema *schema,
                       int64_t value, int fail) {
  struct sample_storage *storage = malloc(sizeof(*storage));
  if (!storage) return -1;
  storage->value = value;
  storage->buffers[0] = NULL;
  storage->buffers[1] = &storage->value;
  *array = (struct ArrowArray) {1, 0, 0, 2, 0, storage->buffers, NULL, NULL,
                                release_array, storage};
  if (fail) return -1;
  *schema = (struct ArrowSchema) {"l", "scheme", NULL, 0, 0, NULL, NULL,
                                  release_schema, NULL};
  return 0;
}
C
)
  (define produce-i64
    (c-lambda ((pointer void) (pointer void) int64 int) int
      "___return(produce_i64(___arg1, ___arg2, ___arg3, ___arg4));"))
  (c-define (read-batch array schema) ((pointer void) (pointer void)) int64
    "gerbil_arrow_qualification_read" "extern"
    (gerbil-scheme-rust/qualification/arrow-qualification#check-batch array schema))
  (c-define (produce array schema) ((pointer void) (pointer void)) int
    "gerbil_arrow_qualification_produce" "extern"
    (with-catch (lambda (_) -1)
      (lambda ()
        (gerbil-scheme-rust/qualification/arrow-qualification#produce-i64
          array schema (+ 17 25) 0))))
  (c-define (fail array schema) ((pointer void) (pointer void)) int
    "gerbil_arrow_qualification_fail" "extern"
    (gerbil-scheme-rust/qualification/arrow-qualification#produce-i64
      array schema 0 1))
  (c-define (produce-batch array schema) ((pointer void) (pointer void)) int
    "gerbil_arrow_qualification_produce_batch" "extern"
    (gerbil-scheme-rust/qualification/arrow-qualification#write-batch array schema)))


(def (check-batch array schema)
  (with-catch
    (lambda (exn) (display-exception exn (current-error-port)) 0)
    (lambda ()
      (def leaked #f)
      (def (same? actual expected)
        (unless (equal? actual expected) (error "Arrow conversion mismatch" actual expected)))
      (gerbil-rs-with-arrow-batch array schema
        (lambda (batch)
          (def (column index) (gerbil-rs-arrow-column batch index))
          (def (cell column-index row) (gerbil-rs-arrow-ref (column column-index) row))
          (set! leaked (column 0))
          (same? (gerbil-rs-arrow-row-count batch) 2)
          (same? (gerbil-rs-arrow-column-count batch) 44)
          (same? (gerbil-rs-arrow-format leaked) "b")
          (same? (gerbil-rs-arrow-name leaked) "boolean")
          (same? (gerbil-rs-arrow-metadata leaked)
                 (list (cons #u8(107) (string->utf8 "箭\x0;"))))
          (same? (gerbil-rs-arrow-child-count (column 3)) 1)
          (same? (cell 0 0) #f)
          (same? (cell 0 1) gerbil-rs-arrow-null)
          (same? (cell 1 0) 18446744073709551615)
          (same? (cell 2 0) "箭\x0;🦀")
          (same? (cell 3 0) #(42 -2))
          (same? (cell 3 1) gerbil-rs-arrow-null)
          (same? (gerbil-rs-arrow-ref (gerbil-rs-arrow-child (column 3) 0) 1) -2)
          (same? (cell 4 0) -123/100)
          (same? (cell 5 0) "short")
          (same? (cell 5 1) "a longer, external Arrow view")
          (same? (cell 6 0) "dictionary")
          (same? (cell 7 0) "a")
          (same? (cell 7 1) "b")
          (same? (gerbil-rs-arrow-format (column 30)) "tsn:Asia/Shanghai")
          (for-each
            (lambda (expected index) (same? (cell index 0) expected))
            (append
              [-128 -32768 -2147483648 -9223372036854775808
               255 65535 4294967295 -0.5 1.25 -2.5
               12 -2500 (- (arithmetic-shift 1 80)) "大"
               #u8(0 255) #u8(1 255) #u8(2 255) #u8(3 255)
               -10 -86400000 42 43 -44 -45 -46 #(-3 4) #(-3 4 -5)]
              (make-list 4 (vector 11 gerbil-rs-arrow-null))
              [#(1) #(#("key" 9)) #(0 -9) #(0 -9) gerbil-rs-arrow-null])
            (iota 36 8))
          (same? (cell 41 1) #(1 0.5))
          (same? (cell 42 1) #(1 0.5))
          (same? (cell 40 1) gerbil-rs-arrow-null)))
      (unless (with-catch (lambda (_) #t)
                (lambda () (gerbil-rs-arrow-ref leaked 0) #f))
        (error "expired Arrow node still readable"))
      (gerbil-rs-root-object #t))))


(def (write-batch array schema)
  (with-catch
    (lambda (_) -1)
    (lambda ()
      (def values (gerbil-rs-arrow-output "l" 3 [#f (s64vector 7 8 9)] name: "item"))
      (def rows (gerbil-rs-arrow-output "+l" 2 [#f (s32vector 0 2 3)]
                                      name: "rows" children: [values]))
      (def flag (gerbil-rs-arrow-output "b" 2 [#u8(1) #u8(0)]
                                      name: "flag" null-count: 1 flags: 2))
      (def word (gerbil-rs-arrow-output "u" 2 [#f (s32vector 0 3 4) (string->utf8 "箭x")]
                                      name: "word"))
      (def labels (gerbil-rs-arrow-output "c" 2 [#f (s8vector 1 0)]
                                        name: "labels" dictionary: word flags: 1))
      (gerbil-rs-arrow-export-physical! array schema
        (gerbil-rs-arrow-output "+s" 2 [#f] children: [rows flag labels])))))
