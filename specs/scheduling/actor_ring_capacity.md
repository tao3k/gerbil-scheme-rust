# Standalone Gerbil Actor ring capacity

This experiment isolates one VM and one processor. Rust does not initialize,
schedule, feed, or consume Actor work. The existing external resource guard only
supervises the child process group. The library engine, SDK, defaults, timer
protocol, and frozen cache-B images are unchanged.

## Workload and ownership

The local upstream Gambit `examples/ring/ring.scm` accepts up to one million
threads. Its default total transfer count is approximately one million; it is
not an open-arrival business workload. This experiment uses its ring topology
with the installed Gerbil standard Actor API and an explicit work-message count.
The cached upstream source revision is not the installed SDK source revision.

Gerbil `spawn` calls `spawn-actor`, constructs an Actor thread, and starts it.
Each worker caches one `LocalActor` successor. All message exchange uses public
`->` and `<-` with a protocol-tagged Envelope. There is no four-worker pool,
three-frame limit, cross-VM admission, or per-message clock/FFI observation.

The source creates all Actors, sends their successor peers, and receives all
ready acknowledgements before timing work. One token circulates through the
ring: this measures a million resident lightweight Actors, not a million
simultaneously runnable Actors. A work count of M includes the source seed and
M-1 successor transfers. The final worker sends a completion acknowledgement.
The work interval includes observing that acknowledgement.

The source then sends stop to every Actor and joins every thread. Each returned
count must equal its independently calculated ring-position count; their sum
must equal M. Thread and peer vector entries are cleared before explicit GC.
Creation, bootstrap/ready, completion, and stop messages are excluded from work
throughput. There is no business payload construction or projection.

## Recorded run

Installed SDK: Gerbil `1cfb032c7a1612637da6205f5f8a15683eba3ba0`,
Gambit `ea114fc3d2f120abbe20393c1f9aeafdd3f8c89f`, patchset
`618d76f9eb725e27ce13194514ce5b8ca5489038`.
Workload C objects were compiled with GCC 16 and `-O2`. Apple clang was used as
the final link driver. Runtime `-:p1` is checked by the workload before creation.
The executable SHA-256 is
`34fa49dc616787b92c738152076789f6753a5fd468506c1edab981cdd2d448b4`.
The exact structured phase events and resource receipt are retained in
[`results.json`](checkpoints/actor-ring-20261031/results.json).

| Metric | Result |
| --- | ---: |
| VM processors | 1 |
| Created Actors | 1,000,000 |
| Validated work messages | 5,000,000 |
| Actor creation wall / process CPU | 1.972927 / 1.970107 s |
| Bootstrap and readiness wall / process CPU | 1.992611 / 1.988804 s |
| Work wall / process CPU | 1.110191 / 1.108192 s |
| Work throughput | 4,503,729.54 messages/s |
| Stop, join and count validation wall | 0.356130 s |
| Final explicit GC wall | 0.001198 s |
| Sampled process-tree peak RSS | 6,123,831,296 bytes (5.70 GiB) |
| Resource stop limit | 8,589,934,592 bytes (8 GiB) |
| Natural process exit | 0 |
| Count or join validation failures | 0 |

A preceding seven-Actor, 31-message correctness smoke passed the same per-Actor
count, total-count, join and GC checks. Its duration is too short for a capacity
claim. The full build's sampled process-tree peak was 783,548,416 bytes; the
full compile and link exited successfully.

The published build driver was subsequently checked with an empty output prefix.
It built successfully at a sampled 781,074,432-byte process-tree peak and passed
the seven-Actor/31-message correctness case. That separate executable hash is
`fff28cd620a5a3bd686fb0cad0caed218de975bcf58c626507266b478af38268`;
the capacity result above belongs only to the original executable. The capacity
experiment was not repeated or rebound to the recipe-check executable.

This is one observed capacity run, not a repeatability or long-lived memory
qualification. The guard samples process groups every 50 ms, so its peak is a
sampled value, not an exact allocator peak or a kernel-enforced allocation cap.
The final GC duration does not prove all process memory returned to the OS.

No admission/rejection policy or arrival schedule exists in this closed ring.
All five million work messages were validated; an open-arrival rejection rate,
backlog percentile, and request p99/p99.9 are **not measured**. No per-hop latency
histogram was added. This result does not replace the cache-B 0.350 ms library
reference, its 128-backlog gate, or business API acceptance.

## Reproduction

Use an empty writable output prefix, and export `GERBIL_PATH` to that prefix.
Set `GERBIL_HOME` and `GERBIL_GSC` to the installed SDK. For a relocated SDK,
`GAMBOPT="~~=$GERBIL_HOME"` supplies Gambit's runtime directory mapping.
The installed SDK in this run also needed the current OpenSSL library directory
in `LIBRARY_PATH`, because its recorded versioned linker directory no longer
existed. `GERBIL_GCC=/usr/bin/clang` selected only the final link driver; SDK
`gambuild-C` still compiled workload objects using GCC 16. No SDK file was edited.

From the repository root, invoke the standard Gerbil compiler driver:

```sh
"$GERBIL_HOME/bin/gxi" \
  crates/gerbil-scheme-qualification/scheme/actor-ring-build.ss \
  crates/gerbil-scheme-qualification/scheme/actor-ring-bench.ss "$GERBIL_PATH"
"$GERBIL_PATH/actor-ring" -:p1 7 31
"$GERBIL_PATH/actor-ring" -:p1 1000000 5000000
```

Use an external process-tree supervisor with the unchanged 8 GiB stop budget
for build and execution. The recording used `guarded_build 8589934592 --` from
the existing resource-safety work; that external supervisor is not shipped by
this benchmark-only checkpoint. Its Rust implementation is resource supervision
only; it has no access to the Actor protocol. Retain phase JSON output, guard output,
compiler/link output, and the executable hash together. Keep failed build
attempts separate from the final successful compile receipt.

## Architectural consequence

Single-VM measurements do not need the library's cross-instance publication,
capacity, or VM lifecycle coordination. The measured standard Actor path is
already capable of millions of lightweight transfers per second in this ring.
Future twelve-VM work should preserve this owner-local execution and measure
cross-instance admission and completion overhead separately. Actor population
is a workload parameter with substantial memory cost, not an automatic way to
increase single-core CPU capacity. This experiment introduces no new scheduling
algorithm or change to the existing QNT safety protocols.
