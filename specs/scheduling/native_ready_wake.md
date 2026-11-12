# Native Gerbil ready-edge handoff

This is a historical experiment. The 3477a02 Mailbox implementation retires
qualification operations 59–61 and their benchmark flags. The protocol and
commands below describe the earlier implementation.

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

## Reassessment at the frozen 7F source

Operation 62 still uses the scheduled path's immutable reply pair and native
mailbox order. The existing edge is not a FIFO substitute: if slot 1 publishes
first and slot 0 publishes next, a cursor at slot 0 selects the later result.
The QNT trace retains that counterexample even though the ownership invariant
holds. Stale wake acknowledgement returns no frame, and a singleton published
after the receiver parks still requires one notification.

The original singleton pays for the reply pair and a mailbox send/receive.
The edge can avoid the pair, but a parked singleton still pays for that
send/receive, plus the publication CAS, edge acknowledgement, and ready scans
before waiting and after acknowledgement. The historical 6.09% notification
saving does not amortize an additional operation of unknown cost on every
completion. FIFO would also require recording publication order rather than
recovering it from a frame scan.

A bounded FIFO of generation-tagged completions with a receiver wait state is
a distinct design. Only the transition from a parked receiver without an
outstanding wake should issue a wake. A ready receiver should pop one result
and retain immediate refill after each returned frame. Publication order and
the check that arms parking must share a linearization boundary. Any already
issued wake still needs acknowledgement or joined-worker teardown. This
design can remove a notification while the source is runnable, but its FIFO
publication and park synchronization costs have not been shown to pay for
themselves. No runtime variant is admitted and no 1M/s pair is started.

## Bounded FIFO and wait registration model

The new refinement reuses the mailbox/frame model. Publication appends the
request identity and atomically converts an armed wait into a claimed wake.
Claim and send are distinct steps. The source can consume the result before
the worker sends, but then must still acknowledge that old wake before it can
arm again or close. Acknowledgement cannot free a frame. FIFO consumption
removes exactly one verified result, and its credit permits the next input
while older ready siblings remain queued. Arming from a stale empty check is
an explicit mutation rejected by the invariant.

The source model uses the existing two-slot reduction. The 7F implementation
has three frames. A native implementation would need a bounded, allocation-free
FIFO with one synchronization boundary shared by append, head removal and
wait registration. Merely replacing the current rotating scan with a FIFO
while retaining the current unconditional empty-token claim does not add
the running-receiver fast path.

The operation ledger for one result has two cases. With the receiver running,
the proposed FIFO can skip the reply pair and mailbox send/receive, but it
still needs synchronized append and head removal. With the receiver already
parked, it additionally needs wait registration and wake claim/acknowledgement
and still incurs mailbox send/receive. Claim can share append's transaction;
acknowledgement can potentially share head removal's transaction. The parked
case thus saves the pair but does not save a notification. The existing
historical edge counters measure coalescing with an outstanding token, not
how often this new wait protocol would actually find the source runnable.
They cannot populate that cost equation. No production patch or performance
result is admitted on the basis of these model transitions.

## Isolated packed FIFO cost check

The local fixture under `.ci/ready-fifo-cost` packs three frame indices and
receiver wait state into one atomic word. It retains frame ownership until
consumption; it neither stores a Scheme object nor substitutes for the native
Mailbox. Its assertions cover FIFO under immediate slot refill, one wake when
an armed receiver gains multiple completions, and acknowledgement of a wake
after the receiver already removed its result. The fixture passed with GCC
16.2.0, `-O2 -mcpu=apple-m1`. The test uses a compiler trap for assertion
failure. The first test link could not resolve the SDK assertion symbol.

With assertions excluded, GCC emits an acquire load and an acquire/release
CAS for publication, and an acquire load and acquire/release CAS for the
receiver update. Neither function calls another function, allocates, or uses
the stack. A failed CAS retries. There is no ready-frame scan.

| Successful singleton, no CAS retry | Added FIFO work | Reply Mailbox work |
|------------------------------------+-----------------+--------------------|
| Source running | Two acquire loads and two CAS operations | Reply pair and send/receive avoided |
| Source already armed | Three acquire loads and three CAS operations, including arming | Pair avoided; send/receive retained |
| Source consumes before wake delivery | One additional receiver update to acknowledge the stale wake | Old wake still has to be received |

The parked row includes the arm that precedes publication. Its acknowledgement
is fused with FIFO removal; if the result was consumed early, acknowledgement
requires a separate update instead. A fixture notification is a return flag:
Scheme token handling, runtime rooting and the real park boundary are outside
this assembly count. These instruction counts establish structural costs,
not cycle costs or the running/parked mix of op62. The running path may pay
for itself; the parked path saves only the pair while adding synchronization.
Without a justified mix and cost comparison the candidate is not admitted.

Source SHA-256:
`03d60b65d2b0c451574264529e9775b600c0e4146e6075bab2ad8a5e3052930f`.
Assembly SHA-256:
`a4afe1b8c5bad89cb5a99e1f69164d626c82df6238a7e28476a62ebae583d3c5`.

## Gerbil Mailbox singleton fixture

The compiled isolation module under `.ci/ready-fifo-cost` uses real
`thread-send`, `thread-receive` and Gambit vector CAS. The generated C has
two vector CAS sites, one for publication and one for receiver updates.
Both arms use the same resident green worker, one reusable frame, request
mailbox, verification and frame retirement. The original sends the immutable
reply pair. The candidate publishes the slot and sends a wake only after
the source has armed. The stale scenario consumes the result before taking
and acknowledging the queued wake. Its comparator is the original running
scenario with the same scheduling handshake.

