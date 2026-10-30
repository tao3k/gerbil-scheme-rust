# Fixed-owner Actor replay: 0.402 ms reference

The user accepted the clean `19e164f` frozen-artifact replay below as a latency
reference for integration into the PR #18 branch. This accepts neither the
128-message debt gate nor general performance stability. The PR remains open.

## Fixed workload and results

One Scheme VM and fixed owner, four existing green workers, three transfer
frames, 500,000 planned messages/s for ten seconds: 5,000,000 messages. The
original same-VM warmup is 1,000 messages/s for one second. The five-second
watchdog, 128-debt guard, independent result oracle, input shape and native types
remain unchanged. Rust has no per-message entry in this workload.

| Cohort | Short p99 ms | p99.9 ms | Completed messages/s | Rejected | Peak due-minus-consumed |
| --- | ---: | ---: | ---: | ---: | ---: |
| Historical original artifact record | 0.298 | 1.634 | 500,000.03 | 0 | 1,415 |
| Earlier clean `19e164f` replay | 0.382 | 2.650 | 500,000.03 | 0 | 2,715 |
| Current clean `19e164f` frozen replay | **0.402** | **1.282** | **500,000.02** | **0** | **1,081** |
| Current historical-original frozen replay | 49.849 | 56.244 | 500,000.01 | 0 | 29,104 |
| Integrated PR tree, newly built matching artifacts | **0.350** | **0.484** | **500,000.02** | **0** | **1,142** |
| Cold-report extraction, reverted | 270.485 | 288.308 | 495,951.29 | 0 | 144,616 |

All four current runs complete all 5,000,000 messages, report zero request errors
and timeouts, and exit cleanly. Each artifact pair runs once, in the order shown
by the current rows, without extra profilers, QoS changes, stopping other jobs
or selecting the best of repeated attempts. The first two use retained frozen
pairs; the third and fourth use their own newly built matching pairs. The second negative
record is retained; it does not replace the clean source-bound reference.

Throughput is completed messages divided by elapsed time, including drain. This
is an open-arrival, rate-limited observation, not a saturation-capacity estimate
or HTTP RPS. Latency uses the original planned-arrival to source-consumption
stamp in 1 us buckets; the final frame-clear/counter tail follows that stamp.
No p50/p95, CPU utilization, long-term memory or multi-VM scaling result is
claimed by this replay. The histogram's original rank calculation is unchanged.

The current 0.402 ms value is under the existing 0.418 ms latency threshold.
Its 1,081 peak still fails 128. The selected scope also enables the original
owner diagnostic, so the full automated admission is false independently of
latency. All original failure markers remain in `results.json`. User acceptance
of this reference must not be rewritten as full performance acceptance.
The integrated tree also passes the original latency threshold at 0.350 ms,
but its 1,142 peak independently fails 128.

An attempted cold-report function extraction failed the unchanged latency and
debt gates. Its native image stayed identical, but its Rust binary differed;
one run does not establish causation. The extraction was reverted. A narrowly
scoped `expect(clippy::too_many_lines)` on the original frozen function preserves
the runtime implementation instead of changing it to satisfy a style limit.
It adds no runtime instruction and changes no other lint level. This is the
only local length exception; strict Clippy remains enabled elsewhere.

The restored normal-build executable and native image are byte-identical to
the pair already measured at 0.350 ms. They are copied and retained before
any further rebuild. No additional replay is relabeled as a success and no
negative run is discarded. `owner_recovery.qnt` retains the 270.485 ms / 144,616
counterexample without attributing the failure to the extraction or the host.

## Artifact and source identity

| Frozen pair | Benchmark SHA-256 | Image SHA-256 |
| --- | --- | --- |
| Clean `19e164f` | `7efdc6fe4fa9dc6a1257466939241bfabb0c28b211b55d13ddaf5d38e9dca3c1` | `938c1bf43acda30ddb7367adf09be7d18c99dc801ab536562dce836cb46e34f8` |
| Historical original | `2b016d1d934ed5de55041d095e0a71801c4aa0b9a61f1cc643a83f0bdf56f289` | `92eba95f7972033f4c9980344f0d62070ed38ea8f5c8e9513a1f9692141900d1` |

Both pairs' hashes are verified before and after execution. Clean source REV is
`19e164f5ac23681d1e2915bc4dea9489a7cb482a`. The historical pair's complete host
source identity is not independently proven; it must not masquerade as a clean
`19e164f` build. Its original 0.298 ms record remains a historical reference.

The native replay command is the existing Rust bench executable with
`--owner-wait-scope --rate=500000 --seconds=10`, selecting its paired image via
`GERBIL_SERVICE_BENCH_IMAGE`. That command includes the original cold source-plan
contract and same-VM warmup. No timer, wait or admission flag is changed.

To rebuild the source-bound reference from a clean `19e164f` checkout through
the existing compiler chain:

```text
cargo bench -p gerbil-scheme-qualification --features owner-wait-diagnostic --bench gerbil_actor_arrival --no-run --locked
```

Use the returned executable and its matching owner diagnostic image. A rebuild
is not asserted byte-identical to the retained pair; archive/check its SDK,
compiler, executable and image identities separately. Raw logs and native
artifacts are retained outside Git; this directory contains curated receipts
only, with no machine-specific absolute paths.

## Integration boundary

Integrate the fixed source without post-checkpoint runtime experiments. Preserve
the PR branch's atomic private-directory creation, borrowed element snapshots,
process-isolated qualification and CI fixes. Conflict resolution retains both
the existing test isolation wrapper and the native-case guard.

The 0.402 ms result is measured on the exact frozen `19e164f` pair. The separate
0.350 ms result belongs to the integrated runtime sources, identified by the source parents and the separately retained runtime-source
manifest, with its own executable and image hashes in
`results.json`. Both source parents are retained in the merge. Native compilation
uses the original `-O2 -fno-gcse-lm` options; the successful native build took
5 min 53 s. Initial dependency discovery required the matching installed
pkg-config search paths; no SDK or optimization setting was changed.

Do not merge the PR into main or declare the 128-debt problem solved from this
checkpoint. A passing local replay is not an exact-head remote CI receipt.

Local validation includes formatting, core and qualification strict Clippy,
17 C/report cases, the actual VM scope GC/exception/continuation case, and the
complete native service contract with root/FD return, plus the focused scope
and frame model checks. The VM case initially hit a sandbox standard-port
restriction; it passed in the same permission context as the benchmark with
its original five-second gate. Build/lint launcher failures are retained
locally, separately from runtime/performance results.
