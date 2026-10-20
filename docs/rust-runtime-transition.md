# Rust Execution Runtime Transition

## Current Contract - 2026-10-19

Read [Hybrid compilation and runtime ownership](hybrid-execution.md) for the
current contract. Rust dispatch hosts complete Scheme runtimes on fixed owners;
“Rust backend” does not imply “no Scheme VM”. Scheme retains its GC, closures,
continuations, dynamic state and actor/green-thread scheduler. Native Gerbil/C
compilation is preserved. Scheme-to-Rust IR lowering is a separate subset tool,
not the full Scheme execution strategy.

The [experimental Darwin instance API](instance-runtime.org) is implemented and
has native lifecycle, continuation and same-VM batch receipts. The failed ordinary
shared-runtime entry below remains failed; private images are a distinct path.
The ledger now has an installed four-owner native path; its subsequent grouped-module package passes the tested expander and shutdown gates. See [the integration receipts](30-39-status/30.10-python-native-owner-integration.org). Portability, hot unloading and complete wheel migration remain unqualified.

## Historical Python Subset Selection - 2026-10-15

The Python examples selected `extension-module,rust-backend` and explicit
`backend="rust"` for their existing subset providers. The ledger has since moved to the Scheme instance API; the ASCENT kernel adapter
has not. That historical choice is not a
requirement to exclude Scheme from the library's Rust-scheduled execution host.
No provider is silently replaced when it is unavailable or unsupported.

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
The independent ledger now executes its Scheme program on four native owners
through the default Rust dispatch provider. The previous typed lowering remains
a historical subset experiment and is not selected by this path.

Neither example establishes general Scheme compatibility. The ASCENT adapter
supports the existing typed relational `(ascent ...)` subset, not arbitrary
Scheme or complete `define-evidence` modules. The ledger preserves native Scheme execution and opaque rooted graphs/procedures,
and its subsequent runtime-loaded expander/DSL tests pass; see [the native packaging repair](30-39-status/30.11-expander-startup-and-dispatch-benchmarks.org). Resolving that embedding
boundary belongs in the native library, not a Python interpreter or a Rust rewrite.
The completed O2 wheel passes the exercised native Scheme gates;
[the O0/O2 report](30-39-status/30.13-progress-supervised-o2-qualification.org)
retains the actual mixed timing results and blocked quiet-host admission.

The actual Gambit-SMP consumer still has richer catalog/review/history DSL and
result/explanation contracts. Its dependency and interface are not silently
rewired to this smaller qualification adapter. Full consumer migration,
complete current graph and matched performance remain open until supported
by the actual Rust Library and qualified against the same complete inputs.

## Admission Of The Historical Rust-Only Wheels

New wheels must have exact source, dependency and wheel identities. Installed
tests must run without SDK paths or a compiler and must verify independent
state, concurrency, withdrawal, retained snapshots, close/invalidation and
explicit unsupported inputs. Inspect linked dependencies and symbols to
ensure those explicitly Rust-only kernel artifacts match their declared runtime.
This exclusion is not the admission rule for a Rust host carrying Scheme VMs. Preserve all
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
