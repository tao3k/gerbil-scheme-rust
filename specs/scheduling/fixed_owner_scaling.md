# Same-workload fixed-owner scaling

This experiment starts at `267ebdc` on `research/single-vm-service-cost`.
The unchanged-engine control retains the `fc82d5a` engine. It measures a
missing comparison: the same short-CPU messages on one and twelve owners.
It does not replace the published single-VM cohort or the historical mixed
open-arrival 10.141/13.039 ms records.

## Contract

Run `just bridge-fixed-owner-scaling 1` or `just bridge-fixed-owner-scaling 12`.
Each case checks exactly **1,000,000 total messages**, one native order with
four records per message, using the same 2,048-message corpus, native types,
public pinned-owner backpressured API and independent Rust oracle. Thirty-two
warmup messages per owner are excluded. No per-message task is spawned.

One resident Tokio task per owner starts behind a common ready barrier.
Tokio worker count equals owner count; each owner has one outstanding request.
Global indices are partitioned by stride, preserving the global corpus union.
Twelve-owner counts are 83,333 or 83,334. Global quantiles pool every request,
not owner percentile summaries. Throughput is total completions divided by
barrier release to the last actual client completion, including oracle work.
Owner completion timestamps are recorded inside each task, before parent joins.

The five-second request/no-progress limits and 300-second capture limit remain.
Actual owner-initialized events refresh startup progress; they are not synthetic
heartbeats during a hung initialization. All owners must join, all tracked
roots must reach zero and copied images must be removed.

This closed-loop comparison measures scaling at one live request per owner.
Zero rejection is not production open-arrival admission success. Fixed equal
assignment constructs count balance; completion spread is an observation of
service balance, not a general dynamic-routing fairness proof. The M2 Pro has
heterogeneous cores, and the process also runs Tokio/timer/watchdog threads.
Linear 12x speedup is a reference, not an assumed attainable requirement.

## Unchanged-engine control

| Metric | 1 owner | 12 owners |
| --- | ---: | ---: |
| Completed messages | 1,000,000 | 1,000,000 |
| Messages/s | 35,083.21 | 70,829.74 |
| Timed loop wall, s | 28.503661 | 14.118363 |
| Whole-cohort p99, ms | 0.373500 | 0.810125 |
| Whole-cohort p99.9, ms | 0.420583 | 1.598666 |
| Maximum, ms | 2.028333 | 61.362667 |
| Sum of owner-thread CPU, s | 27.651456 | 66.827438 |
| Owner CPU per message, microseconds | 27.651456 | 66.827438 |
| Owner completion spread, ms | 0 | 112.134958 |
| Rejected / native errors | 0 / 0 | 0 / 0 |
| Joined owners / tracked roots after | 1 / 0 | 12 / 0 |

Observed throughput scaling is 2.0189x, while owner CPU per message rises
2.42x. This is a real scaling deficit in this contract, not evidence that the
mixed-load historical p99 increased one hundredfold on identical work.

| Marginal phase p99, microseconds | 1 owner | 12 owners |
| --- | ---: | ---: |
| Client start to constructor | 23.417 | 673.583 |
| Native construction | 2.125 | 13.333 |
| Constructed to projection | 346.792 | 503.959 |
| Projection | 0.875 | 1.875 |
| Projection to client consumption | 18.291 | 60.792 |

These are different request populations and cannot be added. Entry covers
more than a registry lock; no lock-wait attribution is established by these
marginals. Owner CPU is the bridge's thread CPU clock. GC wall intervals can
overlap across VMs. SDK GC user/system counters must not be summed as exclusive
owner GC CPU without verifying their native scope.

Whole-process controls were 29.21/22.09 seconds wall, 27.29/56.04 seconds user,
4.32/23.62 seconds system, and 281,853,952/554,827,776 bytes maximum RSS for
one/twelve owners respectively. Setup and output sorting are outside timed
loop scope. These are not lifetime RSS leak measurements.

The published 28,008-message/s single-VM observation belongs to an earlier
cohort driver. The new 35,083 control is not an engine improvement claim.

## Source-grounded first candidate

`instance_backend.rs` serializes every readiness read through one process-wide
mutex, including native turns and submission checks across independent VMs.
Mode is a startup choice, followed only by permanent poison. The candidate
uses an acquire atomic readiness load, a compare-exchange for first selection,
and a release poison store. Same-mode claims remain idempotent, conflicting
claims fail, and no claim overwrites poison. Readiness grants no root, image
lifetime, native-entry or owner-routing authority. A native call already past
the check can still finish while poison is published, as under the old mutex.
The original mutex was correct; this is a hot-read serialization hypothesis.

