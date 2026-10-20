/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
#ifndef GERBIL_SCHEME_INSTANCE_H
#define GERBIL_SCHEME_INSTANCE_H
#include <stdint.h>
#include <stddef.h>
/* Instance ABI v3. All calls after setup stay on its fixed OS owner.
 * Hide SDK globals; link the native SDK statically into each copied image.
 * Application entry: int64_t call(int64_t borrowed_object_root).
 * Catch Scheme exceptions; zero rejects, positive transfers a fresh object
 * root. Never retain host pointers or transfer Scheme handles between images.
 * Resolve all symbols before setup. Setup failure is terminal for admission.
 * setup_phase: 0 never entered native setup; 1 unknown partial setup; 2 completed.
 * Only phase-0 images may be dlclosed. Phase-2 failures can run known cleanup;
 * phase-1 rollback is not admitted. Never unload after native setup attempted. */
uint32_t gerbil_scheme_instance_abi(void);
int32_t gerbil_scheme_instance_setup(uint32_t owner);
uint32_t gerbil_scheme_instance_setup_phase(void);
int32_t gerbil_scheme_instance_cleanup(void);
uint64_t gerbil_scheme_instance_io_retired_bytes(void);
uintptr_t gerbil_scheme_instance_state(void);
int32_t gerbil_scheme_instance_usage(double *out);
uint64_t gerbil_scheme_instance_gc_count(void);
/* Optional ABI-v3 diagnostic extension, not required for native execution.
 * Returns 0 after writing six doubles: cumulative SDK GC user/system/real
 * seconds, fixed-owner thread CPU seconds, strong-root count, GC count.
 * Nonzero is failure; consumers must report absent/failed data as unavailable.
 * No Scheme entry, allocation or GC is performed by this snapshot. */
int32_t gerbil_scheme_instance_diagnostics(double *out);
/* Existing native bridge + strong-root exports required by the image ABI. */
int32_t gerbil_scheme_rust_runtime_cleanup(void);
uint32_t gerbil_scheme_rust_current_vm_processors(void);
int64_t gerbil_scheme_rust_root_count_raw(void);
int64_t gerbil_scheme_rust_bytes_to_bytevector_root_raw(const uint8_t*, uint64_t);
int64_t gerbil_scheme_rust_root_bytevector_length_raw(int64_t);
int64_t gerbil_scheme_rust_root_bytevector_copy_raw(int64_t, uint8_t*, uint64_t);
int32_t gerbil_scheme_rust_root_release_raw(int64_t);
#endif
