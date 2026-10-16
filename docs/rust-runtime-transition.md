# Rust Execution Runtime Transition

## Current Clarification - 2026-10-16

Read [Hybrid compilation and runtime ownership](hybrid-execution.md) for the
current architecture decision. Rust owns the active execution host; this does
not require rewriting Scheme algorithms in Rust or introducing another Scheme
interpreter. Reuse typed ABI exports where their runtime requirements are met,
and the existing Scheme-to-typed-IR build pipeline for VM-independent kernels.
Rooted Scheme exports still require their live VM. The historical receipts
and open VM-entry gaps below are unchanged; this clarification admits no new
runtime, DSL, performance or release capability.

## Decision

On 2026-10-15 the user redirected the active Python execution runtime to Rust.
This means Rust owns execution, session state and concurrency; it is not a
Rust scheduler around a Scheme VM. Cargo features select the provider at build
time. Installed callers must explicitly request `backend="rust"`; unavailable
providers and unsupported DSL fail without fallback.

`examples/python-ascent/pyproject.toml` and
`examples/python-ledger/pyproject.toml` select `extension-module,rust-backend`.
The support crate's resource-only defaults and explicit Cargo feature contracts
are unchanged. Standard builds of these examples do not enable `scheme-backend`.
The Scheme backend and its tests remain as historical/reference assets, not a
silent alternate execution route or a condition for Rust acceptance.

## Recorded Scheme Entry Gap

The real standalone `scheme/runtime-vm-test.ss` constructed two live VM states
inside the existing `1cfb032` runtime, with different addresses, each using its
own processor 0 and one active processor. The official selector did not select
either new processor at foreign entry. Both owners reported `selected=0` and
`outcome=13`; the test exited 70 without timeout in 0.706068542 seconds under
15 seconds. The following Scheme parallelism, root-isolation and independent
destruction checks were not reached and are not admitted.

Gambit source and installed runtime bytes were not edited or rebuilt. The
module-loading check through the corresponding `gxi` binary succeeded, but
module loading is not a VM selector. The private-image instance probe does not
qualify a shared global runtime. All original failure receipts remain intact.
This diagnosis is bounded to the tested binary/configuration and entry path.

## Existing Rust Execution And Open Work

The ASCENT qualification uses the pinned upstream Rust `Program`, `Engine`,
`Session` and immutable `Snapshot`, including source withdrawal and concurrent
independent sessions. Python transports inputs and results; it does not parse
or infer the DSL. Opaque projection failure preserves the complete snapshot.
The independent ledger executes typed functions lowered to Rust at maintainer
build time. Its installed execution does not call Scheme.

Neither example establishes general Scheme compatibility. The ASCENT adapter
supports the existing typed relational `(ascent ...)` subset, not arbitrary
Scheme or complete `define-evidence` modules. The ledger has a precompiled
typed-function interface, not a general runtime Scheme interpreter. Extending
these semantics belongs in the Rust Library, with explicit capability and
conformance tests, not Python special cases.

The actual Gambit-SMP consumer still has richer catalog/review/history DSL and
result/explanation contracts. Its dependency and interface are not silently
rewired to this smaller qualification adapter. Full consumer migration,
complete current graph and matched performance remain open until supported
by the actual Rust Library and qualified against the same complete inputs.

## Required Admission

New wheels must have exact source, dependency and wheel identities. Installed
tests must run without SDK paths or a compiler and must verify independent
state, concurrency, withdrawal, retained snapshots, close/invalidation and
explicit unsupported inputs. Inspect linked dependencies and symbols to
ensure the Rust profile has not included the Scheme runtime. Preserve all
failed builds and executions. Existing build and execution deadlines remain
120 seconds and 15 seconds respectively; no local subset pass is whole-project
or latest-revision acceptance.

## Local Rust-Only Wheels (2026-10-15)

Using source HEAD `2c82b845c141ce68147cf74cb5a4ec680c6ffb07` plus the current
uncommitted bridge changes, cached maturin 1.11.5 and locked offline Cargo
dependencies, both standard wheel profiles were actually rebuilt:

| Artifact | Build / 120s | Installed tests / 15s | Wheel SHA256 |
| --- | --- | --- | --- |
| ASCENT qualification 043 | 40.739691167s | 4 passed, 0 skipped; 0.783637334s | `bd13d078cb0ac41165add5a0c54846f07790815b8d300d0174b8c9f4fba19fb8` |
| Independent ledger 044 | 12.479360333s | 6 passed, 0 skipped; 0.539701292s | `44667334496240a2425ff4eea90c02598ed55f84024bd996a1aaa5509f6b7f7b` |

Each test process exited 0, did not time out, and its process group was absent
afterwards. The installed test sandbox denied Gerbil SDK paths, both original
source repositories and compiler execution. `otool -L` showed only libSystem
and libiconv, apart from each extension's install name. A symbol search found
no Gambit setup/call/cleanup/global-state or bridge runtime-init symbol. Ledger
tests additionally checked the absence of compiled Scheme runtime resources.

The ASCENT upstream dependency remains exactly
`e6884b83c7b141ec66e2095252e4fb9e17da9869`; this is not a claim to use a newer
remote revision. Wheel platform tags are not cross-platform qualification.
These are real installed Rust subset results, not complete rich-DSL consumer
migration, full-current-graph admission, Scheme VM repair or a performance
comparison.

Fresh tool acquisition failed first with DNS errors and then at its original
15-second download cap. The tool used instead was an existing complete local
cache artifact, SHA256
`0ccdcfc210eab0a11c62594cca3dbc12768a58055608d087e3ebb2911f0066e6`.
The first sandbox invocation was denied by the outer sandbox; the subsequent
explicitly approved invocation applied the same SDK/source/compiler-denying
profile. No unsandboxed run was substituted as installed qualification.
All raw observations remain in the consumer's local
`.data/upstream-python-015/rust-*.json`; no release or Git-durable evidence
admission is claimed.

The modified ASCENT adapter also passed its existing ASP source-policy gate
using revision `1ca41124b846dec947f3d05119cafa07eaa57924`: one test, no skips,
exit 0, no timeout, 38.771824292 seconds including dependency compilation
under 120 seconds. No policy waiver or warning downgrade was introduced.
This is not whole-workspace CI.
