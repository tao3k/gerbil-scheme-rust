/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
#ifndef GERBIL_SCHEME_OWNED_H
#define GERBIL_SCHEME_OWNED_H
#include "gerbil_scheme_rust.h"
#ifdef __cplusplus
extern "C" {
#endif

#define GERBIL_OWNED_ABI_VERSION 1u
#define GERBIL_OWNED_ABI_ID "gerbil-owned/v1"
#define GERBIL_OWNED_NO_VM 1u
#define GERBIL_OWNED_SCHEME_VM 2u
/* Exactly one runtime declaration, not combinable feature bits or proof:
 * NO_VM: callbacks/release do not initialize, enter or depend on Scheme.
 * SCHEME_VM: callbacks/release run on the live Scheme runtime's fixed owner.
 * The producer or admitted scope must retain that VM, captures and code through
 * every callback and release. A VM-dependent result needs its own lifetime
 * protection independently of the callable/state; copied results may use NO_VM.
 * Neither flag initializes a runtime, schedules work or authorizes fallback. */
enum GerbilOwnedKind {
  GERBIL_OWNED_NULL = 0, GERBIL_OWNED_BOOL = 1, GERBIL_OWNED_INTEGER = 2,
  GERBIL_OWNED_REAL = 3, GERBIL_OWNED_TEXT = 4, GERBIL_OWNED_BYTES = 5,
  GERBIL_OWNED_LIST = 6, GERBIL_OWNED_MAP = 7
};
typedef struct GerbilOwnedView GerbilOwnedView;
struct GerbilOwnedView {
  uint32_t kind, flags;
  uint64_t bits;
  const uint8_t *bytes;
  size_t length;
  const GerbilOwnedView *children;
  size_t count;
};
/* Null/bool/real use no spans; bool bits are 0/1, real bits are IEEE binary64.
 * Integer uses canonical little-endian magnitude and flags 0/1 for sign;
 * zero has empty magnitude and flags=0. Text is exact UTF-8; NUL is data.
 * Map children are alternating text keys/values with no duplicate keys.
 * Views may share storage but must describe finite acyclic owned data. */
typedef void (*GerbilOwnedRelease)(void *context);
typedef struct GerbilOwnedValue {
  uint32_t abi_version, flags;
  void *context;
  GerbilOwnedView view;
  GerbilOwnedRelease release;
} GerbilOwnedValue;
typedef int32_t (*GerbilOwnedCall)(void *, const GerbilOwnedView *, GerbilOwnedValue *);
typedef int32_t (*GerbilOwnedSnapshot)(void *, GerbilOwnedValue *);
typedef int32_t (*GerbilOwnedReplace)(void *, const GerbilOwnedView *, GerbilOwnedValue *);
typedef struct GerbilOwnedCallable {
  uint32_t abi_version, flags;
  void *context;
  GerbilOwnedCall call;
  GerbilOwnedRelease release;
} GerbilOwnedCallable;
typedef struct GerbilOwnedState {
  uint32_t abi_version, flags;
  void *context;
  GerbilOwnedSnapshot snapshot;
  GerbilOwnedReplace replace;
  GerbilOwnedRelease release;
} GerbilOwnedState;
/* Callbacks execute the producer's compiled algorithm, not a bridge dispatcher.
 * Inputs are borrowed only for the call. Output starts zeroed. Status 0 owns
 * one result and requires release, even for scalar/static views; error preserves
 * its real status and any partially written output is still releasable.
 * Result views remain valid and immutable until that result is released.
 * A state replacement commits atomically only on success and returns the old
 * value; failure leaves state unchanged. Result owners are independent of the
 * callable/state owner. Closing an owner never silently revives it.
 * No exception, longjmp, Rust panic or reentrant owner call may cross this ABI.
 * One owner thread creates, calls and releases each context. Algorithms may
 * have independent owners on parallel host threads. SCHEME_VM does not admit
 * concurrent entry to one VM or transfer of rooted handles between owners.
 * Every non-null release field has this same ABI even on version rejection;
 * consuming adapters release it exactly once. */
#ifdef __cplusplus
}
#endif
#endif
