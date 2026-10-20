# Native Scheme wheel qualification

The default ledger Rust backend includes four complete Scheme runtimes on fixed owner OS threads. Gerbil/Gambit/C native compilation and native type interfaces remain active. Historical VM-free typed-fold and ASCENT adapters have been removed; their subset observations do not qualify this runtime.

The current [owner-resident payload baseline](../../docs/30-39-status/30.19-owner-resident-payload-baseline.org) records the latest 10.10x matched throughput observation and 72.045 ms scheduled short-request p99. The older 21.18x result was not reproduced in magnitude. Read-only native object reuse, preparation cost, finite fairness and cleanup conditions are stated in that report.

The [projection experiment](../../docs/30-39-status/30.20-scalar-projection-experiment.org) passed its candidate capability tests but failed performance admission. Its experimental API was not retained. No improvement is claimed from that experiment.

Maintainers build through the existing Cargo/maturin native O2 chain. Consumers use installed native types and owner-affine calls. Verification must include full macro expansion, native types, GC, multi-shot continuations, exception recovery, initialization and retirement, and matched workload performance. A source/resource test, local wheel result, remote CI and portable distribution are distinct evidence.

Keep raw logs, wheel diagnostics, experimental patches and source snapshots outside the repository. Version maintained tests, English conclusions and reproduction commands. Native private images remain experimental on Darwin; zero roots and runtime-file removal do not establish complete RSS return or indefinite leak freedom.
