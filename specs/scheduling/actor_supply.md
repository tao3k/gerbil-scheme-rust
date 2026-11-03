# Gerbil Actor opportunities and owner supply

## Scheduling authority

Gerbil service workers are resident Scheme threads using `spawn`, `thread-send`,
and `thread-receive`. A worker can take the next queued job from inside Scheme;
it does not require a Tokio task per Actor turn. Gerbil's Actor manual explicitly
supports raw mailbox messages via `<<` as well as enveloped `->` / `<-` protocols:
https://gerbil.scheme.org/reference/std/actor.html

The service currently uses those raw mailbox primitives. It is not a Tokio Actor
implementation or an invocation of the distributed Actor server. Substituting
`->>` for native admission would add a synchronous reply wait, not solve supply.
Rust publishes independent typed work to a fixed owner and observes owned
results. It does not resume, migrate or preempt Scheme continuations.

## Concrete protocol gap

The continuous source's active limit was not its publication opportunity.
`Pump::drive` called `producer.step`, received one job, and continued at the top
of the loop. The next turn called `collect` before another producer step.
Consequently a fast handler could finish and be projected before the second
available input was even constructed. Raising active concurrency from one to
three did not itself create a three-envelope runnable set for Gerbil.

The existing explicit `configure_scheme_service_feed_on` bounded ordinary inbox
publication, but its bound was not applied to the resident producer. The change
applies that same opt-in bound to registered producer turns. The first supplied
request counts toward the bound. Before each additional request, close is
checked and ordinary FIFO input is rechecked; any such input interrupts the
producer turn. Existing shared-short claim precedence is reused by the refill
helper. Capacity acquisition, per-message expiry, independent result/root
retirement and Scheme handler behavior are unchanged. Default feed remains one.
This is a bounded publication opportunity, not an unpreemptible handler batch.
Scheme may schedule or suspend a worker during any native entry.

A deterministic native test stages three inputs while the owner is held behind
a native barrier. With feed three and active three, the first result projector
must observe all three input constructions. This tests the actual producer path,
not just configured limits. It does not assert that three Actors run in parallel
on one VM CPU or prohibit early Scheme scheduling.

## Reused formal boundary

`actor_supply.qnt` imports `continuous_short`, including the existing owner and
Actor lifecycle refinements. It records per-turn publication allowance and
explicit advance opportunities separately from actual Actor execution. Actor
execution is intentionally allowed before an explicit advance: a native entry
can itself give the Scheme scheduler an opportunity. The finite two-input model
compares feed one with feed two, checks interrupted supply and retains the
existing native ownership invariants. Random safety traces do not establish OS
fairness or a performance result.

## Focused cost and acceptance checks

The directed A/B keeps active three, one VM, one Tokio worker, input window128,
native slots4/work32, unchanged short payload and oracle, and ten-second arrivals
at 500,000 or 1,000,000 per second. Only the explicit feed policy changes1 to3.
The same live VM and native image are retained within each pair.

`--phase-observation` reuses the existing owner monitor in a separate diagnostic
capture. Submit includes native construction and admission; Advance includes
Gerbil scheduler/Actor progress and readiness transfer; Retire includes result
take, projection and native cleanup. Supply and Release are host bookkeeping.
CPU and GC counters overlap and are not additive exclusive categories.
Request timestamps also provide payload construction, construction-end to
projection-begin, and projection-begin to native completion sums. The middle
interval includes queueing and boundary waits and is **not pure Actor CPU**.
Observation clocks/counters have cost; diagnostic throughput is never treated
as the unobserved capacity baseline.

No queue/default, Scheme feature, timeout or payload changes are authorized by
this experiment. Retain negative throughput and tail-latency results. Results
must distinguish explicit library window rejection from accepted execution
errors; high rejection remains a failed arrival-rate target even when successful
message p99 improves.

## Retained supply-only negative result

The first diagnostic used the original native image and Snapshot completion in
both arms. At 500,000 planned arrivals/s, feed1 completed491,130 messages in ten
seconds; feed3 completed361,247. Advance calls were491,248 and361,532 respectively:
still approximately one advance per result. Supply calls nearly doubled under
feed3. These observed rates include phase-monitor cost and are not unobserved
performance claims. The unsuccessful supply-only configuration is retained and
is not the new baseline.

In the feed1 diagnostic, owner phase means were about6.25 us for Submit,7.64 us
for Advance and4.40 us for Retire. Existing request timestamps measured about
1.92 us for payload construction and0.92 us for projection plus native cleanup.
Those intervals have different boundaries: the remaining Submit interval is not
an exact subtraction identifying pure admission cost. GC wall deltas across the
three major phases were about905,1487 and283 ms. GC CPU is process-level and is
not attributed exclusively to an Actor. No pure handler-CPU percentage is claimed.

