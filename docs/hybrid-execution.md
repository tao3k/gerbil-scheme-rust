# Native Scheme execution and Rust dispatch

## Execution contract

Scheme application code follows the native Gerbil -> Gambit C -> native compilation chain. Rust owns host admission, cross-instance dispatch and lifecycle. Complete Scheme runtimes execute the application on fixed OS owner threads, retaining macros, GC, closures, continuations, dynamic bindings, Actors and green threads.

The default Python ledger Rust backend starts four independent native Scheme instances. Tokio or Rayon workers submit jobs to those owners; they do not execute or migrate a Scheme VM. Each VM uses one Scheme processor. A same-process native crash can affect the whole process; independent heaps are not crash containment.

The old Scheme-to-Rust IR compiler, ledger typed-fold source and VM-free ASCENT qualification adapter have been removed. There is no retained compatibility alias or automatic no-VM fallback. The support crate's unused `rust-backend` feature has also been removed. The ledger's `rust-backend` feature remains and explicitly selects `instance-backend`.

## Native values and ownership

Use the existing native type interfaces. Stateful sessions pin to one owner; sessions on that owner share its Scheme state. Stateless requests declare independence, follow round-robin owner assignment and reject native references. A native handle belongs to its originating owner and library. Scheme objects and continuations never migrate between heaps.

`session.value(data)` retains an actual Scheme graph. Reusing that handle can avoid repeated Python traversal and native input construction. It preserves identity and therefore does not promise immutability. The benchmark's payload is read-only; mutable application state requires its own owner/order contract.

`prepare_request(data)` is an immutable host-owned input description. Each invocation constructs fresh Scheme objects. It is useful for repeated portable inputs and cannot contain native references. It does not execute Scheme in Rust.

`project_many(..., release=True)` materializes all Python values before optional owner-batched retirement. Projection failure preserves native procedures, cycles and continuations. Release delivery can retire a subset; idempotent release supports retry. Original bounds and observation deadlines remain active; timeout does not cancel accepted native work.

`OwnedValue` and the optional C ABI still describe owned transport and callback lifetimes. Their `NO_VM` flag can describe copied host values or an independent C callback; it does not select a Scheme backend or authorize evaluating Scheme without a VM. `SCHEME_VM` callbacks require their live owner VM through final release. Flags do not acquire leases or establish runtime safety by themselves.

## Build and packaging

The existing Gerbil build owns expansion, compilation, dependency closure and module grouping. Cargo builds and packages native artifacts. Python consumers use the installed wheel and native APIs. Do not add a Python compiler, handwritten Rust Scheme evaluator, subset lowering pipeline, case registry or silent provider fallback.

The separate linked `scheme` provider retains a process-global Scheme runtime and is an explicit single-owner control. It also executes Scheme. It does not qualify independent parallel entry into a shared VM.

## Performance and qualification

The working owner-resident payload observation is 10.10x throughput versus rebuilding the same read-only inputs, with scheduled short-request p99 72.045 ms. This is a bounded matched observation, not a fixed gate, intrinsic Gambit speedup or one-to-four VM scaling result. See [the baseline](30-39-status/30.19-owner-resident-payload-baseline.org).

A scalar projection experiment did not establish a stable endpoint gain and was not admitted. Continue studying temporary roots, native traversal and GC allocation while keeping generic types and failure semantics. See [the non-admission report](30-39-status/30.20-scalar-projection-experiment.org).

Qualify changes through real installed Python calls, native types, GC, multi-shot continuations, exceptions, initialization/retirement, matched same-VM performance, p99 and finite fairness observations. Source checks, models, local runs, CI and portability are distinct evidence. The private-image backend remains experimental on Darwin.

Raw logs, performance captures, experimental patches and source snapshots are kept outside the repository. Maintain English summaries, reproduction commands and executable tests in source control.
