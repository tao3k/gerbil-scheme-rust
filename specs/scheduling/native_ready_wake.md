# Native Gerbil ready-edge handoff

This experiment changes the native Gerbil Actor API path, not Rust dispatch or
the message workload. It targets recovery headroom after an owner loses CPU:
the existing path allocates and sends a result notification for every message,
even when the receiver is already running and can inspect an owned ready frame.
It does not claim to prevent OS preemption or guarantee a CPU reservation.

## Protocol

Keep four resident Gerbil workers, three active transfer frames, original input
construction, the complete order handler and independent result oracle. A worker
marks its frame ready only after handler/oracle completion or captured failure.
The result stays in that frame until the receiver consumes it. No additional
result queue is introduced and no frame becomes reusable on notification alone.

The native receiver and workers share one Scheme vector notification slot within
the existing single-processor VM entry contract. A worker uses Gambit's primitive
`##vector-cas!` to claim an empty slot with its immutable application id plus one,
then sends that integer through Gerbil `thread-send`. If another worker owns the
slot, no second notification is posted. At most one wake is claimed or queued.
The CAS is necessary even on one processor because a green-thread switch may
interleave separate Scheme reads/writes. The primitive is already used by
Gerbil's atomic promise/unwind code and retains no foreign Scheme pointer.
No handler executes under a lock or with its interrupts disabled. A worker
suspended before send retains a publication obligation, not an extra frame/root.
Issued/taken counters count events; they are not token identity or readiness.

The receiver scans the original frames in round-robin order. It consumes a ready
frame immediately; only an empty scan enters `thread-receive` with the original
five-second deadline. On receiving a wake it uses CAS to match/clear that exact
application token, acknowledges
it **before** scanning again, and rechecks readiness. A wake for an already
consumed result may yield no frame: it never completes a request or releases a
credit. Subsequent completion can publish a new edge after acknowledgement.
Acknowledging after the scan would risk erasing an intervening publication.

Round-robin selection prevents a rapidly recycled low-numbered frame overtaking
ready peers indefinitely. This is service opportunity within the owner, not OS
thread scheduling. The usual consumer still checks frame custody/failure,
records original planned-to-consumed latency, enforces the five-second request
deadline, publishes verified watchdog progress and clears the payload before
returning capacity. No timestamps are reset on catch-up. No handler is replaced
by a C implementation and Scheme suspension, exceptions and continuations remain
on the existing handler path. No claim of universal multi-shot application
continuation safety beyond the existing API ownership contract is added.

Shutdown joins every worker before returning wake counters, then drains the
optional remaining token using the existing private inbox cleanup. The counter
`retired_without_receiver_take` distinguishes cleanup from a receiver mailbox
take. Takes can consume an already queued wake and are not OS blocking events.
Joining after elapsed time is recorded keeps teardown outside the existing
throughput interval. This is not a new result Actor, polling thread or scheduler.

## Actual entry and checks

Private native service operation 60 selects the new generated implementation.
The original operation 56 remains unchanged in selection and is the control.
`--backpressure-coalesced` selects the experiment; the explicit comparison flag
`--backpressure-coalesced-compare` runs both in the same live VM/image, with the
original per-arm warmup, at 500k/s for ten seconds each. No default path changes.
The strict guard still requires zero rejection, the historical 0.418 ms p99 and
the original 128 due-but-unconsumed window. It is not enough to send fewer wakes.

Before either arm's warmup, a separate native operation 61 exercises the actual
ready lookup, edge publication and wake acknowledgement through GC, duplicate
publication, a stale token, a duplicate epoch and ready-slot starvation checks.
These checks use the same local procedures as the live workers; they reset their
state and leave no inbox residue. This contract runs once before the pair,
not before each candidate arm: its GC must not give only one arm a different
pre-measurement heap preparation. Both arms retain the original workload access
contract and per-arm warmup/GC behavior.
The procedures and their single typed epoch/cursor record live once in
`instance-ready-edge.ss`. Generated variants call this implementation instead of
expanding helper/contract bodies into every original mode. The record references
the original frame vector and receiver; it contains no second payload store.

`native_ready_wake.qnt` imports the existing mailbox/frame/root model. Small
derived projections expose existing published/consumed facts through model
module interfaces instead of copying ownership state. The refinement separates
frame publication, the worker's pending signal and receiver acknowledgement so
a context switch between them is represented. Checks include empty-scan/wait
races, late signals after fast consumption, a stale wake after frame reuse,
ack-after-scan loss and coalescing peers without starvation on refill. Two frame
slots are the inherited finite model reduction; the native implementation keeps
three frames. Bounded model checks are not OS latency or performance evidence.

## Evidence policy

The first build began before the round-robin refinement and native contracts
were finalized. It failed native compilation and produced no admissible image.
The generated function reported 28,698 basic blocks / 173,002 registers, disabled
optimization passes, ADR/ADRP assembler errors and a compiler terminated by signal
9. The precise cause of signal 9 is not established. Expanding helper bodies into
every generated mode was unnecessary, so the shared Scheme implementation above
removes that duplication. Optimization stays at `-O2`; no build-memory limit or
watchdog is widened. Rebuild the final source cohort and freeze binary/image
identity before the measured pair. Keep failed model/build logs.

