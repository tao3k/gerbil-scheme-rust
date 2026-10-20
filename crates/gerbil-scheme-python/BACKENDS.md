# Explicit Execution Backends

For the required distinction between algorithm origin and execution host, see
[Hybrid compilation and runtime ownership](../../docs/hybrid-execution.md).
`rust-backend` is not a requirement to hand-port Scheme algorithms or add a
runtime interpreter. Build-time generated functions and qualified independent
algorithm exports are compatible with a Rust host. VM-rooted exports remain
dependent on their explicitly selected Scheme runtime; FFI does not erase that
dependency. This clarification does not introduce a new backend or fallback.

The primary consumer migration uses existing compiled Scheme/C algorithm exports
through the typed C ABI and ownership bridge. Scheme-to-Rust IR compilation is
not a substitute for that reuse and must not be reported as C ABI qualification.

`select_backend` selects a library-owned provider named `scheme` or `rust`.
A library may declare its default; the ledger defaults to `rust`. `auto`, unknown names and missing providers are errors.
Selection does not parse DSL, compile code or invoke another provider after a
failure. Python exposes the choice but does not implement either language.

Provider availability is fixed by Cargo features, not runtime probing. The
support crate defaults to resource-only code. `scheme-backend` includes the
Scheme bridge; `rust-backend` enables Python transport for a library's Rust
provider. Maintainers build wheels with either or both features. Installed
applications select only providers compiled into their wheel and never run a
compiler to enable a missing provider.

## Independent Library

The installed `gerbil_ledger` example has two providers:

```python
from gerbil_ledger import open_library

ledger = open_library(backend="rust")
ledger.call(("append", 42)).close()
answer = ledger.call(("total",))
assert answer.to_python() == 42
answer.close()
ledger.close()
```

The default Rust provider now calls the compiler-owned `scheme/ledger.ss` native
export through `open_instance_library`. It starts four distinct Scheme VM images
on four fixed OS owners (`owners=4`). Tokio dispatches requests across those
owners; Scheme retains its actors, green threads, closures, GC and continuations.
The build uses Gerbil -> Gambit C -> native code, without Rust IR lowering.
`call` retains state on owner 0. `session(owner)` pins calls and values to an
owner; sessions on one owner share its Scheme module state. `session.call_many`
submits stateful work in input order. `call_stateless` and `call_many` reserve
round-robin owners for caller-declared independent requests; native references
require an owner-affine call. `call_on` remains an explicit pinned call.

`project_many(values, release=True)` projects on the actual owners and releases
in owner batches after successful projection and Python materialization.
`release_many(values)` performs idempotent owner-batched retirement. Projection
failure preserves cyclic graphs, procedures and continuations as native handles;
release-stage failure may retire a subset and supports retry. See the
[routing and projection contract](../../docs/30-39-status/30.16-routing-and-owner-batch-projection.org).

`call_many` and `call_many_profiled` accept `admission_window=1..128` (default
128). Smaller turns retain the original five-second deadline and timing origin
for each group of 128 inputs. This bounds one caller's in-flight compute work;
it does not establish global queue fairness or cancel accepted Scheme work.
The measured 16-request candidate improved some short-request tails but regressed
throughput and some main-task tails, so it remains explicit rather than default.
Optional bounded `set_owner_profiling` / `owner_profile` diagnostics separate
native owner stages; diagnostic JSON does not replace native value transport.
See [mixed-load attribution and performance limits](../../docs/30-39-status/30.17-bounded-owner-admission-and-attribution.org).

`prepare_request(value)` explicitly snapshots portable typed input into immutable
host-owned data. Repeated calls share that description and build fresh native
Scheme input on each owner. Python mutations after preparation do not alter it;
Scheme references and Python cycles are rejected and composed nesting stays
bounded. Preparation cost is reported separately from warm reuse timings.

`dispatch="tokio-pipeline"` is an explicit experimental completion-driven option.
It preserves original 128-request/five-second groups, input result ordering and
round-robin assignment. Global in-flight work is bounded by `admission_window`;
each owner is bounded by its ceiling divided across the VM count. At a window
of four with four VMs this is one computation per owner. Native Scheme work
stays on fixed owners. Measured short-tail regressions remain, so this does not
replace the default Tokio barrier path or default window 128.

