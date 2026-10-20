# Hybrid Compilation And Runtime Ownership

## Current Runtime Contract - 2026-10-19

The Scheme execution backend is **Rust dispatch -> fixed OS owner -> complete
Gerbil/Gambit runtime -> native Scheme execution**. Each owner retains its VM
across requests. Rust routes owned inputs/results across instances and manages
admission and shutdown. Scheme owns closures, continuations, dynamic bindings,
exceptions, GC and the actor/green-thread scheduler inside each instance.
Rust does not schedule individual Scheme green threads.

**“Rust backend” does not mean “no Scheme VM”.** A C ABI, a Rust worker or a
packaged shared library does not remove the Scheme runtime. The normal Scheme
build remains Gerbil -> Gambit C -> native library; Scheme-to-Rust lowering is
not a prerequisite and cannot replace this full execution contract.

The implemented entry is the experimental Darwin
[`InstancePool`](../crates/gerbil-scheme/src/instances/pool.rs), with
[`submit_native_on`](../crates/gerbil-scheme/src/instances/native_dispatch.rs)
and [`submit_numeric_batch_on`](../crates/gerbil-scheme/src/instances/batch.rs).
Native types are constructed and rooted on the selected owner. Scheme executes
the application, including cooperative actor waits, before owned results return.
See [runtime/lifecycle limits](instance-runtime.org) and
[the measured qualification](30-39-status/30.08-native-batch-tail-attribution.org).
This is not shared-VM SMP or a claim of general crash isolation/hot unloading.

The installed ledger example now defaults to Rust dispatch with four real
fixed-owner Scheme runtimes. It uses native types and ordinary native compilation,
not its historical typed Rust lowering. See [the measured integration](30-39-status/30.10-python-native-owner-integration.org).
Its subsequent grouped-native-module package passes the tested full-expander
and shutdown cases; see [the repair and four-mode benchmark matrix](30-39-status/30.11-expander-startup-and-dispatch-benchmarks.org). The earlier Tokio regression remains a historical observation. The separate Python ASCENT example still
uses its finite Rust kernel provider; it does not establish complete Scheme
execution. Missing providers fail explicitly without fallback.
The completed O2 wheel and actual O0/O2 macro comparison are recorded in
[the progress-supervised qualification](30-39-status/30.13-progress-supervised-o2-qualification.org).
The tested Scheme capabilities pass; noisy timing negatives remain and
quiet-host performance admission is still pending.

## Native Compilation Boundary

### Primary Integration Path: Compiled C ABI

The primary migration path is **existing Scheme/C algorithm export -> typed
C ABI -> gerbil-scheme-rust ownership/conversion -> thin Rust host -> Python**.
Gerbil's C embedding and foreign-export facilities are part of this design.
Do not bypass them by systematically rewriting library algorithms in Rust or
by introducing a second interpreter. Large new Rust algorithm implementations
are not justified merely because the execution host is Rust.

Scheme-to-IR-to-Rust AOT is a distinct, explicitly scoped compilation facility,
not evidence that existing C ABI algorithms have been reused, and not the
default substitute for missing C exports. A compiler test or generated Rust
closure does not qualify the C ABI callable/state contract.

When an existing export requires Scheme objects, GC or VM initialization, record
that actual requirement and host the required runtime explicitly. Do not pretend
a VM-rooted export is independent. Reuse the owner-bound ABI rather than remove
Scheme capabilities to satisfy an obsolete VM-independence requirement. Provider
selection and initialization must be explicit; unsupported entry remains an error.

**A Rust execution host does not require rewriting Scheme algorithms in Rust.**
The intended architecture preserves Scheme-authored algorithms and DSL semantics,
reuses the upstream typed ABI and build-time lowering facilities, and assigns
host scheduling, owned transport, handles and lifecycle to Rust. Python is a
consumer interface, not a language implementation or a compiler workflow.

This decision clarifies the earlier [Rust runtime transition](rust-runtime-transition.md).
It supersedes the earlier blanket VM-independent runtime requirement. The
unqualified shared-runtime multi-VM entry remains a failed route; the explicit
private-image owner backend has its own bounded native evidence. A Rust host
and a complete Scheme VM are compatible parts of the same execution path.

## Why Hybrid Compilation

- Keep algorithms and reusable DSL definitions in their owning Scheme library,
  rather than maintain a second handwritten implementation for each consumer.
- Reuse `gerbil-scheme-rust` for typed lowering, ABI conversions and packaging;
  do not repeat these mechanisms in ASCENT or other downstream libraries.
- Let Rust supply ownership, scheduling and parallel execution where qualified,
  while retaining the required Scheme runtime for Scheme execution.
- Keep applications installable from a wheel. Maintainers build the library;
  downstream Python callers do not supply an SDK, module compiler or build tree.
- Preserve source-to-artifact provenance so generated code remains auditable.

An FFI boundary is not inherently faster than a direct generated function call.
Reuse the existing compiled algorithm through its qualified boundary. Use
C ABI/FFI for an external algorithm boundary, with whole-value or batched transfers where
appropriate. Measure the complete matched path, including conversion, validation,
allocation and teardown; do not infer a speedup from the language choice.

## Execution Paths Must Name Their Runtime

| Path | Algorithm origin and execution | Scheme VM requirement |
| --- | --- | --- |
| Linked Scheme AOT export | Scheme compiled into the linked module graph, called through its typed C ABI | Required when the export uses Scheme objects, roots, GC, closures, continuations or module initialization |
| Independently compiled algorithm export | A kernel whose ABI, allocation and initialization contract is demonstrably independent of a Scheme heap | No VM only after that exact artifact and contract are qualified; not implied by a `.c` file or an `extern "C"` declaration |
| Scheme-authored typed IR lowered to Rust at build time | Scheme produces admitted function IR; the upstream compiler emits checked Rust functions compiled into the library | No Scheme VM during installed execution; the maintainer build may use Scheme tooling |

