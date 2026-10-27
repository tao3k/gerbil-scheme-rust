# Native Gerbil mailbox: supply, execution and verified consumption

## Current acceptance order

First require zero rejection at 500,000 planned messages/second, preserve the
historical 476,775.66 completions/second and 0.418 ms successful p99, and expose
source debt instead of hiding it outside the admission window. Only after this
gate passes may 1,000,000/s testing resume. No public defaults or Rust per-message
integration change. The earlier high-rejection results remain in
[native_mailbox_evidence.md](native_mailbox_evidence.md).

## Application protocol

Gerbil's installed `std/actor/interaction.ss` sends to a local Actor with
`thread-send` and uses the existing green-thread mailbox machinery. The
qualification protocol uses these existing primitives and four resident workers;
it adds no executor, timer, scheduler Actor or Runtime/SMP modification.

| Phase | Owner | Work and retained data | Transfer boundary |
| --- | --- | --- | --- |
| Scheduled source | Source/caller | Original rate, start time and next ordinal; no retained payload for an unadmitted arrival | A free frame permits construction |
| Construction/publication | Caller | Fresh original four-record input and tag in one of three frames | `thread-send` hands frame custody to its worker |
| Business execution | Existing worker | Unchanged `orders-service-handler`; generic checked kernel, original allocation and suspension facilities | Result stays in that worker-owned frame |
| Independent verification | Same worker turn | Unchanged full `actor-arrival-verify!`, not an echo or cached answer | Immutable application-id response plus frame sent to caller |
| Verified consumption | Caller | Deadline, original planned-to-consumed histogram and verified progress publication | Clear result references, then return frame and active credit together |

Only three transfer frames exist. A response carries an immutable application id
in addition to the mutable frame identity; the receiver checks slot identity,
generation and ready state before consumption. The worker never mutates a frame
after sending its response. The caller cannot recycle it until verified
consumption. A result is not dropped merely because its business execution ended.
Normal/exceptional unwinding stops and joins all workers before clearing unread
results and frame references. Native exception/continuation stress qualification
of this new candidate remains separate from its successful-load observation.

The previous per-message ready-id vector, pending-id hash and separate take call
are not part of this direct native protocol. Payloads and the independent oracle
remain unchanged. Moving verification into the existing worker turn is a custody
change, not removal of its CPU cost. Receiving a completion uses the existing
blocking mailbox operation; no yield/poll scheduling loop is added while work is
active. Original sparse-source eligibility still uses the existing green yield.

## Backpressure and honest capacity accounting

The old diagnostic explicitly rejects each arrival when outstanding work reaches
128. The new source does not allocate or reject a catch-up list. It computes the
number of due arrivals from the original clock and retains the next unpublished
ordinal until a frame is free. Every arrival keeps its original timestamp; debt
is therefore included in the five-second deadline and final latency. This is a
change from immediate-rejection admission to backpressure, not an identical
admission policy. The original control is retained.

An ordinal ledger has bounded storage but can represent excessive waiting work.
It is not proof of stable zero-rejection service. Report observed maximum due
unconsumed messages, pending source messages, last publication time, full drain
duration, completion throughput and p99/p99.9. If logical outstanding work exceeds
the original 128-message window, label the original-window guard failed; do not
call a zero in the rejection column capacity acceptance. These are observed
source-cycle maxima, not continuous-time instrumentation. No arrival timestamp
is reset to admission time. The existing five-second watchdog, per-message bound
and startup/shutdown limits stay fixed.

Each 500k/s measured arm schedules five million messages over ten seconds and
includes the final drain in completion throughput. The executable rejects rates
above 500k/s for this candidate until the zero-rejection guard is established.
Use the existing native launcher with `--backpressure-compare --rate=500000
--seconds=10`. This retains the original mailbox control and runs the candidate
on the same live VM/image. It is a native component result, not Rust API admission.

## Reused model and retained counterexample

`native_mailbox_backpressure.qnt` refines `native_actor_reference`, which reuses
the continuous-producer, capacity-owner and Actor models. It separates offered
source debt, delivery, frame custody, native execution, verification and final
consumption. Two frame slots and three ids are finite symmetry reductions of
the concrete three-frame protocol; this is bounded safety evidence, not a proof
of timing, arbitrary-scale liveness or zero rejection at a numeric rate.

The first random run found a real modeling omission: the inherited API allowed
client consumption before native retirement, so a frame could be recycled while
native capacity was still held. Seed `0x1ce36` retains the counterexample. The
refinement now requires native retirement before frame-returning consumption.
A focused mutation test demonstrates that removing this condition violates
ownership safety. Other negatives detect silently dropping source debt and
recycling an unread result. Seven scenarios and 1,000 traces of 50 steps with
seed 1 pass after the correction. The counterexample is retained locally with
the frozen source and native results; no passing test substitutes for performance
or exceptional-path qualification.