`backend_choice.qnt` adds only this missing shared primitive. Existing owner,
Actor, continuation and cancellation protocols remain the lifecycle authority.
Six directed cases include unsafe check/store negative controls; 1,000 traces
of at most 50 steps check exclusive choice and sticky poison. The negative
controls describe an unsafe proposed atomic rewrite, not a defect in the old
mutex. Actual-source Rust tests cover conflicting claims and claim/poison races.

The public submission also visits a shared registry for validation, capacity
capture, repeated validation and final enqueue. Those close/reconfiguration
fences remain intact. Coalescing them requires preserving the final publication
fence and detecting stale capacity generations; simply removing them is unsafe.

No default, Scheme code, native image, workload or threshold changes. Candidate
acceptance is separate from these correctness checks and from production gates.

## Frozen identities

| Artifact | SHA-256 |
| --- | --- |
| Control executable | `d85f9dd3fd1835cebef610d5e7e7f269310155194fa89fcf3030a165cb226396` |
| Candidate executable | `3c03ffd0882587761894ecb2d17b51257e53ca988674c5feab8aef830797f796` |
| Shared native image | `271d52efd66b04999fbbbcca0c9d49cd5cc2d55e0d5cbd23b763dbaa40cb24ec` |
| Parent benchmark source, both executables | `80d14ba407b2ea62386ee15f3fe651c4307c1b4c5bdf1ea732f68d09caf28360` |
| Scaling module, both executables | `f35362c81ee48faeeb86311855f1aa3d48c4abd6212bc6a39cf9d269ed9bd7fa` |
| Candidate backend source | `2d7af1d2af75c30b7e3fb2864a5f2d744d222c148c5363f0b22d34e3fe76873c` |

The executable's embedded image path was checked against its Cargo build
output before freezing. Image bytes match the control. Local captures and
executables remain in ignored `target/` directories; no private path is required
by the recipes or recorded here. Host/toolchain are unchanged from the baseline
README. External activity was not paused; these sequential observations alone
do not establish variance or a causal percentage speedup.

## Candidate outcome: rejected, engine restored

| Metric | Control 1 owner | Candidate 1 owner | Control 12 owners | Candidate 12 owners |
| --- | ---: | ---: | ---: | ---: |
| Messages/s | 35,083.21 | 27,667.12 | 70,829.74 | 61,886.69 |
| p99, ms | 0.373500 | 0.372750 | 0.810125 | 1.247333 |
| p99.9, ms | 0.420583 | 0.579458 | 1.598666 | 4.915000 |
| Owner completion spread, ms | 0 | 0 | 112.134958 | 448.163333 |
| Owner-thread CPU sum, s | 27.651456 | 33.287731 | 66.827438 | 57.789296 |
| Completed messages | 1,000,000 | 1,000,000 | 1,000,000 | 1,000,000 |
| Rejected / native errors | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |

All four runs drained roots, joined the expected owners and removed runtime
image files. The candidate passed four actual-source Rust tests, the original
three-arm smoke and strict benchmark Clippy before measurement. Those checks
are correctness evidence, not performance acceptance. Its native image and
benchmark source hashes matched the control exactly.

The candidate is **not accepted**: twelve-owner throughput fell 12.63%, p99
rose 53.97%, and completion spread increased about fourfold. The single-owner
throughput also fell 21.14%. Sequential runs with concurrent external activity
cannot prove the atomic change caused those percentages; they nevertheless
provide no basis to promote it. No machine-competition explanation is asserted.
No additional runs are used to search for a favorable outcome.

The production backend source has been restored exactly to `267ebdc`; temporary
candidate-only Rust tests were removed with it. The rejected source, test
sources, executable and raw observations remain in the ignored local capture.
The small QNT module remains as a research contract for a future shared-mode
implementation, not a claim that atomic mode selection is now installed.

The next architectural question is how to reduce shared coordination at the
public boundary while preserving close and capacity-generation validity. A
validated capacity snapshot could replace repeated registry visits only if
final publication still checks the live owner and the same capacity generation.
After native entry, values, roots and continuation state remain fixed-owner
resources. Before entry, only owned Rust input is movable. This requires a
refinement of the existing publication/cancellation protocol, not a new lease
layer or an unguarded cached sender. Backend atomic reads alone did not
establish a solution to the measured scaling deficit.