These paths describe implementation choices, not automatic runtime fallback.
Cargo features determine which providers are built. An explicit provider choice
must not silently enter another provider after a missing capability or failure.
Only a provider explicitly advertised and qualified as VM-independent must
exclude Scheme initialization. That is not a condition on the Scheme backend. An independently compiled C algorithm export is a contract category, not
a claim that this repository already supplies every such export.

## Existing Implementations And Limits

- [`GerbilProgramLinker` and initialization](../crates/gerbil-scheme-sys/src/abi_program.rs)
  bind a generated Gambit module graph to a live runtime. Initializing one Scheme
  processor does not admit independent parallel VMs.
- [`LinkedObjectExport` and related exports](../crates/gerbil-scheme/src/runtime/program.rs)
  retain a `GerbilRuntime` owner and rooted objects. Calling these exports through
  C does **not** remove their VM, GC or thread-affinity requirements.
- [`OwnedValue`](../crates/gerbil-scheme-python/src/values.rs) is an existing
  VM-independent Python/Rust transport. Reuse it for supported owned data instead
  of copying conversion code. It is a tree representation, not a cyclic graph,
  closure evaluator or permission to dereference a foreign handle. See
  [owned-value limits](../crates/gerbil-scheme-python/OWNED_VALUES.md).
- The optional `c-abi` feature exposes
  [`gerbil_scheme_owned.h`](../include/gerbil_scheme_owned.h) and
  [owner-thread adapters](../crates/gerbil-scheme-python/src/c_abi.rs). It reuses
  those owned values without acquiring Python or implicitly initializing a
  Scheme runtime. `NO_VM` and `SCHEME_VM` explicitly distinguish callback runtime
  needs. A Scheme provider/caller retains the required live VM on its fixed owner;
  the flag does not acquire that lifetime. Producers own captures, algorithms,
  state transitions and result allocations; Rust validates transport and release. See the [contract and qualification](owned-c-abi.md).
- [`gerbil-scheme-rust-ir`](../build-support/gerbil-scheme-rust-ir/src/compiler.rs)
  lowers typed build-time function IR and validates emitted Rust syntax. It does
  not parse arbitrary Scheme source or prove source-level semantic equivalence.
  Source-level purity and call admission remain the producer's responsibility.
- The [ledger Scheme producer](../examples/python-ledger/scheme/ledger.ss)
  and its [build integration](../examples/python-ledger/build.rs) now use the
  ordinary Gerbil/Gambit native graph and fixed-owner Scheme runtimes. The old
  `scheme/rust-ledger.ss` file is historical subset input, outside this default
  execution path. Installed full-expander cases now pass with grouped native modules; portable
    packaging and optimal phase-code performance remain open.

ASCENT likewise has a maintainer-side producer at `program/rust-execution.ss`
in its own repository. That example must not become an ASCENT-specific compiler
or dispatcher in this bridge. General lowering/type capabilities belong here;
relations, rules, Graph semantics and research decisions remain in ASCENT.

**Open capability:** the current typed function IR is not a general exported
closure/capture/mutable-component-state contract. Existing folds and local
bindings do not establish support for arbitrary higher-order Scheme programs.
Closing a C ABI consumer gap first requires its typed export/ownership contract
and real installed consumer qualification; adding Rust IR nodes alone does not
close it. Do not replace the gap with a handwritten Rust Scheme
interpreter, a Python evaluator, a case-name registry or a silent VM fallback.

## Ownership And Extension Rules

1. Start with the existing C ABI types, safe Rust wrappers, owned transport and
   typed IR compiler. Check whether a type is VM-rooted before reusing it on a
   VM-independent path; a Rust wrapper alone is not evidence of independence.
2. Keep source algorithms and DSL semantics in the owning library. Extend the
   generic compiler or bridge for a genuinely reusable capability; do not copy
   application algorithms into generic bridge modules.
3. Specify closure captures, mutable-state ownership, argument/result types,
   effects, error propagation, source identity, concurrency and invalidation
   before admitting new callable exports. Do not turn a captured Scheme heap
   pointer into an allegedly independent Rust handle.
4. Reuse an existing owner/generation protocol when one applies. Cross-owner,
   stale, released and mismatched-artifact references must remain errors.
5. Package the exact required artifact/resource closure, not an entire SDK.
   Keep per-path hashes, generation inputs, ABI version and dependency identity.
   Installed consumers must not repair missing resources with build-machine paths.
6. Keep execution results and explanation/projection capabilities separate.
   Missing projection is a declared gap, not evidence that an authorized kernel
   failed, and not permission to fabricate a proof or execution authorization.

## Qualification Is Separate From This Decision

A documentation decision or compiler contract test does not close the runtime
or downstream migration. For each new capability, bind the source revision and
complete source inventory to the generated code, library and wheel. Qualify
installed wheels with both ASCENT and an independent Scheme-authored library,
without downstream SDK/compiler access. For the full Scheme path, verify the
packaged VM is actually initialized, reused and retired on its owner. Only an
explicitly independent kernel has a no-Scheme-entry gate. Check full answers, closure capture, component-state isolation, higher-order
composition, source withdrawal, close/invalidation and teardown as applicable.

Keep all failed builds, timeouts, missing-source and unsupported-capability
receipts. Do not increase execution budgets, downgrade policy or use a local
subset pass as whole-consumer, current-graph, performance or Git-release closure.
Match performance inputs and complete outputs. Verify publication and installation
identity independently of a successful local build.
