# Source-owned timestamp checkpoint: 0.317 ms p99

The user authorized publishing this research checkpoint despite its incomplete
outstanding-window admission. Operation 62 is the explicit continuation reference
for further source-debt work. Original operation 56 at `6b35864` (0.391 ms) remains
the historical reference. Runtime/benchmark defaults, queue bounds and deadlines
are unchanged; this is not a production promotion or PR #18 merge.

| Metric | Historical original | This run's original control | Source-plan arm |
| --- | ---: | ---: | ---: |
| Completed | 5,000,000 | 5,000,000 | 5,000,000 |
| Successful messages/s | 500,000.02 | 500,000.02 | 500,000.02 |
| Rejection / native error / accepted timeout | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| p99 | 0.391 ms | **23.044 ms** | **0.317 ms** |
| p99.9 | 0.617 ms | 35.505 ms | 0.431 ms |
| Peak pending source ordinals | 446 | 18,232 | 349 |
| Peak due-but-unconsumed | 448 | 18,234 | **351** |

The source-plan arm passes the numerical latency and zero-rejection conditions
but **351 exceeds the unchanged 128 outstanding guard**. Both measured arms'
failed guard receipts are preserved in `results.json`. The 23.044 ms control is
unexplained and cannot establish a 72x speedup; do not replace either frozen
reference with it or attribute it to OS/GC without correlated evidence.

The pair uses one live VM/fixed owner, four Gerbil workers, three active frames,
one executable/image and 500k original arrivals/s for ten seconds per arm.
Warmups are separate. Every message still constructs fresh native order input,
executes the full four-line handler and checks the independent result oracle.
Rust enters once per arm. Throughput is offered-rate throughput, not a measured
capacity ceiling. Original source timestamps, all clock reads and five-second
request/progress checks remain; there is no retiming or queue expansion.

## Replay through the existing Cargo bench

Use the repository's paired aarch64 macOS Gerbil/Gambit SDK and normal release
profile, retaining native `-O2`. For the exact original/control pair:

```console
GERBIL_BUILD_CORES=4 GERBIL_DYNAMIC_MODULE_OPT_LEVEL=2 CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 GERBIL_SCHEME_RUST_CHECK_GENERATED_SCM=1 cargo bench -p gerbil-scheme-qualification --features arrival-clock --bench gerbil_actor_arrival --locked -- --backpressure-schedule-compare --rate=500000 --seconds=10
```

Explicit source-plan-only replay uses `--backpressure-owned-schedule` instead of
the comparison flag. The native exact-schedule contract runs before both arms'
warmups; no candidate-only GC is inserted. No new performance run was performed
for publication. The recorded binary/image hashes remain separate from the new
source REV; SDK/toolchain changes do not produce an identical experiment merely
because the source commit is the same.

The measured source manifest is
`664b492eb87003034ede5246ddf9e60479fdfa6aeb580274ce3f780b3a9e21dc`;
executable SHA-256 is
`033d269c61895c79b2d1625d1c380c1dfe3d14d9981fe6a99a6f839088746f76`;
image SHA-256 is
`de30430f474106c3e8691e9e5bb99d62877d53ea0f1778f1c1843ba4d924e25b`.
`checkpoint-inputs.sha256` identifies changed build/runtime inputs plus root
metadata relative to `6b35864`. Reports/results were added after measurement.
Raw logs, traces, patches and binaries stay outside Git.

The native build, actual plan contract, two 1,000-message reuse/cleanup checks,
11 QNT scenarios, 1,000 traces of 80 steps and strict release Clippy passed.
These are scoped local checks, not full-runtime/remote-CI admission. The
[protocol and complete negative record](../../native_source_plan.md) retain the
initial model-scenario correction and the control anomaly. Further optimization
must reduce debt while preserving the exact original schedule and Scheme
execution capability; this checkpoint does not resolve owner OS availability.
