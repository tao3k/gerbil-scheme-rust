;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

;;; Scoped read-only Arrow C Data view. Rust owns the descriptors and buffers.
(import :std/ffi)
(export gerbil-rs-with-arrow-array gerbil-rs-with-arrow-batch gerbil-rs-arrow-row-count
        gerbil-rs-arrow-column-count gerbil-rs-arrow-int64-ref
        gerbil-rs-arrow-column gerbil-rs-arrow-child gerbil-rs-arrow-format
        gerbil-rs-arrow-name gerbil-rs-arrow-flags gerbil-rs-arrow-length
        gerbil-rs-arrow-metadata gerbil-rs-arrow-child-count gerbil-rs-arrow-dictionary
        gerbil-rs-arrow-ref gerbil-rs-arrow-null
        gerbil-rs-arrow-output gerbil-rs-arrow-export-physical!)

(extern
  arrow-rows arrow-columns arrow-int64-state arrow-int64-value arrow-offset
  arrow-length arrow-format arrow-name arrow-flags arrow-array-child
  arrow-schema-child arrow-array-dict arrow-schema-dict arrow-present arrow-buffer
  arrow-signed arrow-unsigned arrow-float arrow-bit arrow-payload arrow-decimal-byte arrow-copy-bytes arrow-metadata arrow-metadata-size
  arrow-owned-array-init arrow-owned-schema-init arrow-owned-array-child
  arrow-owned-schema-child arrow-owned-buffer! arrow-owned-metadata!)

