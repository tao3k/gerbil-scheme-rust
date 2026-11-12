# Scheme-owned native mailbox observation

This diagnostic investigates transient debt in the existing 500k/s native
backpressure experiment. It does not change supply, execution, verification,
capacity return, public defaults or the five-second bounds. Its throughput is
not a performance acceptance result. The original unobserved frozen artifacts
and the historical 0.418 ms p99 remain the acceptance reference.

## Ownership and native types

`instance-mailbox-observation.ss` contains the raw C reads inside `begin-foreign`
and exposes named primitives using `c-lambda`. C directly reads monotonic time,
current owner-thread CPU time and the current VM's GC counters. It has no stage
catalog, stage counters, shared snapshot, report arrays or manually synchronized
record widths. Scheme stores each named reading in a private typed record;
timestamp brackets expose acquisition time, including green-thread preemption.
C retains no Scheme pointer, GC root or mutable reading shared by green threads.

The protocol declares its lexical stages once using `with-mailbox-observation`.
Each green thread owns its own stages and sampling state. The source and each
existing worker have separate lexical contexts; the parent registers children
before starting them, then joins workers before reading child reports. Each
stage owns a count, accumulated interval wall time and a preallocated maximum
interval record. The observation stores only request ids and numeric samples;
it cannot keep a payload/result alive or return its frame. Sample records are
captured at the existing admission window's service duration (`window / rate`).
Both inputs come from the existing benchmark contract; no sampling-threshold
configuration matrix or new scheduling policy is introduced.

The CPU/GC window is a pair of raw snapshots, not exact per-phase CPU execution.
Their timestamp brackets are included so readers can check whether a long
interval is enclosed before interpreting CPU and GC deltas. New sampling must
finish before the following interval begins. Sampling overhead remains in
planned-to-consumed request latency even where excluded from phase intervals.
Other hook overhead may remain in the intervals. Phase totals are diagnostic
attribution, not independent additive p99 measurements.

The unobserved and observed paths are generated from one protocol definition.
Literal false observation branches disappear from the unobserved expansion;
there is no per-message callback or runtime observation branch in that path.
The observed result is a native vector containing the ordinary numeric receipt
and a native table of named stage reports. Reports contain native string keys,
tables and scalar values. The Rust launcher uses the existing `NativeSession`
object interfaces to walk those types after the experiment ends. It neither
knows the stage count nor decodes fields by fixed column positions. JSON is only
the launcher's final output format; no JSON enters or crosses the VM.

## Stage meaning

Source bookkeeping, construction/publication, receive dispatch, consumption and
sparse-source idle belong to the caller context. Business, independent
verification, reply handoff and worker idle belong to each worker's context.
Intervals in different contexts can overlap. A stage's wall interval can include
other green threads executing on the same native owner, GC, blocking or time
without CPU. The CPU counter measures the shared OS thread, not a green thread's
exclusive execution. Inspect raw brackets before attributing a pause. Off-CPU
time alone does not distinguish runnable delay from an unobserved native block.

Before the measured clock, the native read contract forces GC and checks clock
brackets, positive/monotonic owner CPU, increasing GC count and monotonic GC time.
The existing guarded-access edge contract also remains. The QNT refinement
models observation as a stutter: it cannot verify an unverified result, return
a live frame or advance completion progress. Its new scenario supplements the
existing debt and premature-retirement negative controls. Formal checks do not
prove observational overhead or reproduce the previous 95.966 ms pause.

Use the existing launcher with `--backpressure-observed --rate=500000
--seconds=10`. The path is explicitly diagnostic and remains limited to 500k/s.
Keep its raw outcome and any failure beside the frozen source/image identity.
Do not replace the unobserved acceptance benchmark with this diagnostic.

## Retained diagnostic failure and ownership correction

The first Scheme-owned diagnostic shared one stage context across green threads
and used an OS-thread-local C reading slot. It returned inconsistent stage counts
(for example 5,000,034 business intervals for 5,000,000 requests). Those stage
attributions are rejected. A green switch can interrupt the read/update sequence;
OS-thread-local storage is not green-thread-local storage. That run and its frozen
source remain under `target/replay-native-mailbox-20261027/scheme-owned-observation`.

The corrected implementation gives every green thread private sampling and stage
records. C readings have no shared cache. No mutex or interrupt suppression is
introduced. Workers join before reports are read. Native acceptance checks require
exactly one business, verification, construction and consumption interval per
completed request. The QNT observation refinement retains a shared-counter
lost-update mutation and tests private ownership under the same interleaving.
The base model also explicitly demonstrates that zero drops plus resource safety
can coexist with excessive source debt; bounded debt is a separate requirement.

## Corrected 500k/s diagnostic on 2026-10-27

The native clock/forced-GC contract passed, all 5,000,000 messages completed, and
the four main interval counts matched exactly. Each worker executed and verified
1,250,000 requests. Shutdown was clean, with no accepted timeout or native error.

| Captured interval | Wall time | Paired CPU window | VM GC count change |
| --- | ---: | ---: | ---: |
| Source construction/publication, request 2,174,718 | 57.767625 ms | 0.337792 ms CPU over 57.955333 ms | 0 |
| Source receive/dispatch, request 3,714,978 | 49.228542 ms | 0.377834 ms CPU over 49.419500 ms | 0 |
| Worker 0 business, request 3,714,976 | 32.878834 ms | 0.174667 ms CPU over 32.880500 ms | 0 |

The construction interval's CPU sample begins 0.182833 ms before its stage
interval and ends just after it; workers 0, 2 and 3 have overlapping idle maxima
of roughly 57.8 ms. The long wall intervals therefore cannot be treated as CPU
spent aggregating orders or as GC pauses. Most of that time is outside owner
thread CPU execution. This does not distinguish runnable scheduling delay from
another native block, and does not causally reproduce the original 95.966 ms gap.

The original 47,983-message pending-source peak and 95.966 ms publication lag are
consistent with 500,000/s times that lag. Those messages were unmaterialized
source debt, not 47,983 payloads inside worker mailboxes. Returning frame credits
or enlarging a mailbox does not remove a source/owner progress interruption.
This identifies where the debt is represented, not the still-unresolved blocker.

Full observation is materially intrusive: completion throughput was 348,972.15/s,
elapsed time 14.327791 s, p99 4,294.859 ms, and peak due unconsumed work 1,493,971.
The diagnostic's own bookkeeping contributes to sustained debt. It is neither a
replacement baseline nor proof that the unobserved engine regressed to this
level. These diagnostic numbers cannot validate an unobserved performance
improvement. The original 500k/s zero-rejection and 0.418 ms p99 gate remains unmet.

Frozen directory: `target/replay-native-mailbox-20261027/private-green-observation`.
Source manifest SHA-256:
`9de46ca771e49dfa9efac7c41f58ff33d661edea42c1cb158e2aa6169ee5727f`.
Binary SHA-256:
`442a2fcab80da47914e173521a32776e20439e13bc08189d790f627d5d82fa2b`.
Image SHA-256:
`31a8030112f6ab9518269d5eb0e6437805ccfd374a79f90e79d08edc12011bf0`.
Raw records, failed shared-context run, native contracts, formal checks and strict
release bench Clippy are retained locally. No production Rust library source,
public default, service deadline or optimization level was changed. No commit,
push or million-rate experiment followed. The native waiting/runnable boundary
still needs to be resolved before a protocol fix or performance claim.