No performance outcome is claimed until the original native guard is evaluated.
In particular, the existing control's historical 0.418 ms reference is retained,
and a delayed or failing control cannot establish a favorable improvement ratio.
No queue, rate, deadline, default, Rust integration or priority change is part of
this candidate. A failed candidate is isolated and never promoted or submitted.

## Native and fixed-load outcome: rejected

The shared CAS implementation builds at the original `-O2`. Twelve model
scenarios and 1,000 traces of 80 steps pass, including a non-atomic claim
mutation. The generated C contains the expected `___VECTORCAS` claim/clear
operations. The native operation-61 contract passes on the real private image;
two successive 1k/s correctness calls complete 1,000 messages each with zero
error/rejection and clean shutdown. These calls do not qualify capacity.

Strict release Clippy with warnings denied passes. Tracking the new Scheme file
initially made the build script 101 lines; extracting its existing dependency
declarations into one helper preserves the exact emitted directives. That
bookkeeping refactor is recorded separately after freezing the benchmark. It
does not change the frozen executable or Scheme/C measurement code. All local
build/model checks end before the sole performance pair. A short frozen-source
hash check runs during the pair; no compiler, profiler, observer or priority
experiment is launched alongside it. No quiet-host assumption is made.

Each measured arm schedules five million original messages over ten seconds,
500k/s, in the SAME live VM/image and executable. The contract runs once before
both warmups; both retain original access-contract checks and source timestamps.

| Metric | Original operation 56 | Coalesced operation 60 |
| --- | ---: | ---: |
| Planned / completed | 5,000,000 / 5,000,000 | 5,000,000 / 5,000,000 |
| Successful throughput/s | 500,000.02 | 500,000.01 |
| Rejection / accepted timeout / native error | 0 / 0 / 0 | 0 / 0 / 0 |
| Planned-to-consumed p99 | **0.391 ms** | **0.453 ms** |
| p99.9 | 0.617 ms | 3.897 ms |
| Maximum pending source ordinals | 446 | 3,506 |
| Maximum due-but-unconsumed messages | 448 | 3,508 |
| Maximum publication lag | 0.891333 ms | 7.012333 ms |
| Completion notifications issued | 5,000,000 (source-derived) | 4,695,264 (native counter) |
| Candidate notifications taken / retired | N/A | 4,695,264 / 0 |
| Original outstanding-window guard | Failed | Failed |

Final pool cleanup is clean. Notifications fall only 6.09472%, while the
candidate p99 rises 15.86% within the pair and exceeds the historical 0.418 ms
guard by 8.37%. Its absolute p99 and outstanding-window gate both fail. The
original path meets the requested latency and zero-rejection conditions in this
run but misses the separate 128 due-but-unconsumed guard; it is not declared
fully admitted. No threshold is relaxed to relabel either arm.

This falsifies the proposed protocol's acceptance, not all possible completion
protocols. The implementation introduces real per-message frame scans and CAS
checks, while most completions still need individual wakes. The low observed
coalescing ratio weakens the proposed payoff. However this unprofiled pair does
not attribute its entire p99 increase to those operations: its different
publication-lag spike may include owner scheduling/GC effects. Do not infer a
precise syscall, GC or OS cause from these aggregate values. No retry or matrix
is used to select a favorable run.

The candidate and launcher are frozen locally in `native-ready-edge` with source
manifest SHA-256
`2e8f22c5db0beab0acc32ad889b0b1dc95116a3217eb6fb23972dcf5e140181c`,
executable SHA-256
`d5c7b0f15bc7ed5062b99d669c43d84ed86810a69c2c72808bdd3abc408dba5a`,
and native image SHA-256
`83ba1f57e9b665c6b377b90238bac4719fbc91bed39cb60f427fc9233f3152bb`.
Raw receipts, environment, earlier failed builds and source identities stay in
ignored artifacts. This preserves the observed original 0.391 ms result and
the candidate's negative result without overwriting historical references.

After freezing, candidate runtime/launcher edits are removed from the active
source; operation 60, operation 61 and their CLI flags are available only in the
archived experiment. Existing native/bench sources are restored to their exact
pre-experiment identities. Models and this report remain as explicit rejected
research. The generated Cargo image must be rebuilt from restored sources
before any future run through the implicit build-owned image path. Reproduction
of this pair uses the explicit frozen executable/image, never a mixed cohort.
No public default, queue, priority or Scheme execution capability changed, no
1M/s trial followed, and no commit or push is made. Owner OS availability and
bounded source-debt recovery remain unresolved engineering requirements.

## Published research checkpoint

The restoration above describes the original investigation checkout. The user
subsequently authorized freezing BOTH measured arms on
`research/single-vm-service-cost`. This separate checkpoint deliberately includes
the optional operation-60/61 implementation and launcher flags so the negative
experiment can be rebuilt from Git. This is source preservation, not candidate
promotion: defaults remain unchanged and further optimization uses original
operation 56 at **0.391 ms** as its reference. The candidate's 0.453 ms result
and both failed outstanding-window guards remain part of the published record.

The [checkpoint report](checkpoints/native-mailbox-500k-20261027/README.md)
contains the exact replay command, artifact identities and full structured
benchmark events. No new performance run is attributed to the publication REV.
The independent investigation checkout stays restored. PR #18 is not merged or
used as this checkpoint's destination branch.
