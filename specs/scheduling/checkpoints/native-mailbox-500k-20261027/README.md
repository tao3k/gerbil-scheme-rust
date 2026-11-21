# Native mailbox 500k/s research checkpoint

The original path is the reference for further optimization: **0.391 ms p99**
with zero rejection. The ready-edge candidate is retained only as an explicit
negative experiment. Its 6.09472% reduction in notifications did not improve
throughput and came with worse tail latency and source debt. No runtime defaults,
queue capacities, optimization level, deadlines or owner priority are changed.
This checkpoint is published to the existing research branch; it does not merge
or update PR #18's head branch.

## Fixed workload and results

Both arms use one live Scheme VM/fixed owner, four resident Gerbil workers,
three active frames, the same executable/image and original order. Each arm
schedules 5,000,000 messages at 500,000/s for ten seconds. Each message constructs
fresh native Scheme input for four order lines, executes the complete handler
and checks the independent 21-field result oracle. Rust enters once per arm and
does not schedule individual messages. This is a native Actor/message benchmark,
not HTTP RPS or empty-mailbox capacity. The 1k/s warmups are excluded below.

| Metric | Original | Ready-edge candidate |
| --- | ---: | ---: |
| Completed | 5,000,000 | 5,000,000 |
| Successful messages/s | 500,000.02 | 500,000.01 |
| Rejected / errors / accepted timeouts | 0 / 0 / 0 | 0 / 0 / 0 |
| Planned-to-consumed p99 | **0.391 ms** | 0.453 ms |
| p99.9 | 0.617 ms | 3.897 ms |
| Peak pending source ordinals | 446 | 3,506 |
| Peak due-but-unconsumed messages | 448 | 3,508 |
| Completion notifications | 5,000,000 (source-derived) | 4,695,264 (counted) |
| Final pool cleanup | Clean | Clean |

The historical 0.418 ms guard is retained. The original meets that guard and
zero rejection in this run, but both arms exceed the separate 128-message
due-but-unconsumed guard. Neither is declared fully performance-admitted.
Source ordinals are unmaterialized scheduled debt, not an enlarged payload
mailbox. The candidate is 15.86% worse in p99 than this control. No favorable
percentage is calculated against older slow/failed controls. These are offered
rate results, not a measurement of maximum sustainable capacity or a guarantee
that future runs reproduce the same quantiles.

## Rebuild and explicit replay

Use the repository's installed Gerbil/Gambit SDK and Rust dependencies on
aarch64 macOS. Keep C optimization at `-O2` and the workspace release profile
(Rust opt-level 3, thin LTO, overflow checks). The native SDK/compiler cohort
and frozen artifact identities remain separate from this Git source checkpoint.
The measured SDK reported Gambit `ea114fc 20260930210602`; do not silently
substitute another SDK and call the resulting image byte-identical.

```console
GERBIL_BUILD_CORES=4 GERBIL_DYNAMIC_MODULE_OPT_LEVEL=2 CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 GERBIL_SCHEME_RUST_CHECK_GENERATED_SCM=1 cargo bench -p gerbil-scheme-qualification --features arrival-clock --bench gerbil_actor_arrival --locked -- --backpressure-coalesced-compare --rate=500000 --seconds=10
```

The comparison explicitly selects operation 56 first and operation 60 second;
it does not promote operation 60. The native GC/CAS/stale-token contract runs
once through operation 61 before both arms' warmups. Keep five-second request
and progress deadlines. Original-only work uses `--backpressure-native-access`
instead of the comparison flag; candidate-only replay requires the explicit
`--backpressure-coalesced` flag. `--require-native-acceptance` fails after clean
shutdown if a single qualified arm misses the unchanged numerical guard.

No replay was run merely to create this Git revision. `results.json` contains
the complete structured events from the existing frozen run, including warmups,
guard failures and cleanup. Raw traces, compiler logs, patches and binaries are
not committed. The local immutable artifacts are identified by SHA-256:

- Executable: `d5c7b0f15bc7ed5062b99d669c43d84ed86810a69c2c72808bdd3abc408dba5a`.
- VM image: `83ba1f57e9b665c6b377b90238bac4719fbc91bed39cb60f427fc9233f3152bb`.
- Pre-publication full source manifest: `2e8f22c5db0beab0acc32ad889b0b1dc95116a3217eb6fb23972dcf5e140181c`.

`checkpoint-inputs.sha256` covers changed build/runtime inputs relative to
`974a941` plus root build metadata; unchanged files remain identified by that
Git base. It includes the validated dependency-tracking extraction in the build script.
That extraction preserves emitted directives; reports and normalized results
were added after measurement. This is not a claim the newly committed source
has already been benchmarked under a new executable hash.

## Validation and continuation boundary

The recorded native build, actual operation-61 contract and two 1,000-message
reuse/cleanup checks passed. Twelve QNT scenarios and 1,000 traces of 80 steps
passed. Strict release Clippy passed. These are scoped local checks; no full
test-suite, remote-CI, long-term memory or OS scheduling guarantee is inferred.

The measured cohort includes pre-existing native type/service support changes
relative to its old base `974a941`; they are frozen to preserve actual compiled
dependencies, not presented as newly accepted improvements. The source contains
optional older research modes whose failures are retained in the linked reports.
Only this original arm is the continuation reference, and candidate opt-in code
must not become the default or the baseline for a future optimization.

The original investigation checkout removed the rejected candidate after
measurement. This separate research checkpoint deliberately restores its exact
optional source for reproducibility. See the [protocol and negative evidence](../../native_ready_wake.md)
for the complete sequence, including the earlier compiler failure. Owner CPU
availability and bounded recovery remain open; PR #18 stays unmerged.
