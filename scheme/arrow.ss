;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

;;; Scoped read-only Arrow C Data view. Rust owns the descriptors and buffers.
(import :std/ffi)
(export gerbil-rs-with-arrow-batch gerbil-rs-arrow-row-count
        gerbil-rs-arrow-column-count gerbil-rs-arrow-int64-ref)

(extern arrow-rows arrow-columns arrow-int64-state arrow-int64-value)

(begin-foreign
  (namespace ("gerbil-scheme-rust/scheme/arrow#" arrow-rows arrow-columns
              arrow-int64-state arrow-int64-value))
  (c-declare #<<C
#include <stdint.h>
#include <stddef.h>
#include <string.h>
#ifndef ARROW_C_DATA_INTERFACE
#define ARROW_C_DATA_INTERFACE
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
#endif

static int arrow_data_present(const struct ArrowArray *array, int64_t index) {
  const uint8_t *validity = array->buffers[0];
  return validity == NULL || (validity[index >> 3] & (1u << (index & 7)));
}

static int64_t arrow_data_rows(const struct ArrowArray *array,
                               const struct ArrowSchema *schema) {
  if (array == NULL || schema == NULL || array->release == NULL ||
      schema->release == NULL || schema->format == NULL ||
      (schema->format[0] != '+' || schema->format[1] != 's' ||
       schema->format[2] != '\0') || array->length < 0 ||
      array->offset < 0 || array->n_children < 0 ||
      array->n_children != schema->n_children)
    return -1;
  return array->length;
}

static int arrow_data_int64_state(const struct ArrowArray *array,
                                 const struct ArrowSchema *schema,
                                 int64_t column, int64_t row) {
  if (column < 0 || column >= array->n_children || row < 0 ||
      row >= array->length || row > INT64_MAX - array->offset ||
      array->n_buffers < 1 || array->buffers == NULL ||
      array->children == NULL || schema->children == NULL)
    return -1;
  const struct ArrowArray *child = array->children[column];
  const struct ArrowSchema *type = schema->children[column];
  const int64_t index = row + array->offset;
  if (child == NULL || type == NULL || child->release == NULL ||
      type->release == NULL || type->format == NULL ||
      (type->format[0] != 'l' || type->format[1] != '\0') || type->dictionary != NULL ||
      child->offset < 0 || index >= child->length ||
      index > INT64_MAX - child->offset || child->n_buffers < 2 ||
      child->buffers == NULL || child->buffers[1] == NULL ||
      (uint64_t)(index + child->offset) > SIZE_MAX / sizeof(int64_t))
    return -1;
  return arrow_data_present(array, index) &&
    arrow_data_present(child, index + child->offset);
}

static int64_t arrow_data_int64_value(const struct ArrowArray *array,
                                     int64_t column, int64_t row) {
  const struct ArrowArray *child = array->children[column];
  const size_t index = (size_t)(row + array->offset + child->offset);
  int64_t value;
  memcpy(&value, (const uint8_t *)child->buffers[1] + index * sizeof(value),
         sizeof(value));
  return value;
}
C
)
  (define arrow-rows
    (c-lambda ((pointer void) (pointer void)) int64
      "___return(arrow_data_rows(___arg1, ___arg2));"))
  (define arrow-columns
    (c-lambda ((pointer void)) int64
      "___return(((const struct ArrowArray *)___arg1)->n_children);"))
  (define arrow-int64-state
    (c-lambda ((pointer void) (pointer void) int64 int64) int
      "___return(arrow_data_int64_state(___arg1, ___arg2, ___arg3, ___arg4));"))
  (define arrow-int64-value
    (c-lambda ((pointer void) int64 int64) int64
      "___return(arrow_data_int64_value(___arg1, ___arg2, ___arg3));")))

;; The exported operations carry no foreign pointer out of the scope, even
;; if an application saves a view or tries to reenter through a continuation.
(defstruct arrow-view (rows columns read active?) final: #t)

(def (gerbil-rs-with-arrow-batch array schema consume)
  (def rows (arrow-rows array schema))
  (when (< rows 0) (error "invalid Arrow record batch"))
  (def owner (current-thread))
  (def state 'fresh)
  (def (active?)
    (and (eq? state 'active) (eq? owner (current-thread))))
  (def (read column row)
    (let (status (arrow-int64-state array schema column row))
      (cond ((< status 0) (error "Arrow Int64 column or row is invalid"))
            ((zero? status) #f)
            (else (arrow-int64-value array column row)))))
  (def view (make-arrow-view rows (arrow-columns array) read active?))
  (dynamic-wind
    (lambda ()
      (unless (eq? state 'fresh) (error "expired Arrow batch"))
      (set! state 'active))
    (lambda () (consume view))
    (lambda ()
      (set! state 'closed)
      (set! array #f)
      (set! schema #f))))

(def (require-active-arrow-view! view)
  (unless (and (arrow-view? view) ((arrow-view-active? view)))
    (error "expired or foreign-owner Arrow batch")))

(def (gerbil-rs-arrow-row-count view)
  (require-active-arrow-view! view)
  (arrow-view-rows view))

(def (gerbil-rs-arrow-column-count view)
  (require-active-arrow-view! view)
  (arrow-view-columns view))

;;; Returns an exact integer, or #f for an Arrow null.
(def (gerbil-rs-arrow-int64-ref view column row)
  (require-active-arrow-view! view)
  ((arrow-view-read view) column row))
