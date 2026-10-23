# Python, Rust dispatch and complete Scheme instances

## Current execution contract

The default ledger Rust backend opens four native Scheme instances on four fixed OS owner threads, each with one active Scheme processor. Tokio and Rayon workers submit jobs across those instances; Scheme executes the application and schedules its own Actors and green threads. The native Gerbil/Gambit/C compilation path is retained.

The former VM-free typed-fold and ASCENT adapters and the Scheme-to-Rust IR compiler are removed. The ledger's `rust-backend` feature selects `gerbil-scheme-python/instance-backend`. The support crate has no transport-only `rust-backend` feature or subset fallback.

Sessions share their owner's Scheme state and preserve affinity. Stateless routing rejects native references. Retained graphs, closures and continuations stay rooted on their original VM; host copies use native typed projection. Projection failure preserves opaque values, and timeout observes accepted work without forcibly cancelling it.

## Qualification and limits

Maintained installed tests exercise distinct VM identities, independent module images, native values, exact integers, shared/cyclic graphs, GC, multi-shot continuations, exceptions, all dispatcher controls, retained values at shutdown and resource-file cleanup. Zero roots and removed files do not prove full RSS recovery. Same-process instances do not provide native crash containment. The private-image implementation remains experimental on Darwin.

The linked `scheme` provider is a distinct process-global one-owner control. Its historical shared-runtime foreign-entry experiment constructed two VM states, but both selector readbacks failed (`selected=0`, `outcome=13`), exiting 70 without timeout. Those later overlap/isolation checks were not reached. This failure remains a failure and is not evidence against every Gambit configuration; private runtime images use a different entry path.

See [the native API](../../docs/instance-runtime.org), [the adopted 10.10x payload baseline](../../docs/30-39-status/30.19-owner-resident-payload-baseline.org), and [the rejected projection experiment](../../docs/30-39-status/30.20-scalar-projection-experiment.org). Keep throughput, p99, finite fairness, source qualification, local wheel results and remote CI as distinct evidence.

## Reproduction

Build the ledger through its existing Cargo/maturin native O2 chain and install the wheel into a separate environment. Run:

```sh
just bridge-python-owner-tests /path/to/venv/bin/python
just bridge-python-owner-routing-tests /path/to/venv/bin/python
just bridge-python-owner-full /path/to/venv/bin/python
just bridge-python-native-payload-ab /path/to/venv/bin/python
```

Raw logs, source snapshots and rejected patches are local diagnostics kept outside the repository. Version maintained test programs, English summaries and reproduction commands.