## First same-VM 500k/s observation: not accepted

Both arms used the same frozen binary/image and the original generic handler.
The candidate moved full verification into the existing worker turn and enabled
source backpressure. It did not enable the earlier guarded vector specialization.

| Path | Planned | Completed | Completed/s | Rejection | p99 ms | p99.9 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Original mailbox | 5,000,000 | 4,963,523 | 496,348.03 | 0.72954% | 0.330 | 0.495 |
| Backpressure, generic handler | 5,000,000 | 5,000,000 | 496,914.61 | 0% | 99.170 | 107.956 |

Both report zero accepted errors/timeouts and clean shutdown. The candidate
observes 54,431 pending source messages and 54,433 due unconsumed messages,
exceeding the original 128-message bound. It drains 62.093 ms after the final
scheduled arrival; maximum publication lag is 108.862 ms. Its observed zero
rejection does not pass latency or bounded-outstanding acceptance. Do not infer
stable capacity or a numeric mailbox ceiling from this finite run. The original
control's favorable result also does not reproduce every historical condition.

Local artifact directory: `target/replay-native-mailbox-20261027/backpressure-500k`.
Source manifest SHA-256:
`6b773e9578f58afb1ba616af4d015457110258a4285e685028b82260860ef97f`.
Binary SHA-256:
`b788bd162812d41ad8b9e0693d0d3f839d2ddbceac4115a9e78b33d7de56b809`.
Image SHA-256:
`37cdc96daf3b1c1092d4f889ce33aa5041c51a5a44071561eb80dd2321b24795`.
Raw results, source snapshot, checks and the earlier model counterexample are
retained locally. Release bench Clippy and `git diff --check` pass. Nothing has
been committed, pushed or promoted to a public default.

The next targeted variant uses the same source/frame/verification protocol with
the existing guarded native vector-access kernel. This changes business access
cost, not payload generation, the oracle, scheduling layers, credit limits,
source timestamps or the acceptance gate. Its native edge contract runs before
the measured clock. It is restricted to 500k/s and must independently pass the
historical guard; a throughput increase in an earlier rejected experiment is
not acceptance of this combination.

## Guarded-access observation: zero rejection, latency still fails

The targeted variant ran only at 500k/s, once, after the build finished. It used
the same original input constructor and independent oracle, three transfer
frames, four workers and five-second bounds. The existing data-access edge
contract ran before the clock. No million-rate experiment followed.

| Metric | Result |
| --- | ---: |
| Scheduled/completed messages | 5,000,000 / 5,000,000 |
| Rejected / accepted timeout / native error | 0 / 0 / 0 |
| Completed messages/s, including drain | 500,000.03 |
| Planned-to-consumed p99 | 62.106 ms |
| Planned-to-consumed p99.9 | 92.450 ms |
| Observed maximum pending source messages | 47,983 |
| Observed maximum due unconsumed messages | 47,985 |
| Maximum publication lag | 95.966 ms |
| Drain after final scheduled arrival | 0.001458 ms |
| Final retained native/staged requests | 0 / 0 |
| Owner shutdown | Clean |

It reaches the offered rate and drains fully, but source debt greatly exceeds
128 and p99 fails the historical 0.418 ms guard. Zero rejection is therefore a
delivery/backpressure result, not acceptance of low-latency 500k/s capacity.
Final drain alone hides transient debt; successful p99 and debt must stay next
to throughput. The finite observation does not establish long-term stability.

The generic pair and this specialized observation are separate runs with
different frozen binaries/images. They are not a paired proof of the numeric
latency improvement. Do not attribute the source pauses to OS scheduling, GC
or a particular protocol phase without same-interval evidence. The cost removed
is grounded in generated vector-access code; the remaining pause is unresolved.
Further threshold, queue-size or scheduling-layer changes are not justified by
these aggregate results. In particular, replacing the source-debt representation
with a larger materialized mailbox would hide rather than resolve the guard.

Local artifact directory:
`target/replay-native-mailbox-20261027/backpressure-access-500k`.
Source manifest SHA-256:
`e1ef4113a3d4ceb157bea727f78b76708cbcff96852ce22e8fedbe822ad103b9`.
Binary SHA-256:
`e815e5029aaab2f855ab03949be09e550cb03fb4a7f124c29f7867054d5dc36e`.
Image SHA-256:
`b694331813cd710ac72c3fa8355dc680a1675e08c9d45598852fd58b0ed33dc8`.
The candidate remains unaccepted and unsubmitted. The next open problem is
transient debt during the native exchange cycle, with original source timestamps
and full verification retained, not another 1M/s attempt.
