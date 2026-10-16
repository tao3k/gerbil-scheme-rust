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
The caller must choose; `auto`, unknown names and missing providers are errors.
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

The Rust provider's sum is authored in `scheme/rust-ledger.ss`, lowered by the
existing typed IR compiler and compiled into the extension at build time. Calls
execute Rust, without a Scheme VM. Its supported operations are `append`,
`total`, `entries` and input/output `echo`; this is a precompiled typed-function
boundary, **not** a runtime interpreter for general Scheme definitions. Dynamic
`dsl`, macros, arbitrary procedures and other unsupported operations raise
`NotImplementedError`, with no Scheme fallback. No compiler is shipped or run
by the installed consumer.

The Scheme provider retains the rooted result API and official module loading.
It explicitly initializes one processor on its owner thread. Independent,
parallel VM ownership is not yet implemented; see `MULTI_VM.md`. Selecting
`scheme` must not be described as working multi-VM support.

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
