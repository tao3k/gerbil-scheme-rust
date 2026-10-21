# Shared capacity and explicit Scheme batches

Run from the repository root using the existing SDK and Cargo build:

```text
just bridge-owner-capacity-benchmark --test
just bridge-owner-capacity-benchmark
```

This focused bench retains four identical native Scheme VM states and two Tokio
workers. The current AB/BA pair fixes request slots at four, native batch size
and declared item budget at eight. It compares per-batch caller replenishment
with an explicit producer registered on the native owner. CPU and mixed cooperative
Scheme waits use the same 48-line invoice corpus. This adds no thread/VM matrix
and changes no production defaults. Historical request-capacity and batch-size
pairs remain in report 30.28; work-budget pairs and CPU regressions remain in 30.29.

Each full sample offers 8,192 independent bulk orders and 64 four-line short
probes scheduled every 500 microseconds. Smoke uses 256 and 8. Probes are one
closed-loop producer: a delayed response postpones its next submission, retaining
the original schedule for latency. Wake lateness is reported separately. This is
not an independent open-loop arrivals test. The bulk producer uses a 128-request
invocation credit bound and immutable owner assignment. It can have only one
pending capacity reservation per owner; existing accepted work retains its shared
credit through real native completion. Two Rust tasks represent the producers,
never one task per native request; only the fixed native owners execute Scheme.

Bulk inputs are generated outside timing. Probe fixture construction and cloning
are included in the probe entry/admission span. Bulk payload cloning, admission,
native type construction, Scheme execution, projection and independent Rust
validation remain inside elapsed timing. Explicit Scheme Actor batches preserve their documented
whole-batch exception/drain and cooperative quantum semantics. The numeric batch
is an existing specialized native API; the generic backpressured native operation
retains arbitrary Scheme types and procedures. No arbitrary calls are implicitly
fused. Each owner verifies a real eight-item batch error and subsequent recovery.

Warm each pair arm on the same 256-order native workload before measurement. Five
forward/reverse blocks provide ten samples per arm, one block in smoke. The fixed
five-second startup/progress/observation gates remain. Reconfigure only between
samples once every shared credit has returned; pool state is never reinitialized.

`capacity_sample` retains complete per-owner bulk native offsets and individual
probe native/delivery/admission/wake spans. `capacity_result` gives complete and
bulk-only rates, probe p99, bulk owner progress at first owner completion, probe
completions during bulk execution and bulk completions by the final probe. Faster
bulk completion can reduce overlap; inspect that count before claiming a
steady-state p99 improvement. Bulk batch members share completion timestamps.
Actor peer progress applies to batch 8 and is null for batch 1. Finite progress,
semaphore FIFO and fixed probe totals do not prove starvation freedom or arbitrary
cross-caller fairness. Probe admission spans include host validation as well as
capacity wait; they are not pure mutex or GC estimates.

Retain every failure and negative pair. Store raw captures outside Git in a
caller-selected directory using relative evidence names. Follow the existing
[portable capture instructions](../owner_dispatch/README.md#portable-evidence-capture)
for Git/source, executable/native-image SHA-256, command, exit and host-load
identities. JSON Lines are result records, not the Scheme payload protocol.


Request slots and item credits are independent. All native numeric batch API
variants charge their item count; byte and generic native calls charge one unless
the application explicitly declares a weight. Acquire work credits before a slot,
under one finite admission deadline. Cancellation returns partial reservations;
accepted work holds both resources through native completion and rollback. FIFO
multi-permit acquisition can make small requests wait behind a larger reservation;
a smaller work budget is not a priority scheduler or a starvation theorem.

Both arms use the same opt-in profiled batch API. Each batch records
`dispatched`, `entered`, `constructed`, `executed`, `projected`, native completion,
and GC counter deltas. `bulk_service_ns` and `probe_service_ns` measure owner entry
to native completion, including construction, Scheme/GC, projection and root
retirement, excluding final result delivery/credit return. Queue-and-admission
spans run from API entry to owner entry and include host validation. Scheme spans
include diagnostics between markers; they are wall-clock spans, not pure CPU cost.
The profiling overhead is present in both arms, so these rates must not be compared
directly to the historical unprofiled executable. Raw per-call spans stay in the
external sample captures; summary p99 fields are derived from the same rows.

Whole native turns remain non-preemptible to the Rust owner queue. Scheme Actor
yields run other Scheme threads but cannot service another Rust entry mid-call.
Both arms permit at most eight reserved/accepted declared items per owner and
rejects a larger indivisible batch; it does not silently split a batch or weaken
exception/drain semantics. Eight bounded handlers imply a finite service boundary
only if the application independently bounds each handler, GC and host scheduling.
Arbitrary Scheme retains its complete capability and has no automatic wall-clock
bound. Observed service p99 is a measurement, not an enforced deadline.


The producer arm stages at most 128 original batches per cursor, bounded by this
pool's native queue capacity. One registered producer per owner holds a separate
metadata permit; each actual turn acquires the same work/slot credits. Staged Rust
input is unadmitted work. Aggregate input/output bytes use existing transfer limits.
For 8,192 orders the control makes 1,024 batch admissions; the producer makes eight
cursor registrations (two per owner), still executing all 1,024 original Scheme
calls with unchanged item counts/quantum. No batch splitting or fusion occurs.
Both arms clone payloads inside the sample; producer grouping/reindexing and final
row collection/independent oracle are included in elapsed timing.

`owner_producer` identifies the arm. Per-turn bulk dispatch markers start at the
first credit attempt, after any caller/cursor backlog waiting; those markers are
not total bulk latency from sample release. Full native offsets retain release-to-
completion clocks. Probe APIs and clocks are identical across arms. Common-window
outside-service gaps are reconstructed from every bulk/probe entry/completion pair,
excluding probe-only tails after bulk completion. Gaps include delivery, credit
return, inbox/handoff and host scheduling; they are not pure Tokio CPU time.

Report 30.30 preserves an initial full-run startup watchdog failure with no measured
samples and `cleanup=UNVERIFIED`, plus separately sampled current/predecessor smoke
controls. Do not overwrite or relabel that failure when collecting successful
initialized-VM cohorts. It is not evidence that the startup defect was repaired.
