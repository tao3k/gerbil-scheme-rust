# Explicit Execution Backends

The execution contract is [native Scheme execution with Rust dispatch](../../docs/hybrid-execution.md). Scheme application code remains on the native Gerbil/Gambit/C path. The former Scheme-to-Rust IR and VM-free qualification examples have been retired.

`select_backend` selects a library-owned provider named `scheme` or `rust`.
A library may declare its default; the ledger defaults to `rust`. `auto`, unknown names and missing providers are errors.
Selection does not parse DSL, compile code or invoke another provider after a
failure. Python exposes the choice but does not implement either language.

Provider availability is fixed by Cargo features, not runtime probing. The
support crate defaults to resource-only code. `scheme-backend` includes the linked Scheme bridge; `instance-backend` includes Python transport and complete fixed-owner Scheme instances. The old support-crate `rust-backend` feature is removed. The ledger has its own `rust-backend` feature selecting `instance-backend`. Installed applications select only providers compiled into their wheel and never run a compiler to enable a missing provider.

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
owner; sessions on one owner share its Scheme module state. `session.call_many` submits stateful work in input order. Its explicit
`dispatch="direct"|"tokio"|"rayon"|"rayon-blocking"` and
`admission_window=1..128` controls preserve that owner and within-batch execution
order; defaults remain direct and 128. Tokio admits ordered native asynchronous
tickets before supervising replies. Rayon admits the ordered stream on its
worker pool; the blocking reference holds that worker while waiting. Concurrent
batches may interleave and a batch is not a transaction. The experimental
`tokio-pipeline` remains stateless-only and rejects pinned requests before entry. `call_stateless` and `call_many` reserve
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

The linked `scheme` provider remains a separate process-global linked owner.
The experimental private-image provider is Darwin-only. The subsequent grouped native-module package passes the tested four-owner
expander, hygienic macro, closure/GC and shutdown gates inside the original
five-second operation budget. Prior failures remain in 30.10; see [the repair
and direct/Tokio/Rayon matrix](../../docs/30-39-status/30.11-expander-startup-and-dispatch-benchmarks.org). This does not establish universal Scheme conformance or optimized phase-code performance.
The actual O2 wheel also passes these exercised capability gates; see
[the progress-supervised O0/O2 qualification](../../docs/30-39-status/30.13-progress-supervised-o2-qualification.org).
All matched macro timing cohorts were noisy, and quiet-host macro and dispatcher
performance admission remains pending.

Both providers verify the package's complete per-path SHA inventory before
opening. A result's successful scalar projection does not prove complete DSL
coverage, source refinement, or performance equivalence.

## Qualification records

The current native [wheel qualification](WHEEL_QUALIFICATION.md) and [10.10x matched baseline](../../docs/30-39-status/30.19-owner-resident-payload-baseline.org) state tested behavior and limits. The retired VM-free ASCENT and typed-fold wheels are historical subset experiments and do not qualify complete Scheme execution. Raw diagnostics and rejected patches are archived outside the repository.

## Scheduler safety and failure semantics

The owner queue has finite capacity. Overload may return `QueueFull`; accepted
work and state transitions are not rolled back, and there is no automatic retry.
`rayon-blocking` can instead limit producer progress by occupying the finite
Rayon workers while they wait; this may starve other otherwise-idle VM owners.
These controls are not interchangeable latency guarantees.

A 128-request group shares its original five-second observation budget across
smaller turns. Direct and Tokio admission check that budget before enqueueing
new work, like the Rayon and pipeline paths. Observing/draining accepted results
and retiring roots may take additional time. Timeout does not cancel Scheme.

Use the maintained `bridge-python-dispatch-startup`, `bridge-python-dispatch-stress`,
`bridge-python-owner-deadline` and `bridge-python-resident-dispatch-ab` targets.
Keep generated output outside the repository. Qualification status and open
industrial gates are recorded in the dispatch qualification report.

## Sustained acceptance boundary

The finite owner-lifecycle Quint model is separate from native acceptance.
The preserved DSL and startup failures were traced to the SDK normalizer's
process-wide cwd mutation across independent images. The Darwin instance
adapter confines directory operations to each fixed owner; the common lifecycle
also preserves host SIGINT/SIGTERM/SIGCHLD handlers. Scheme execution, native
types, GC, continuations and Actor/green-thread scheduling remain native.

The repaired O2 artifact completes the original 600-second reuse and eight-cycle
lifecycle targets, plus actual API, host-signal and initialization-failure
controls. Every completed close has zero external roots, removed files and
retired owner threads. This is finite acceptance, not industrial certification.
Initialized images remain mapped: the eight-cycle capture grows from 263.1 to
731.2 MiB RSS, with 32 retained images. Complete RSS return and indefinite leak
freedom are not established. Darwin thread-directory private SPI is an explicit
platform dependency; other OS-global resources require separate qualification.
The follow-up distinguishes retained image mappings from allocator residency:
32 images represent eight retired four-owner pools; live malloc remains about
4.34--4.36 MiB in the mapping-observer capture despite 263.4--684.0 MiB RSS.
A separate maintained-observer repeat reaches 768.5 MiB RSS with 4.98 MiB live
malloc. No large outstanding Scheme heap leak is established, but small
residual allocations are not all attributed. The current no-unload policy is
conservative; it is not proof that every mapping is intrinsically necessary.
Fresh pool replacement remains unbounded in mappings. Reuse the fixed pool;
general native callback/TLS retirement must be qualified before hot unload.

See `docs/30-39-status/30.23-owner-lifecycle-model-and-native-reuse.org` in the
repository root for the exact artifact, preserved failures, controls and limits.
