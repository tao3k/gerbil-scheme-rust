# Source-owned original timestamps

This experiment starts from original operation 56 in checkpoint `6b35864`, whose
recorded 500k/s p99 is 0.391 ms. It does not use the rejected ready-edge operation
60 as an optimization base. Operation 62 retains the original per-result Gerbil
mailbox handoff, four workers and three active frames. Its sole protocol change
is source ownership of an exact original planned timestamp through consumption.

## Evidence and hypothesis

Original source supply checks the oldest ordinal's deadline, records publication
lag and later records result latency by repeatedly calculating
`start + floor(id * 1e9 / rate)`. This redundant ordinal-to-time conversion is
visible in the actual source. The earlier clock-heavy CPU samples cannot
attribute an exact cost to it because they include idle pacing and sample
aliasing. This is therefore a falsifiable recovery-headroom hypothesis, not an
assertion that division caused the observed OS owner stalls.

The source now selects a plan once per lifetime. For period quotient `q` and
remainder `r`, its exact recurrence is `time += q + carry_overflow` and
`carry = (carry + r) mod rate`. The qualified 500k/s and 1k/s paths have integer
periods; they select a direct time-increment procedure once and do not maintain
unused fractional state per message. General/non-divisor rates retain the exact
rational procedure. Arithmetic stays generic and checked; Scheme application
bignums, exceptions, suspension and continuations are not restricted.

The source stores that immutable time with the existing frame before sending
it. Workers still receive fresh native input, execute the original full handler
and independent oracle, and replace only the payload/result field. Verified
receiver consumption uses the saved time, then clears it before frame reuse.
No new payload queue, input reuse, result batching, Actor, scheduler, notification
scheme or Rust per-message entry is introduced. The extra frame field is one
owned timestamp, not an extra request slot.

All wall-clock reads, due-arrival calculations, original metric sample points,
5-second source/request/progress checks and the 128 outstanding guard remain.
Nothing is retimed to actual issue or restart time. This can improve useful
work per owner CPU interval; it cannot make an OS-preempted owner execute.

## Protocol checks and native entry

`native_source_plan.qnt` imports the existing mailbox/frame/root model. It checks
the recurrence against the original closed form, timestamp retention through
source stalls and out-of-order completion, clearing on consumption and frame
reuse, and accepted-work drain after close. Retiming late work and losing carry
are negative cases. The initial close/drain test incorrectly tried to admit
root-free work after close; the scenario was corrected to admit work first,
preserving the inherited close protocol rather than weakening it.

Native operation 63 exercises the actual plan procedures against the unchanged
closed-form oracle for integral and non-divisor rates, carry transitions and
ordinals around the five-million boundary. It runs once before both arms'
warmups, without a candidate-only pre-arm GC. Native operation 62 is selected
only by explicit `--backpressure-owned-schedule`. The explicit comparison flag
`--backpressure-schedule-compare` runs original 56 and candidate 62 in one live
VM/image with the original warmup and five million arrivals per arm. The frozen
0.391 ms result is retained separately from any newly observed control value.

The source/C clock bridge, application computation, result oracle and runtime
defaults are unchanged. The candidate is a native qualification path, not a
claim that production Rust integration or upstream Gambit optimization has been
completed. Only a passing original-load gate permits submission. Negative
performance, failed controls and unqualified cleanup remain explicit.

## Native outcome: promising absolute latency, incomplete admission

The original `-O2` native build and strict release Clippy pass. The native
operation-63 contract passes, and the same VM completes two 1,000-message
correctness calls with zero error/rejection and clean shutdown. Eleven model
scenarios and 1,000 traces of 80 steps pass. These do not establish latency.
All local compilation/model checks finish before the sole fixed-load pair.
No profiler, observer, priority change or extra diagnostic build runs alongside
that measurement. Other tasks are neither paused nor assumed absent.

Each measured arm completes all 5,000,000 original messages in the same live VM,
executable and image. The historical reference is separate from this run's
control; no performance replay is used to select a favorable quantile.

| Metric | Frozen original reference | This run's original control | Source-plan candidate |
| --- | ---: | ---: | ---: |
| Successful messages/s | 500,000.02 | 500,000.02 | 500,000.02 |
| Completed messages | 5,000,000 | 5,000,000 | 5,000,000 |
| Rejection / accepted timeout / native error | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| Planned-to-consumed p99 | **0.391 ms** | **23.044 ms** | **0.317 ms** |
| p99.9 | 0.617 ms | 35.505 ms | 0.431 ms |
| Peak pending source ordinals | 446 | 18,232 | 349 |
| Peak due-but-unconsumed messages | 448 | 18,234 | **351** |
| Maximum publication lag | 0.891333 ms | 36.463125 ms | 0.697625 ms |
| Final pool cleanup | Clean | Clean | Clean |

The candidate's observed absolute p99 is below both the retained 0.391 ms
reference and the 0.418 ms guard, with lower observed p99.9 and source debt than
the frozen reference. This is not proof of a causal percentage improvement
across runs. In particular, the newly delayed control must not be used to claim
a roughly 72x speedup or to replace the frozen baseline. Without same-spike
CPU/OS/GC evidence, its 23.044 ms is not attributed to the host, compiler,
source arithmetic or Scheme runtime. The fixed offered-rate throughput also
does not measure the maximum sustainable service rate or recovery CPU capacity.

The candidate still fails the original 128 due-but-unconsumed guard (351).
Thus it is **not fully admitted and is not submitted**. Its source remains
isolated behind the explicit experimental operation/flag in the local worktree;
default execution and the published reference at `6b35864` stay unchanged.
The 0.391 ms original path remains the basis for subsequent work, not the
23.044 ms control or an unaccepted candidate. No retry or broader matrix follows.

Frozen local directory: `native-source-plan`. Base REV:
`6b3586425b9d75c5bf57ef4a5c911e3fa5d88124`. Pre-outcome source manifest SHA-256:
`664b492eb87003034ede5246ddf9e60479fdfa6aeb580274ce3f780b3a9e21dc`.
Executable SHA-256:
`033d269c61895c79b2d1625d1c380c1dfe3d14d9981fe6a99a6f839088746f76`.
Native image SHA-256:
`de30430f474106c3e8691e9e5bb99d62877d53ea0f1778f1c1843ba4d924e25b`.
Source, complete receipts, environment, model failure/correction and validation
logs remain local. No logs, patches or native binaries are pushed to GitHub.

## User-authorized research publication

After reviewing the incomplete admission, the user explicitly authorized freezing
the 0.317 ms version on the current research branch. This supersedes the earlier
decision not to submit a checkpoint; it does not change the failed guard into a
pass. Operation 62 becomes the explicit research continuation reference, with
0.391 ms retained as the original historical reference. Neither default flags nor
the 128 bound are changed, and PR #18 is not merged.

The [published checkpoint record](checkpoints/native-source-plan-0317ms-20261027/README.md)
contains every structured event, including the 23.044 ms control and 351 > 128
failure. Publication adds source/evidence, not a new benchmark result or a claim
of production acceptance. No raw logs, patches, traces or binaries are committed.