## Paired Actor supply and bounded completion snapshots

The source of the second barrier is explicit: `service-ready-ids/receive!`
previously popped exactly one short-ready identity even in native-cycle modes.
The next candidate pairs feed3 with the existing opt-in NativeCycle protocol.
In native-cycle modes only, Scheme now transfers at most the existing resident
worker count of short-ready identities in one snapshot. Snapshot/Fair/Coalesced
policies retain their prior behavior. Bulk completions retain their existing
selection; one bounded short snapshot does not monopolize an unbounded turn.

The snapshot is already-completed work, not a CPU execution unit or a merged
logical request. Every identity still has an independent result slot, exception,
projection, root release and native capacity retirement. Client abandonment does
not return native capacity early. The existing epoch acknowledgement is accepted
only after every claimed result slot is retired. `actor_completion.qnt` refines
`actor_supply` and the existing continuous-source ownership model to cover this
independent retirement, including cancellation/close with a partially consumed
snapshot.

The final directed A/B is therefore feed1/Snapshot versus feed3/NativeCycle,
both active3, using the same live VM and newly built image within each pair.
It tests the joint protocol; it does not attribute a speedup solely to feed or
solely to fewer FFI calls. The original native image remains frozen separately.
The default Snapshot/feed1 path is unchanged.

## Joint-protocol observations and rejection (2026-10-27)

The frozen joint candidate executable was
`1fe45e3fdcc1b32f670e175ab32a2b655d5fdbee5f725ac088603d7b80e74f63`;
the candidate native image was
`0ae4afea1688762a20b89c27e65406266ecb67f95f34e247c1f4f21ba5d09dae`.
The same image and live VM were used for both arms of each pair. The image
contains the unchanged default Snapshot branch and the experimental bounded
NativeCycle branch; it is not the prior frozen original image.

| Arrival rate/s | Arm | Successes | Completed/s | Rejection | Successful p99 | Successful p99.9 |
| ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 500,000 | feed1/Snapshot | 386,289 | 38,367 | 92.27422% | 27.368 ms | 69.020 ms |
| 500,000 | feed3/NativeCycle | 465,705 | 46,560 | 90.68590% | 17.609 ms | 45.494 ms |
| 1,000,000 | feed1/Snapshot | 554,033 | 55,391 | 94.45967% | 7.822 ms | 13.476 ms |
| 1,000,000 | feed3/NativeCycle | 568,511 | 56,841 | 94.31489% | 9.221 ms | 26.793 ms |

Each arm accounts for all five/ten million arrivals. Benchmark-ingress drops and
accepted-request errors were zero. Both processes reported clean service
quiescence, one joined owner, zero roots and removed runtime files. This does
not make the arrival-rate target successful: almost all arrivals were rejected.
No quiet-host or repeated-run stability claim is made.

The joint candidate improves the worse500k control but does not beat the prior
record of56,381/s and7.364ms at500k. At1M, throughput increases only about2.6%
while p99 worsens about17.9% and p99.9 nearly doubles. These are negative
acceptance outcomes. Neither the worse control nor this candidate replaces the
prior baseline. No default, commit, push or PR update follows.

## Correctness scope and initialization failure

The supply/retirement models passed the directed serial/bounded cases and1,000
bounded safety traces. Four continuous-service native tests passed separately,
including three publications before first projection under NativeCycle. The
original runtime GC/continuation/exception/close test passed separately in27.64s.
Strict Clippy for the test and bench passed.

The complete default-parallel16-test process failed with exit70. Unfiltered
reproduction identified `SERVICE-STALL last=start phase=initialization controls`
at5.02s, with the original five-second gate. A same-run three-second thread
sample shows the main runtime test waiting for its startup receipt; several
owners wait on the process-wide LIFECYCLE mutex while other owners progress
through image loading/dlopen/SDK setup and cleanup. This is initialization/lifecycle
contention evidence, not proof of a Scheme handler crash, a permanently stuck
lock, or OS scheduling as the hot-path bottleneck. The exact cause of missing
that startup deadline remains open; isolated passing tests do not close it.
Full native admission therefore remains blocked as well as performance admission.
Raw failure and sample records remain local ignored artifacts, not Git content.

The limiting cost is not explained by treating Gerbil Actors as Tokio tasks.
At500k/s a single execution CPU has2us per successfully serviced message; at1M/s
it has1us. The observed payload-construction interval alone is around1.9us under
diagnostic observation, before the remaining submission, scheduler, result and
GC costs. This is a budget comparison, not a proof of an intrinsic Gambit limit.
Further work must reduce accepted-message boundary/allocation cost and preserve
Scheme's own runnable/suspended scheduling, rather than enlarging queues or
claiming that more host tasks supply more native execution capacity.
