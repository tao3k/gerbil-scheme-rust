# Scheme runtime ownership under Rust dispatch

The current architecture is documented in [Native Scheme execution and Rust dispatch](hybrid-execution.md). Complete Scheme runtimes run on fixed owner OS threads. Rust handles host admission and cross-instance distribution; Scheme retains execution and green-thread scheduling.

## Retired paths

The Scheme-to-Rust IR workspace crate and CLI, the ledger's unused typed-fold Scheme producer, the VM-free ASCENT Python adapter, and the support crate's unused transport-only `rust-backend` feature have been removed. No compatibility aliases or fallback retain those execution paths. The ledger still defaults to its own `rust-backend` feature, which selects complete native Scheme instances.

Historical subset results do not qualify current Scheme behavior. The previous ordinary shared-runtime foreign-entry failure remains a historical failure; the private-image path is distinct and does not establish shared-runtime SMP support.

## Current qualification

The native ledger exercises four owner runtimes, real hygienic macro expansion, exact native types, closures, multi-shot continuations, GC, exception recovery, state affinity and retirement. Its original deadline is an observation boundary, not cancellation. Runtime-file removal and zero roots do not prove full RSS return or indefinite leak freedom.

The [owner-resident payload baseline](30-39-status/30.19-owner-resident-payload-baseline.org) records the latest 10.10x matched observation. The [scalar projection follow-up](30-39-status/30.20-scalar-projection-experiment.org) retains the negative endpoint conclusion. No projection experiment changed the public API or defaults.

Keep Gerbil/Gambit/C compilation and native types. Evaluate future changes using same-VM matched controls, throughput, p99 and finite fairness observations. Portability, hot unloading and native crash containment remain separate gates. Raw diagnostic artifacts are archived outside this repository; only maintained summaries and reproducible tests are versioned.
