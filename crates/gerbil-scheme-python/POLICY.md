# Source responsibilities and verification

The Cargo test gate uses the pinned workspace `asp-rust` dependency in strict
mode. Responsibility hints describe source owners; they are not receipts.
`asp-rust` is inherited only under each crate's `[dev-dependencies]`.
The workspace version declaration does not add a normal/build dependency.
There are no rule suppressions, waivers, advice allowances, task-kind overrides
or disabled verification tasks in this crate's gate.

| Source owner | Actual responsibility | Required checks |
| --- | --- | --- |
| `src/lib.rs` | Public feature-gated exports | Feature-specific compilation and API tests |
| `src/backend.rs` | Pure explicit provider selection and public errors | Missing/unknown backend rejection, no auto/fallback |
| `src/build_source.rs` | Build-time source inventory root for the actual compiled Cargo dependency | Exact manifest ownership and complete upstream source/header availability; never an installed runtime path |
| `src/package/inventory.rs` | Public manifests, filesystem admission, persisted resource identities and trust boundary | Full per-path SHA, complete inventory, aliases, malformed/duplicate paths, relocation |
| `src/package/publication.rs` | Persistent generated output, filesystem operations and ownership boundary | Verified previous inventory, changed-byte denial, collision/overlap denial, source preservation |
| `src/staging.rs` | Public staging API, filesystem reads/writes and source trust boundary | Complete trees, source drift, aliases, fresh output |
| `src/closure.rs` | Public dependency-closure transport, filesystem operations and exact-source mapping | Unknown schema, duplicate files, ambiguous roots, complete source inventory |
| `src/runtime.rs` | Shared native protocol, service topology and lifetime | Process/fork checks, idempotent retirement, actual selected host |
| `src/runtime_library.rs` | Python library API and owner routing | Native type admission, explicit modes, errors and bulk projection |
| `src/runtime_value.rs` | Python native result handles | Affinity, apply, projection failure and release |
| `src/runtime_input.rs` | Host input conversion and trust boundary | Cycles/depth, foreign handles, exact native values |
| `src/runtime_linked.rs` | Linked Scheme owner startup, execution and codec | Single-owner lifecycle and native root balance |
| `src/dispatch_runtime.rs` | Tokio executor owner and supervised task handles | Fixed worker count, observed handles and shutdown |
| `src/session.rs` | Owner-affine stateful batches | Same-owner identity, within-batch execution order and failures |

The runtime modules are not pure domain logic: they start owner threads, enter
Scheme, wait for replies and own VM roots. Source hints do not
prove cancellation safety, bounded callbacks, multiple independent VMs or
performance. Resource-only Cargo tests do not qualify that runtime branch.

Manifest checks use component ancestors and ordered sets, not string prefixes
or repeated scans of all declared roots. Publication replaces only verified
build-owned roots. Root additions/removals are supported; changing a root into
an ancestor/descendant of an old root is denied. Failed partial publication
retains staging/backup data and is not an admitted package. The publisher does
not claim concurrent-writer serialization, multi-root atomicity or durability.

The native instance backend owns the complete Scheme runtime path. The retired Scheme-to-Rust IR compiler and VM-free qualification examples are not part of this package's verification matrix.

```sh
cargo test --locked -p gerbil-scheme-python
```

Compile `instance-backend,external-program` and `scheme-backend,external-program` separately in the maintainer build
environment. Run each standalone example's `integration_test` target with an
explicit compiled backend. Installed-wheel tests remain independent acceptance
requirements; downstream applications never receive a compiler or SDK setup.