One fixture run used A/B/B/A order for each scenario, with 250,000 exchanges
per block. Only block boundaries read the clock. The running and stale
scenarios use a common worker-turn marker so the source deliberately takes
after publication. The parked scenario uses the blocking receive path
without that marker. It does not measure whether each individual receive
blocked, or estimate the runtime path mix. No business handler, admission
load, percentile or op62 observation was run.

| Scenario | Original ns/exchange | FIFO ns/exchange | Paired FIFO minus original, ns/exchange |
|----------+----------------------+------------------+-----------------------------------------|
| Running | 340.257 | 321.308 | -15.025, -22.872 |
| Armed, then receive | 499.944 | 520.072 | -83.955, +124.212 |
| Consume before wake acknowledgement | 335.291 | 343.547 | +16.336, +0.175 |

The running arm saved 18.95 ns/exchange on average. The parked pair changed
sign with order, and the stale wake path did not show a gain in either
pair. For illustration only, the means would require about 51.5 percent
running-path replies to break even in a mixture of running and parked paths,
before charging stale drains. The sign reversal makes that estimate unsuitable
as an acceptance threshold. The historical 6.09 percent coalescing figure
comes from a different mechanism and cannot provide the mixture for op62.
The fixture also deliberately excludes the business handler and interleaving
of three active frames. The existing evidence does not establish positive
net cost on the original service cycle, so no runtime integration is admitted.

An initial run relying on one yield failed to establish the worker turn.
The common fixture marker corrected that assumption, and the failed receipt
is retained. The executable uses the installed Gerbil with a dynamically
loaded GCC-compiled fixture module after the standalone static link failed
against local SDK and library paths. Fixture source SHA-256:
`0e8c6b22fa9d0f9a461ba4dfeb7a06d8c3b335735eda09a345bef1ca2c722bac`.
Compiled module SHA-256:
`649adbce05018b4c038d9d2403b1d2d4604775163c7ee74f32419f2cf995b34a`.

## Op62 path mix and committed receive

The scheduled op62 loop supplies free frames before its timed mailbox receive.
If there is no active frame, its separate idle branch yields. The receipt's
`native_source_park: false` describes that idle-source policy. It says
nothing about whether the timed completion receive found an existing reply.
The 1M record contains 1,000,000 consumptions, three physical frames, a
6,744 peak source debt and 998,385 samples over the outstanding guard.
Those samples describe source debt and live custody, not mailbox occupancy
at receive. One schedule can have a reply already queued at the same source
counts where another schedule still needs its worker to run. The retained
receipt gives no useful numeric bound on the running/parked fraction.

After an atomic empty-check and arm, the source can commit to receiving the
wake before removing a result. The acknowledgement and FIFO head removal
then use a single state update. A worker delayed between claim and send
still owes the wake. The source does not take a frame during that gap,
so this path cannot turn its current wake into an empty acknowledgement.
The new QNT case checks that the first result returns its own credit,
that the next input can reuse it while an older sibling remains queued,
and that no additional wake is issued. Actual frame release must still
follow the existing source-side validation, independently of the FIFO pop.

The parked fixture already measured acknowledgement fused with pop.
Committing to receive removes the separate stale-drain update in that
scenario, but does not remove the parked arm or improve the measured
parked singleton's operation count. The existing Mailbox wait checks its
own inbox. A separate FIFO must register its wait against the same state
that publication updates. Dropping that registration reintroduces the
model's stale-empty-check counterexample. Arming on the previous pop
instead would cover source validation and refill, issuing wakes while
the source is still running. Absorbing the arm requires a scheduler-owned
wait operation coupled to FIFO publication. No such operation was found
in the current path, and modifying the SDK is outside this experiment.

The prior fixture's roughly 51.5 percent running-path break-even estimate
remains illustrative, since its parked deltas disagree in sign. The
committed-receive scenario closes an ownership alternative, not the
performance gate. No fixture or business workload was rerun.

## Reuse of the installed Mailbox boundary

The installed Gambit reports ea114fc, matching the locally available source
revision ea114fc3d2f120abbe20393c1f9aeafdd3f8c89f. The installed mailbox
declarations and library symbols agree with the local source interfaces.
Gerbil's local Actor send delegates to `thread-send`. In Gambit
`lib/_thread.scm`, `##thread-send` checks the heap, locks the mailbox, appends
one value, unlocks, and signals its condition variable. The corresponding
receive first checks for a queued node, then on an empty result locks and
rechecks that same FIFO before committing the condition wait. Signal does
not append a second message. An empty condition wait queue needs no thread
to be put back on the run queue. The notification here is distinct from
the separate Scheme wake-token message in the ready-edge prototype.

The mailbox already owns the only useful combined wait boundary. A ready
frame outside the inbox cannot satisfy its empty check. Publishing it without
an inbox entry and relying on a later completion to wake the source can strand
the last result or delay the first retirement. The new model counterexample
covers the empty-inbox case. Joining multiple ready results into one inbox
entry would require a publisher that already owns the others in publication
order, or synchronized shared staging. Op62's independent workers publish
one verified frame at a time. Waiting to collect a batch delays its first
result and therefore its credit and next refill.

The source also separates allocation from handoff: `lib/_fifo#.scm` allocates
a queue-node pair for every send. The op62 reply value is another pair with
the immutable request identity and the mutable frame. A packed identity may
remove that outer pair, but it retains the node allocation, enqueue, receive
and condition signaling and adds decoding plus slot lookup. Passing a bare
mutable frame drops stale-generation protection. Private mailbox fields
would couple the library to runtime internals and are outside this candidate.
No alternative examined removes a recurring handoff within the current public
boundary while preserving FIFO, independent retirement and immediate refill.