For reusable owner-affine data, the existing `session.value(data)` and native
handle arguments retain the actual Scheme graph on its VM. The read-only Actor
benchmark measures this path without reconstructing inputs or translating Scheme
to Rust; full GC and native lifetime checks remain active. See
[client admission, data placement and retained negative results](../../docs/30-39-status/30.18-client-admission-and-native-data-paths.org).

The installed API uses native lists, vectors, tables, binary integers and opaque
rooted procedures/graphs. Cyclic projection can fail without losing the graph.
`call_many_profiled` reports per-window admission-to-native-ready nanoseconds,
excluding Python result projection. Its `direct` control uses exactly the same
VMs, Scheme program and workload. The measured Tokio candidate regressed; see
[installed owner qualification](../../docs/30-39-status/30.10-python-native-owner-integration.org).

The legacy `scheme` provider remains a separate process-global linked owner.
The experimental private-image provider is Darwin-only. The subsequent grouped native-module package passes the tested four-owner
expander, hygienic macro, closure/GC and shutdown gates inside the original
five-second operation budget. Prior failures remain in 30.10; see [the repair
and direct/Tokio/Rayon matrix](../../docs/30-39-status/30.11-expander-startup-and-dispatch-benchmarks.org). This does not establish universal Scheme conformance or optimized phase-code performance. The separate `python-ascent` kernel
qualification has not been migrated to this runtime.
The actual O2 wheel also passes these exercised capability gates; see
[the progress-supervised O0/O2 qualification](../../docs/30-39-status/30.13-progress-supervised-o2-qualification.org).
All matched macro timing cohorts were noisy, and quiet-host macro and dispatcher
performance admission remains pending.

Both providers verify the package's complete per-path SHA inventory before
opening. A result's successful scalar projection does not prove complete DSL
coverage, source refinement, or performance equivalence.

## ASCENT Qualification

`examples/python-ascent` exercises ASCENT's existing pure Rust
`Program::read_sources` / `Engine` / `Session` API at revision
`e6884b83c7b141ec66e2095252e4fb9e17da9869` (PR #1). It does not use the
`ascent-scheme` crate or the Scheme frontend followed by a Rayon kernel. The
supported runtime input is ASCENT's finite typed relational `(ascent ...)`
subset. General Scheme forms and `define-evidence` are explicitly unsupported.
This test adapter is not a replacement for ASCENT's complete public Python API.
Its Scheme provider is unavailable and rejects selection rather than falling
back. Opaque result projection errors retain the complete Rust snapshot.

The remote Git dependency preparation timed out at its 15-second cap. That
failure remains. A separate local build used byte-fixed sources exported from
the same Git commit, not the dirty ASCENT checkout: only `rust` and
`program/rust-execution.ss` were needed. This local transport is recorded in the
receipt and is not a claim that remote Cargo preparation or portable CI passed.

## Historical Evidence Boundary (Before Source Snapshot 020)

Local receipts are under the consumer workspace's `.data/upstream-python-015`.
Candidate 012 built in 62.902 seconds under 120 seconds. Its installed Rust
ledger suite passed 5 tests in 3.445 seconds under 15 seconds, observing no
Scheme initialization before or after each case. Six selected Scheme controls
passed on the same wheel; the known full DSL timeout was not rerun or admitted.

The fixed-source ASCENT Rust wheel built in 33.118 seconds under 120 seconds.
Three installed tests passed in 0.685 seconds under 15 seconds with SDK/source
reads and known compiler execution denied by the qualification sandbox. These
cover real recursive rules, source withdrawal, concurrent independent Rust
sessions, snapshot retention and explicit unsupported inputs. Rust sessions are
not Scheme VMs.

These are local subset observations, not whole-project closure or exact-head
CI. Parallel Scheme VM creation/entry/root isolation/destruction, both complete
Library paths, full regressions and the existing Scheme DSL loading timeout
remain open. No completed dual-backend or multi-VM release is claimed.

## Frozen Source Snapshot 020

New builds and installed tests are recorded in `WHEEL_QUALIFICATION.md`.
They use the actual GitHub-pinned ASCENT Cargo dependency, with its exact Git
objects supplied from the local upstream repository. The earlier network
timeout remains a failure; local Git transport is not remote-fetch success.
The Rust-only, Scheme-only and combined feature builds have distinct wheel
identities. Combined-feature selection has been tested without Scheme entry
on the Rust path. Independent Scheme VM execution remains unimplemented.