(begin-foreign
  (namespace ("gerbil-scheme-rust/scheme/arrow#"
    arrow-rows arrow-columns arrow-int64-state arrow-int64-value arrow-offset
    arrow-length arrow-format arrow-name arrow-flags arrow-array-child
    arrow-schema-child arrow-array-dict arrow-schema-dict arrow-present arrow-buffer
    arrow-signed arrow-unsigned arrow-float arrow-bit arrow-payload arrow-decimal-byte arrow-copy-bytes arrow-metadata arrow-metadata-size
  arrow-owned-array-init arrow-owned-schema-init arrow-owned-array-child
  arrow-owned-schema-child arrow-owned-buffer! arrow-owned-metadata!))
  (c-declare #<<C
#include <stdint.h>
#include <stddef.h>
#include <string.h>
#include <stdlib.h>
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
static const void *arrow_buffer(const struct ArrowArray *array, int slot) {
  return array->buffers[slot];
}

#define ARROW_READ(name, type)                                              \
  static type name(const void *data, int64_t index) {                         \
    type value;                                                             \
    memcpy(&value, (const uint8_t *)data + index * sizeof(value),             \
           sizeof(value));                                                  \
    return value;                                                           \
  }

ARROW_READ(arrow_i8, int8_t)
ARROW_READ(arrow_i16, int16_t)
ARROW_READ(arrow_i32, int32_t)
ARROW_READ(arrow_i64, int64_t)
ARROW_READ(arrow_u16, uint16_t)
ARROW_READ(arrow_u32, uint32_t)
ARROW_READ(arrow_u64, uint64_t)
ARROW_READ(arrow_f32, float)
ARROW_READ(arrow_f64, double)

static int64_t arrow_signed(const void *data, int64_t index, int width) {
  switch (width) {
    case 1: return arrow_i8(data, index);
    case 2: return arrow_i16(data, index);
    case 4: return arrow_i32(data, index);
    default: return arrow_i64(data, index);
  }
}

static uint64_t arrow_unsigned(const void *data, int64_t index, int width) {
  switch (width) {
    case 1: return ((const uint8_t *)data)[index];
    case 2: return arrow_u16(data, index);
    case 4: return arrow_u32(data, index);
    default: return arrow_u64(data, index);
  }
}

/* Widen half precision, including signed zeros, infinities and NaNs. */
static double arrow_float(const void *data, int64_t index, int width) {
  if (width == 8) return arrow_f64(data, index);
  if (width == 4) return arrow_f32(data, index);
  const uint16_t half = arrow_u16(data, index);
  const uint32_t sign = (uint32_t)(half & 0x8000) << 16;
  const uint32_t exponent = (half >> 10) & 31;
  const uint32_t fraction = half & 1023;
  if (exponent == 0) {
    const double value = fraction * 0x1p-24;
    return sign ? -value : value;
  }
  uint32_t bits = sign | ((exponent == 31 ? 255 : exponent + 112) << 23) |
                  (fraction << 13);
  float value;
  memcpy(&value, &bits, sizeof(value));
  return value;
}

static int arrow_bit(const uint8_t *bits, int64_t index) {
  return (bits[index >> 3] >> (index & 7)) & 1;
}

static const uint8_t *arrow_view_payload(const struct ArrowArray *array,
                                         int64_t index) {
  const uint8_t *view = (const uint8_t *)arrow_buffer(array, 1) + 16 * index;
  if (arrow_i32(view, 0) <= 12) return view + 4;
  return (const uint8_t *)arrow_buffer(array, 2 + arrow_i32(view, 2)) +
         arrow_i32(view, 3);
}

static void arrow_copy_bytes(uint8_t *destination, const void *source,
                             size_t start, size_t length) {
  const uint8_t *bytes = (const uint8_t *)source + start;
  size_t offset = 0;
  for (; length - offset >= 8; offset += 8) {
    uint64_t word;
    memcpy(&word, bytes + offset, 8);
    memcpy(destination + offset, &word, 8);
  }
  for (; offset < length; offset++) destination[offset] = bytes[offset];
}

static uint8_t arrow_decimal_byte(const void *data, int64_t index,
                                  int width, int byte) {
  /* Arrow decimal buffers store native-endian signed integers. */
  const uint16_t one = 1;
  const int offset = *(const uint8_t *)&one ? byte : width - byte - 1;
  return ((const uint8_t *)data)[index * width + offset];
}

static void arrow_owned_array_release(struct ArrowArray *array) {
  array->release = NULL;
  if (array->buffers != NULL) {
    for (int64_t i = 0; i < array->n_buffers; ++i)
      free((void *)array->buffers[i]);
    free(array->buffers);
  }
  if (array->children != NULL) {
    for (int64_t i = 0; i < array->n_children; ++i) {
      struct ArrowArray *child = array->children[i];
      if (child != NULL && child->release != NULL) child->release(child);
      free(child);
    }
    free(array->children);
  }
  if (array->dictionary != NULL) {
    if (array->dictionary->release != NULL)
      array->dictionary->release(array->dictionary);
    free(array->dictionary);
  }
}

static void arrow_owned_schema_release(struct ArrowSchema *schema) {
  schema->release = NULL;
  free((void *)schema->format);
  free((void *)schema->name);
  free((void *)schema->metadata);
  if (schema->children != NULL) {
    for (int64_t i = 0; i < schema->n_children; ++i) {
      struct ArrowSchema *child = schema->children[i];
      if (child != NULL && child->release != NULL) child->release(child);
      free(child);
    }
    free(schema->children);
  }
  if (schema->dictionary != NULL) {
    if (schema->dictionary->release != NULL)
      schema->dictionary->release(schema->dictionary);
    free(schema->dictionary);
  }
}

static void *arrow_owned_copy(const void *source, size_t bytes) {
  void *copy = malloc(bytes == 0 ? 1 : bytes);
  if (copy != NULL && bytes != 0) arrow_copy_bytes(copy, source, 0, bytes);
  return copy;
}

static int arrow_owned_array_init(struct ArrowArray *array, int64_t length,
                                  int64_t nulls, int64_t offset, size_t buffers,
                                  size_t children) {
  *array = (struct ArrowArray) {
    .length = length, .null_count = nulls, .offset = offset,
    .n_buffers = buffers, .n_children = children, .release = arrow_owned_array_release
  };
  if (buffers != 0) {
    array->buffers = calloc(buffers, sizeof(*array->buffers));
    if (array->buffers == NULL) return 0;
  }
  if (children != 0) {
    array->children = calloc(children, sizeof(*array->children));
    if (array->children == NULL) return 0;
  }
  return 1;
}

static int arrow_owned_schema_init(struct ArrowSchema *schema, const char *format,
                                   const char *name, int64_t flags, size_t children) {
  *schema = (struct ArrowSchema) {
    .flags = flags, .n_children = children, .release = arrow_owned_schema_release
  };
  schema->format = arrow_owned_copy(format, strlen(format) + 1);
  schema->name = arrow_owned_copy(name, strlen(name) + 1);
  if (schema->format == NULL || schema->name == NULL) return 0;
  if (children != 0) {
    schema->children = calloc(children, sizeof(*schema->children));
    if (schema->children == NULL) return 0;
  }
  return 1;
}

static struct ArrowArray *arrow_owned_array_child(struct ArrowArray *array,
                                                  int64_t index) {
  struct ArrowArray **slot = index < 0 ? &array->dictionary : &array->children[index];
  return *slot = calloc(1, sizeof(**slot));
}

static struct ArrowSchema *arrow_owned_schema_child(struct ArrowSchema *schema,
                                                    int64_t index) {
  struct ArrowSchema **slot = index < 0 ? &schema->dictionary : &schema->children[index];
  return *slot = calloc(1, sizeof(**slot));
}

/* Copy native homogeneous-vector storage without retaining a Scheme address. */
static void *arrow_owned_scheme_buffer(___SCMOBJ value) {
  ___SCMOBJ ___temp;
  const void *body;
  size_t count, width;
  if (0) { return NULL; }
#define ARROW_SCHEME_BUFFER(T, W)                                          \
  else if (___##T##VECTORP(value)) {                                      \
    body = ___BODY_AS(value, ___t##T##VECTOR);                            \
    count = ___##T##VECTORSIZE(value);                                    \
    width = W;                                                           \
  }
  ARROW_SCHEME_BUFFER(U8, 1)
  ARROW_SCHEME_BUFFER(S8, 1)
  ARROW_SCHEME_BUFFER(U16, 2)
  ARROW_SCHEME_BUFFER(S16, 2)
  ARROW_SCHEME_BUFFER(U32, 4)
  ARROW_SCHEME_BUFFER(S32, 4)
  ARROW_SCHEME_BUFFER(U64, 8)
  ARROW_SCHEME_BUFFER(S64, 8)
  ARROW_SCHEME_BUFFER(F32, 4)
  ARROW_SCHEME_BUFFER(F64, 8)
  else return NULL;
#undef ARROW_SCHEME_BUFFER
  if (count > SIZE_MAX / width) return NULL;
  return arrow_owned_copy(body, count * width);
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
      "___return(arrow_data_int64_value(___arg1, ___arg2, ___arg3));"))
  (define arrow-offset (c-lambda ((pointer void)) int64
    "___return(((const struct ArrowArray *)___arg1)->offset);"))
  (define arrow-length (c-lambda ((pointer void)) int64
    "___return(((const struct ArrowArray *)___arg1)->length);"))
  (define arrow-format (c-lambda ((pointer void)) UTF-8-string
    "___return((char *)((const struct ArrowSchema *)___arg1)->format);"))
  (define arrow-name (c-lambda ((pointer void)) UTF-8-string
    "___return((char *)((const struct ArrowSchema *)___arg1)->name);"))
  (define arrow-flags (c-lambda ((pointer void)) int64
    "___return(((const struct ArrowSchema *)___arg1)->flags);"))
  (define arrow-array-child (c-lambda ((pointer void) int64) (pointer void)
    "___return(((const struct ArrowArray *)___arg1)->children[___arg2]);"))
  (define arrow-schema-child (c-lambda ((pointer void) int64) (pointer void)
    "___return(((const struct ArrowSchema *)___arg1)->children[___arg2]);"))
  (define arrow-array-dict (c-lambda ((pointer void)) (pointer void)
    "___return(((const struct ArrowArray *)___arg1)->dictionary);"))
  (define arrow-schema-dict (c-lambda ((pointer void)) (pointer void)
    "___return(((const struct ArrowSchema *)___arg1)->dictionary);"))
  (define arrow-present (c-lambda ((pointer void) int64) bool
    "___return(arrow_data_present(___arg1, ___arg2));"))
  (define arrow-buffer (c-lambda ((pointer void) int) (pointer void)
    "___return((void *)arrow_buffer(___arg1, ___arg2));"))
  (define arrow-signed (c-lambda ((pointer void) int64 int) int64
    "___return(arrow_signed(___arg1, ___arg2, ___arg3));"))
  (define arrow-unsigned (c-lambda ((pointer void) int64 int) unsigned-int64
    "___return(arrow_unsigned(___arg1, ___arg2, ___arg3));"))
  (define arrow-float (c-lambda ((pointer void) int64 int) double
    "___return(arrow_float(___arg1, ___arg2, ___arg3));"))
  (define arrow-bit (c-lambda ((pointer void) int64) bool
    "___return(arrow_bit(___arg1, ___arg2));"))
  (define arrow-payload (c-lambda ((pointer void) int64) (pointer void)
    "___return((void *)arrow_view_payload(___arg1, ___arg2));"))
  (define arrow-decimal-byte (c-lambda ((pointer void) int64 int int) int
    "___return(arrow_decimal_byte(___arg1, ___arg2, ___arg3, ___arg4));"))
  (define arrow-copy-bytes (c-lambda (scheme-object (pointer void) int64 int64) void
    "arrow_copy_bytes((uint8_t *)___BODY_AS(___arg1, ___tU8VECTOR), ___arg2, ___arg3, ___arg4);"))
  (define arrow-metadata (c-lambda ((pointer void)) (pointer void)
    "___return((void *)((const struct ArrowSchema *)___arg1)->metadata);"))
  (define arrow-metadata-size (c-lambda ((pointer void) int64) int
    "___return(arrow_i32((const uint8_t *)___arg1 + ___arg2, 0));"))
  (define arrow-owned-array-init
    (c-lambda ((pointer void) int64 int64 int64 size_t size_t) bool
      "___return(arrow_owned_array_init(___arg1, ___arg2, ___arg3, ___arg4, ___arg5, ___arg6));"))
  (define arrow-owned-schema-init
    (c-lambda ((pointer void) nonnull-UTF-8-string nonnull-UTF-8-string int64 size_t) bool
      "___return(arrow_owned_schema_init(___arg1, ___arg2, ___arg3, ___arg4, ___arg5));"))
  (define arrow-owned-array-child
    (c-lambda ((pointer void) int64) (pointer void)
      "___return(arrow_owned_array_child(___arg1, ___arg2));"))
  (define arrow-owned-schema-child
    (c-lambda ((pointer void) int64) (pointer void)
      "___return(arrow_owned_schema_child(___arg1, ___arg2));"))
  (define arrow-owned-buffer!
    (c-lambda ((pointer void) int64 scheme-object) bool
      "___return((((struct ArrowArray *)___arg1)->buffers[___arg2] = arrow_owned_scheme_buffer(___arg3)) != NULL);"))
  (define arrow-owned-metadata!
    (c-lambda ((pointer void) scheme-object) bool
      "___return((((struct ArrowSchema *)___arg1)->metadata = arrow_owned_scheme_buffer(___arg2)) != NULL);")))

;; Foreign pointers stay private. The shared guard rejects expired views
;; before any dereference, including continuation reentry and other owners.
(defstruct arrow-view (rows columns read column active?) final: #t)

(def (call-with-arrow-scope consume)
  (def owner (current-thread))
  (def state 'fresh)
  (def (active?)
    (and (eq? state 'active) (eq? owner (current-thread))))
  (dynamic-wind
    (lambda ()
      (unless (eq? state 'fresh) (error "expired Arrow view"))
      (set! state 'active))
    (lambda () (consume active?))
    (lambda () (set! state 'closed))))

;; Entry points accept trusted, read-only Arrow C Data for one synchronous call.
(def (gerbil-rs-with-arrow-array array schema consume)
  (call-with-arrow-scope
    (lambda (active?)
      (consume (make-node array schema active?)))))

(def (gerbil-rs-with-arrow-batch array schema consume)
  (def rows (arrow-rows array schema))
  (when (< rows 0) (error "invalid Arrow record batch"))
  (call-with-arrow-scope
    (lambda (active?)
      (def (read column row)
        (let (status (arrow-int64-state array schema column row))
          (cond ((< status 0) (error "Arrow Int64 column or row is invalid"))
                ((zero? status) #f)
                (else (arrow-int64-value array column row)))))
      (def (column index)
        (require-index! index (arrow-columns array))
        (make-node (arrow-array-child array index) (arrow-schema-child schema index) active?))
      (consume (make-arrow-view rows (arrow-columns array) read column active?)))))

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

;; The format belongs to the node and is read once, including for nested reads.
(defstruct arrow-node (array schema format active?) final: #t)

(def (make-node array schema active?)
  (make-arrow-node array schema (arrow-format schema) active?))

(def (require-arrow-node! node)
  (unless (and (arrow-node? node) ((arrow-node-active? node)))
    (error "expired or foreign-owner Arrow array")))

(def (require-index! index length)
  (unless (and (exact-integer? index) (<= 0 index) (< index length))
    (error "Arrow index out of bounds" index length)))

(def (gerbil-rs-arrow-column batch column)
  (require-active-arrow-view! batch)
  ((arrow-view-column batch) column))

(def (gerbil-rs-arrow-format node)
  (require-arrow-node! node)
  (string-copy (arrow-node-format node)))

(def (gerbil-rs-arrow-name node)
  (require-arrow-node! node)
  (arrow-name (arrow-node-schema node)))

(def (gerbil-rs-arrow-flags node)
  (require-arrow-node! node)
  (arrow-flags (arrow-node-schema node)))

(def (gerbil-rs-arrow-length node)
  (require-arrow-node! node)
  (arrow-length (arrow-node-array node)))

(def (node-child node index)
  (let (array (arrow-node-array node))
    (require-index! index (arrow-columns array))
    (make-node (arrow-array-child array index)
               (arrow-schema-child (arrow-node-schema node) index)
               (arrow-node-active? node))))

(def (gerbil-rs-arrow-child node index)
  (require-arrow-node! node)
  (node-child node index))

(def (gerbil-rs-arrow-child-count node)
  (require-arrow-node! node)
  (arrow-columns (arrow-node-array node)))

(def (node-dictionary node)
  (let (schema (arrow-schema-dict (arrow-node-schema node)))
    (and schema (make-node (arrow-array-dict (arrow-node-array node)) schema
                           (arrow-node-active? node)))))

(def (gerbil-rs-arrow-dictionary node)
  (require-arrow-node! node)
  (node-dictionary node))

;; A dedicated value keeps Arrow null distinct from a valid false cell.
(defstruct arrow-null-value () final: #t)
(def gerbil-rs-arrow-null (make-arrow-null-value))

(def (parameters format start)
  (map string->number (string-split (substring format start (string-length format)) #\,)))

(def (read-range child start length)
  (let (result (make-vector length))
    (let loop ((i 0))
      (when (< i length)
        (vector-set! result i (node-ref child (+ start i)))
        (loop (+ i 1))))
    result))

(def (read-bytes buffer start length)
  (let (result (make-u8vector length))
    (when (> length 0) (arrow-copy-bytes result buffer start length))
    result))

;; C Data metadata carries length-prefixed bytes. Preserve ordering, duplicate
;; keys and embedded zero bytes even when a producer uses non-UTF-8 values.
(def (gerbil-rs-arrow-metadata node)
  (require-arrow-node! node)
  (let (bytes (arrow-metadata (arrow-node-schema node)))
    (if (not bytes) []
      (let loop ((remaining (arrow-metadata-size bytes 0)) (offset 4) (entries []))
        (if (zero? remaining)
          (reverse entries)
          (let* ((key-length (arrow-metadata-size bytes offset))
                 (key-start (+ offset 4))
                 (value-size-at (+ key-start key-length))
                 (value-length (arrow-metadata-size bytes value-size-at))
                 (value-start (+ value-size-at 4)))
            (loop (- remaining 1) (+ value-start value-length)
                  (cons (cons (read-bytes bytes key-start key-length)
                              (read-bytes bytes value-start value-length))
                        entries))))))))

(def (decimal-value values index width)
  (let loop ((byte (- width 1)) (result 0))
    (if (< byte 0)
      (let (bits (* width 8))
        (if (bit-set? (- bits 1) result)
          (- result (arithmetic-shift 1 bits))
          result))
      (loop (- byte 1)
            (+ (arithmetic-shift result 8)
               (arrow-decimal-byte values index width byte))))))

(def (union-index ids id)
  (let loop ((remaining ids) (index 0))
    (cond
      ((null? remaining) (error "unknown Arrow union type ID" id))
      ((= (car remaining) id) index)
      (else (loop (cdr remaining) (+ index 1))))))

(def (run-index ends index)
  (let loop ((start 0) (end (arrow-length (arrow-node-array ends))))
    (if (= start end)
      start
      (let (middle (quotient (+ start end) 2))
        (if (> (node-ref ends middle) index)
          (loop start middle)
          (loop (+ middle 1) end))))))

(def (node-ref node row)
  (def array (arrow-node-array node))
  (def format (arrow-node-format node))
  (require-index! row (arrow-length array))
  (def index (+ row (arrow-offset array)))
  (def (signed slot width (at index))
    (arrow-signed (arrow-buffer array slot) at width))
  (def (unsigned width)
    (arrow-unsigned (arrow-buffer array 1) index width))
  (def (bytes fixed? view? width)
    (cond
      (fixed? (read-bytes (arrow-buffer array 1) (* index width) width))
      (view? (read-bytes (arrow-payload array index) 0 (signed 1 4 (* 4 index))))
      (else
       (let* ((start (signed 1 width))
              (end (signed 1 width (+ index 1))))
         (read-bytes (arrow-buffer array 2) start (- end start))))))
  (cond
    ((string=? format "n") gerbil-rs-arrow-null)
    ;; Union and run-end arrays lack validity buffers. Their active child
    ;; supplies the logical nullability.
    ((or (string-prefix? "+ud:" format) (string-prefix? "+us:" format))
     (let* ((id (signed 0 1))
            (child (node-child node (union-index (parameters format 4) id))))
       (vector id (node-ref child (if (string-prefix? "+ud:" format) (signed 1 4) index)))))
    ((string=? format "+r")
     (node-ref (node-child node 1) (run-index (node-child node 0) index)))
    ((not (arrow-present array index)) gerbil-rs-arrow-null)
    (else
     (let* ((dictionary (node-dictionary node))
            (value
             (cond
               ((string=? format "b") (arrow-bit (arrow-buffer array 1) index))
               ((string=? format "c") (signed 1 1))
               ((string=? format "C") (unsigned 1))
               ((string=? format "s") (signed 1 2))
               ((string=? format "S") (unsigned 2))
               ((member format '("i" "tdD" "tts" "ttm" "tiM")) (signed 1 4))
               ((string=? format "I") (unsigned 4))
               ((or (member format '("l" "tdm" "ttu" "ttn"))
                    (string-prefix? "ts" format) (string-prefix? "tD" format)) (signed 1 8))
               ((string=? format "L") (unsigned 8))
               ((member format '("e" "f" "g"))
                (arrow-float (arrow-buffer array 1) index
                             (cond ((string=? format "e") 2)
                                   ((string=? format "f") 4)
                                   (else 8))))
               ((member format '("z" "u")) (bytes #f #f 4))
               ((member format '("Z" "U")) (bytes #f #f 8))
               ((member format '("vz" "vu")) (bytes #f #t 0))
               ((string-prefix? "w:" format) (bytes #t #f (car (parameters format 2))))
               ((string-prefix? "d:" format)
                (let* ((parts (parameters format 2))
                       (width (if (null? (cddr parts)) 16 (quotient (caddr parts) 8))))
                  (/ (decimal-value (arrow-buffer array 1) index width)
                     (expt 10 (cadr parts)))))
               ((string=? format "tiD")
                (vector (signed 1 4 (* 2 index)) (signed 1 4 (+ (* 2 index) 1))))
               ((string=? format "tin")
                (vector (signed 1 4 (* 4 index)) (signed 1 4 (+ (* 4 index) 1))
                        (signed 1 8 (+ (* 2 index) 1))))
               ((member format '("+l" "+L" "+m" "+vl" "+vL"))
                (let* ((width (if (member format '("+L" "+vL")) 8 4))
                       (start (signed 1 width))
                       (length (if (string-prefix? "+v" format)
                                 (signed 2 width)
                                 (- (signed 1 width (+ index 1)) start))))
                  (read-range (node-child node 0) start length)))
               ((string-prefix? "+w:" format)
                (let (length (car (parameters format 3)))
                  (read-range (node-child node 0) (* index length) length)))
               ((string=? format "+s")
                (let* ((count (arrow-columns array)) (result (make-vector count)))
                  (let loop ((i 0))
                    (when (< i count)
                      (vector-set! result i (node-ref (node-child node i) index))
                      (loop (+ i 1))))
                  result))
               (else (error "unsupported Arrow C Data format" format)))))
       (cond
         (dictionary
          (node-ref dictionary value))
         ((member format '("u" "U" "vu")) (utf8->string value))
         (else value))))))

(def (gerbil-rs-arrow-ref node row)
  (require-arrow-node! node)
  (node-ref node row))

;; A trusted, schema-directed physical layout. Use homogeneous vectors for
;; Arrow buffers and #f for an absent buffer. Metadata, when present, is an
;; already encoded C Data bytevector. Children and dictionary use the same
;; representation. The caller must supply correctly sized buffers and valid
;; offsets and metadata for the declared format before native import.
(defstruct arrow-output (format name length buffers children dictionary offset nulls flags metadata)
  final: #t)

(def (gerbil-rs-arrow-output format length buffers
                           name: (name "") children: (children [])
                           dictionary: (dictionary #f) offset: (offset 0)
                           null-count: (nulls 0) flags: (flags 0) metadata: (metadata #f))
  (unless (and (string? format) (not (string-empty? format)) (string? name)
               (exact-integer? length) (<= 0 length)
               (exact-integer? offset) (<= 0 offset)
               (exact-integer? nulls) (<= -1 nulls length)
               (exact-integer? flags) (<= 0 flags)
               (list? buffers) (list? children) (andmap arrow-output? children)
               (or (not dictionary) (arrow-output? dictionary))
               (or (not metadata) (u8vector? metadata)))
    (error "invalid Arrow output layout"))
  (make-arrow-output format name length buffers children dictionary offset nulls flags metadata))

;; Both output slots are supplied empty by the native Rust binder. Every
;; successful allocation is attached immediately, so a failed export can be
;; released by Rust even when a Scheme exception interrupts construction.
(def (gerbil-rs-arrow-export-physical! array schema output)
  (with-catch
    (lambda (_) -1)
    (lambda ()
      (write-output! array schema output)
      0)))

(def (write-output! array schema output)
  (def buffers (arrow-output-buffers output))
  (def children (arrow-output-children output))
  (def dictionary (arrow-output-dictionary output))
  (def metadata (arrow-output-metadata output))
  (unless (and array schema
               (arrow-owned-array-init array (arrow-output-length output)
                                       (arrow-output-nulls output) (arrow-output-offset output)
                                       (length buffers) (length children))
               (arrow-owned-schema-init schema (arrow-output-format output)
                                        (arrow-output-name output) (arrow-output-flags output)
                                        (length children)))
    (error "cannot allocate Arrow output"))
  (when metadata
    (unless (arrow-owned-metadata! schema metadata) (error "cannot copy Arrow metadata")))
  (for-each
    (lambda (buffer index)
      (when (and buffer (not (arrow-owned-buffer! array index buffer)))
        (error "cannot copy Arrow buffer" index)))
    buffers (iota (length buffers)))
  (def (write-child child index)
    (write-output! (arrow-owned-array-child array index)
                   (arrow-owned-schema-child schema index) child))
  (for-each write-child children (iota (length children)))
  (when dictionary (write-child dictionary -1)))
