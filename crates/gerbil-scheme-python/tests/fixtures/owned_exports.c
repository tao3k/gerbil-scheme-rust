/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 * ABI qualification producer only, not ASCENT or a Scheme interpreter.
 * Deliberately links no Scheme runtime. All captures/state live in this C owner. */
#include "gerbil_scheme_owned.h"
#include <stdatomic.h>
#include <stdlib.h>
#include <string.h>
typedef struct Payload { GerbilOwnedView view; } Payload;
static _Atomic size_t live;
size_t owned_test_live(void) { return atomic_load(&live); }
static void clear_view(GerbilOwnedView *view) {
  for (size_t i = 0; i < view->count; ++i)
    clear_view((GerbilOwnedView *)&view->children[i]);
  free((void *)view->children); free((void *)view->bytes);
  memset(view, 0, sizeof(*view));
}
static int copy_view(const GerbilOwnedView *input, GerbilOwnedView *output) {
  *output = *input; output->bytes = NULL; output->children = NULL;
  output->length = 0; output->count = 0;
  if (input->length) {
    uint8_t *bytes = malloc(input->length);
    if (!bytes) return 0;
    memcpy(bytes, input->bytes, input->length);
    output->bytes = bytes; output->length = input->length;
  }
  if (input->count) {
    GerbilOwnedView *children = calloc(input->count, sizeof(*children));
    if (!children) { clear_view(output); return 0; }
    output->children = children;
    for (size_t i = 0; i < input->count; ++i) {
      output->count = i + 1;
      if (!copy_view(&input->children[i], &children[i])) { clear_view(output); return 0; }
    }
  }
  return 1;
}
static Payload *payload(const GerbilOwnedView *input) {
  Payload *value = calloc(1, sizeof(*value));
  if (!value) return NULL;
  if (!copy_view(input, &value->view)) { free(value); return NULL; }
  atomic_fetch_add(&live, 1); return value;
}
static void release_payload(void *context) {
  Payload *value = context;
  if (value) { clear_view(&value->view); free(value); atomic_fetch_sub(&live, 1); }
}
static int32_t result(const GerbilOwnedView *input, GerbilOwnedValue *output) {
  Payload *value = payload(input);
  if (!value) return GERBIL_STATUS_RUNTIME_UNAVAILABLE;
  *output = (GerbilOwnedValue){GERBIL_OWNED_ABI_VERSION, GERBIL_OWNED_NO_VM,
                               value, value->view, release_payload};
  return 0;
}
static int32_t captured(void *context, const GerbilOwnedView *input, GerbilOwnedValue *output) {
  if (input->kind != GERBIL_OWNED_NULL) return GERBIL_STATUS_INVALID_VALUE;
  return result(&((Payload *)context)->view, output);
}
int32_t owned_test_capture(const GerbilOwnedView *input, GerbilOwnedCallable *output) {
  Payload *value = payload(input);
  if (!value) return GERBIL_STATUS_RUNTIME_UNAVAILABLE;
  *output = (GerbilOwnedCallable){GERBIL_OWNED_ABI_VERSION, GERBIL_OWNED_NO_VM,
                                  value, captured, release_payload};
  return 0;
}
static int32_t snapshot(void *context, GerbilOwnedValue *output) {
  return result(&((Payload *)context)->view, output);
}
static int32_t replace(void *context, const GerbilOwnedView *input, GerbilOwnedValue *output) {
  Payload *current = context;
  int32_t status = result(&current->view, output);
  if (status) return status;
  /* Real error after a partial output: host must release output and keep state. */
  if (input->kind == GERBIL_OWNED_TEXT && input->length == 6 && !memcmp(input->bytes, "reject", 6))
    return GERBIL_STATUS_INVALID_VALUE;
  GerbilOwnedView next = {0};
  if (!copy_view(input, &next)) return GERBIL_STATUS_RUNTIME_UNAVAILABLE;
  clear_view(&current->view); current->view = next; return 0;
}
int32_t owned_test_state(const GerbilOwnedView *input, GerbilOwnedState *output) {
  Payload *value = payload(input);
  if (!value) return GERBIL_STATUS_RUNTIME_UNAVAILABLE;
  *output = (GerbilOwnedState){GERBIL_OWNED_ABI_VERSION, GERBIL_OWNED_NO_VM,
                               value, snapshot, replace, release_payload};
  return 0;
}
static int32_t opaque(void *context, const GerbilOwnedView *input, GerbilOwnedValue *output) {
  int32_t status = captured(context, input, output);
  if (!status) output->view.kind = 99;
  return status;
}
int32_t owned_test_opaque(const GerbilOwnedView *input, GerbilOwnedCallable *output) {
  int32_t status = owned_test_capture(input, output);
  if (!status) output->call = opaque;
  return status;
}
int32_t owned_test_bad_version(const GerbilOwnedView *input, GerbilOwnedCallable *output) {
  int32_t status = owned_test_capture(input, output);
  if (!status) output->abi_version = 99;
  return status;
}
int32_t owned_test_requires_vm(const GerbilOwnedView *input, GerbilOwnedCallable *output) {
  int32_t status = owned_test_capture(input, output);
  if (!status) output->flags = 0;
  return status;
}
int32_t owned_test_missing_call(const GerbilOwnedView *input, GerbilOwnedCallable *output) {
  int32_t status = owned_test_capture(input, output);
  if (!status) output->call = NULL;
  return status;
}
int32_t owned_test_factory_error(const GerbilOwnedView *input, GerbilOwnedCallable *output) {
  int32_t status = owned_test_capture(input, output);
  return status ? status : GERBIL_STATUS_INVALID_VALUE;
}
static int32_t bad_result(void *context, const GerbilOwnedView *input, GerbilOwnedValue *output) {
  int32_t status = captured(context, input, output);
  if (!status) output->abi_version = 99;
  return status;
}
int32_t owned_test_bad_result(const GerbilOwnedView *input, GerbilOwnedCallable *output) {
  int32_t status = owned_test_capture(input, output);
  if (!status) output->call = bad_result;
  return status;
}
size_t owned_test_view_size(void) { return sizeof(GerbilOwnedView); }
size_t owned_test_value_size(void) { return sizeof(GerbilOwnedValue); }
size_t owned_test_callable_size(void) { return sizeof(GerbilOwnedCallable); }
size_t owned_test_state_size(void) { return sizeof(GerbilOwnedState); }
