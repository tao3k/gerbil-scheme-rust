# Python / Tokio / Scheme VM Qualification

## Active Direction (2026-10-15)

The user has redirected the active execution runtime to Rust. The installed
Scheme binary's shared-runtime VM entry gap below remains a recorded failure,
not a requirement to patch or rebuild Gambit. The two Python example wheel
profiles now explicitly select `rust-backend`; no `auto` provider or Scheme
fallback is used. Rust engine sessions are independent execution contexts,
not Scheme VMs, and must not be reported as repaired Gambit multi-VM support.

The separate `scheme/runtime-vm-test.ss` actually constructed two simultaneously
live VMs against SDK `1cfb032`. Each had processor 0 and one active processor.
Both official selector readbacks returned `selected=0`, `outcome=13`; the test
exited 70 in 0.706068542 seconds under the unchanged 15-second ceiling, with
no timeout and an absent process group afterwards. Scheme overlap, root
isolation and surviving-VM calls were not reached. The existing SMP test
remains separate and unchanged. This is a reproducible gap in this configured
binary's foreign-entry path, not proof that all Gambit multi-VM configurations
fail or that VM-state construction is broken.

See `docs/rust-runtime-transition.md` for the migration and DSL coverage boundary.

## Current Execution Contract

The bridge currently owns one process-global, one-shot `GerbilRuntime` on one
fixed OS thread. Python calls release the GIL; native roots never leave their
owner. Multiple Python threads or Tokio tasks do not create independent VMs.
The existing Tokio handoff tests qualify transport and root disposal only.

The Python owner now uses `initialize_program_single_processor`: it explicitly
sets Gambit's startup parallelism to one and rejects initialization if actual
processor population differs. `PythonLibrary.active_processors` queries the
selected VM on its owner thread. This is a startup policy, not a sandbox against
trusted Scheme code explicitly changing runtime policy later.

`PythonLibrary.capabilities()` reads the ABI linked into the extension without
initializing Scheme. It reports multiple-VM storage, foreign-entry TLS and
compiled processor capacity separately from the implemented execution model.
`independent_vm_execution` is currently false even if the SDK storage flag is
true. A second open must fail without aliasing or damaging the first owner.

## Requirements For Independent VM Execution

Implementing a worker pool by removing the Rust lifecycle lock is unsafe. The
following native contracts must first be implemented and tested together:

1. One process runtime lifetime and separately owned VM lifetimes; each VM has
   a unique generation, processor state and owner thread. Closing a VM cannot
   call process-global cleanup while any other VM remains live.
2. A VM-selecting entry mechanism: thread-local state or an explicitly propagated
   processor state, consistently supported by the runtime, generated exports
   and Library modules. One processor per VM is the intended policy, not a
   restriction against multiple independent VMs. No intra-VM SMP is required.
   The selected SDK's ABI cannot be changed by defines in only the bridge.
3. Official VM initialization and module initialization in every VM. Allocating
   a VM-state structure or calling `___setup_vmstate` alone is not module
   initialization or proof of independent Gerbil module globals.
4. VM-local roots and generations. A value from another VM must be rejected
   before any C ABI access, including when root numbers happen to coincide.
   Messages transfer owned bytes or checked values, never Scheme heap pointers.
5. Failed initialization rolls back only resources actually acquired. Closing
   one VM drains its admitted work and roots before releasing processor state;
   final runtime shutdown waits for all VM owners. Cancellation of a Tokio
   waiter is not cancellation of a running Scheme computation.

## Required Installed-Wheel Acceptance

Tests live upstream, run outside the source tree, and do not supply SDK paths,
compiler commands or fallback bootstrap code. Use both ASCENT and an independent
stateful Scheme Library. Verify distinct VM identities, isolation of mutable
module state, simultaneous execution, independent collections, retained roots,
cross-VM rejection, failed initialization, cancelled waiters, queue draining,
closing one VM while another remains callable, and final teardown. Preserve
the original whole-run deadlines and every failure/timeout observation.

Installation or single-VM tests cannot admit this contract. Neither can a
capability bit, a process pool, or a refusal test. Until actual independent-VM
execution and both Library paths pass, this work remains unqualified and must
not be published as completed multi-VM support.

## Local Observations (2026-10-15)

The installed ledger wheel candidate 009 reported `multiple_vm_storage=true`,
`thread_local_entry=false`, `max_processors=1`, and
`independent_vm_execution=false`. Two boundary tests passed with actual
initialization and teardown in 2.582 seconds under the original 15-second cap.
They prove truthful reporting and rejection of a second open, not multi-VM
execution. The package closure/publication suite passed 16 tests without skips.

Candidate 008's full installed suite timed out at 15.006 seconds during the
runtime-loaded hygienic DSL test. The prior teardown failure was separately
fixed and checked by candidate 009's boundary run, but this does not close the
full DSL timeout. The unchanged failed DSL route was not rerun as a new pass.
ASCENT installed-wheel acceptance and independent-VM execution remain open.

Raw observations are retained in the consumer workspace under
`.data/upstream-python-015/`: `ledger-build-008.json`, `ledger-build-009.json`,
`ledger-installed-002.json`, `package-tests-002.json`, and
`vm-boundary-001.json`. These local receipts are not a portable source archive,
CI result, or durable Git evidence. No completed multi-VM release is claimed.

Candidate 010 adds that explicit startup policy and processor-population
readback; its actual build completed in 20.379 seconds under the original
120-second cap, with the changed source inputs stable across the build. Six
installed controls, including actual GC/root handling and teardown, passed in
3.019 seconds under the original 15-second cap with SDK environment hidden.
Receipts are `ledger-build-010.json` and `single-processor-policy-001.json` in
the same observation directory. These are still single-VM controls.

The selected SDK's `___SINGLE_THREADED_VMS` branch expands
`___GET_REAL_PSTATE()` to `vmstate0`'s processor and makes
`___SET_REAL_PSTATE(ps)` a no-op. `___BEGIN_PSTATE_BIND` changes that shared
processor's binding, not an independent per-host-thread slot. Locking that
binding can serialize different VM entries but cannot qualify parallel VM
execution. A correct fixed-owner implementation needs an explicit-state ABI or
supported per-thread selector throughout the runtime and generated exports;
removing the Rust singleton or adding worker threads cannot supply it.
