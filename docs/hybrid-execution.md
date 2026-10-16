# Hybrid Compilation And Runtime Ownership

## Architecture Decision - 2026-10-16

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
that actual requirement. Do not pretend a VM-rooted export is independent. Add
the reusable ABI/ownership capability upstream and qualify a genuinely independent
algorithm export; until then the consumer capability remains open. Do not silently
rewrite the algorithm, initialize a VM or call another backend to hide the gap.

**A Rust execution host does not require rewriting Scheme algorithms in Rust.**
The intended architecture preserves Scheme-authored algorithms and DSL semantics,
reuses the upstream typed ABI and build-time lowering facilities, and assigns
host scheduling, owned transport, handles and lifecycle to Rust. Python is a
consumer interface, not a language implementation or a compiler workflow.

This decision clarifies the earlier [Rust runtime transition](rust-runtime-transition.md).
It does not reverse the active restriction against entering the unqualified
Scheme VM path. It also does not declare every Scheme export VM-independent.
Hybrid compilation is an intentional architecture, **not a deviation from the
Rust-host decision**. Future changes and reviews must use this distinction.

## Why Hybrid Compilation

- Keep algorithms and reusable DSL definitions in their owning Scheme library,
  rather than maintain a second handwritten implementation for each consumer.
- Reuse `gerbil-scheme-rust` for typed lowering, ABI conversions and packaging;
  do not repeat these mechanisms in ASCENT or other downstream libraries.
- Let Rust supply ownership, scheduling and parallel execution where qualified,
  without making Scheme VM availability a prerequisite for independent kernels.
- Keep applications installable from a wheel. Maintainers build the library;
  downstream Python callers do not supply an SDK, module compiler or build tree.
- Preserve source-to-artifact provenance so generated code remains auditable.

An FFI boundary is not inherently faster than a direct generated function call.
Reuse the existing compiled algorithm through its qualified boundary. Use
C ABI/FFI for an external algorithm boundary, with whole-value or batched transfers where
appropriate. Measure the complete matched path, including conversion, validation,
allocation and teardown; do not infer a speedup from the language choice.

## Three Different Execution Paths

| Path | Algorithm origin and execution | Scheme VM requirement |
| --- | --- | --- |
| Linked Scheme AOT export | Scheme compiled into the linked module graph, called through its typed C ABI | Required when the export uses Scheme objects, roots, GC, closures, continuations or module initialization |
| Independently compiled algorithm export | A kernel whose ABI, allocation and initialization contract is demonstrably independent of a Scheme heap | No VM only after that exact artifact and contract are qualified; not implied by a `.c` file or an `extern "C"` declaration |
| Scheme-authored typed IR lowered to Rust at build time | Scheme produces admitted function IR; the upstream compiler emits checked Rust functions compiled into the library | No Scheme VM during installed execution; the maintainer build may use Scheme tooling |

These paths describe implementation choices, not automatic runtime fallback.
Cargo features determine which providers are built. An explicit provider choice
must not silently enter another provider after a missing capability or failure.
The active VM-independent path must not initialize Scheme through a hidden FFI
call. An independently compiled C algorithm export is a contract category, not
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
  those owned values without acquiring Python or a Scheme runtime. Producers own
  captures, algorithms, state transitions and result allocations; Rust validates
  transport and manages release. See the [contract and qualification](owned-c-abi.md).
- [`gerbil-scheme-rust-ir`](../build-support/gerbil-scheme-rust-ir/src/compiler.rs)
  lowers typed build-time function IR and validates emitted Rust syntax. It does
  not parse arbitrary Scheme source or prove source-level semantic equivalence.
  Source-level purity and call admission remain the producer's responsibility.
- The [ledger Scheme producer](../examples/python-ledger/scheme/rust-ledger.ss)
  and its [build integration](../examples/python-ledger/build.rs) demonstrate
  Scheme-authored summation lowered into the installed Rust provider. This is a
  real precompiled algorithm route, not a general Scheme interpreter.

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
without downstream SDK/compiler access or Scheme VM entry on the independent
path. Check full answers, closure capture, component-state isolation, higher-order
composition, source withdrawal, close/invalidation and teardown as applicable.

Keep all failed builds, timeouts, missing-source and unsupported-capability
receipts. Do not increase execution budgets, downgrade policy or use a local
subset pass as whole-consumer, current-graph, performance or Git-release closure.
Match performance inputs and complete outputs. Verify publication and installation
identity independently of a successful local build.
