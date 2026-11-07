# Rejected typed Gerbil mailbox candidate (archived)

This candidate failed capacity acceptance and is isolated from the active native
type-exchange experiment. The API and implementation described below belong to
the archived candidate; they are not enabled or available in this experiment.
The negative results remain evidence. See [native type exchange](native_exchange.md).

## Ownership protocol

`configure_scheme_service_mailbox_on(owner, true)` explicitly enables an idle
ABI33 Snapshot service. The existing typed request and single-transaction input
constructor are retained. No Rust green-thread executor, queue enlargement or
new default is introduced. Other completion policies require disabling this
option first; stateful Actor lanes keep their original worker identity.

A worker performs the original handler inside the existing budget, runtime,
progress, continuation and exception contexts. It settles execution ownership on
that same Scheme thread, marks the job `posted` and sends the job identity/graph
to a resident Gerbil completion Actor. That Actor publishes the result under the
existing service lock and dispatches the next eligible queued job to that worker.
The original short/general alternation and lane eligibility are reused. It does
not wait for a Rust client to consume a result before dispatching queued work.

The private job phase distinguishes `running`, `posted` and `ready`. A posted
completion is not native readiness, a returned root or returned capacity. Close
continues to wait for it. Repeated publication is rejected. The service job table
retains the graph until normal result transfer. The completion Actor's mailbox is
bounded by already-admitted jobs, not a second independently growing backlog.

Rust transfers only the currently ready result set using the existing native
result-slot ABI. Each slot is projected, cleared and charged independently.
Pending completion messages do not prevent another ready result from retiring.
The result batch is not an unpreemptible execution batch. Native credits return
after projection/root balance; unread-output window credits return only when
Rust clients consume/drop their tickets. Existing accepted-work drain survives
caller cancellation and observation timeout.

The owner fills a snapshot of currently free producer active credits before its
next completion visit. Each entry still acquires the original native slot/work
permits and rechecks source lifetime. Close and ordinary FIFO controls interrupt
supply; no callback executes under a transport mutex. There is no new configurable
feed count or implicit larger queue. Ordinary source/wake protocols are reused.

Shutdown drains running and posted jobs, joins workers, stops/joins the resident
completion Actor, then closes native wake resources. An idle policy switch also
joins the completion Actor. Unsupported older images fail explicitly.

## Model and native checks

`mailbox_handoff.qnt` refines the existing Actor completion/supply and owner/root
models. Completion posting and readiness publication are distinct. Its directed
cases cover close with mailbox custody, the premature-claim negative mutation,
and independent retirement while another completion is still posted. Existing
`actor_completion` adds an explicit visible-ready projection rather than a new
scheduler or duplicated ownership model. Random safety exploration is not a
latency/fairness proof.

Four new focused native tests passed, including single-transaction type quota/GC,
Scheme exceptions, independent unread-window drain, cancellation/expiry, and
multishot continuation resumption on the same affine Actor. The full native suite
passed22 cases. Strict Clippy passed for the relevant test and Rust bench.
Correctness checks are separate from performance admission.

## Backpressured arrival qualification

The directed Rust profile is `--capacity-mailbox-ab`, with continuous short-only
supply. Both arms use single-transaction construction, one live VM, one Tokio
worker, active3, native slots4/work32, total outstanding128, original four-record
payload/oracle, and original five-second deadlines/watchdog. The service control
runs first and the optional mailbox protocol second on the same live VM/image.
Only the mailbox protocol differs. Excluded warmup remains unchanged.

Both arms call the actual `submit_with` API. Full windows suspend the bounded
producer; they do not drop arrivals or reset timestamps. Planned arrivals remain
500,000/s or1,000,000/s for ten seconds: respectively five/ten million outcomes.
The source lifetime remains the existing measurement duration plus five seconds.
Expected upstream deadline expiry is counted without per-message diagnostic I/O.

`capacity_flow` reports actual API admission within the original planned window,
upstream obligations still unadmitted at its end, unsuccessful upstream outcomes
already observed by then, and accepted requests not yet successfully observed.
The latter includes accepted failures, not just unfinished native execution.
API admission is staging, not proof the Scheme handler has started. Legacy
`library_started_messages` in this profile likewise denotes API staging; native
execution counts are not independently observed. Attempt-processing rate includes
expired upstream obligations and must not be presented as accepted throughput.

The complete result separately reports pre-admission expiry, accepted observation
expiry and other errors. Successful p99 still spans original planned arrival to
client observation; timed-out arrivals do not disappear from outcome accounting.
Zero `QueueFull` with large backlog or expiry remains failed capacity acceptance.
The earlier immediate-admission and native-only records remain separate cohorts.

## Final admission result, 2026-10-27: rejected candidate

Two additional correctness gaps were closed during qualification. Window credit
is jointly held by a staged job and ticket; cancellation withdraws an unclaimed
input before replacement admission, serialized against owner claim. The existing
cancellation allocation and request-started identity are reused. Claimed native
work still drains. Publication now checks the original request deadline under
the source mutex, including time spent preparing input, rather than accepting an
already-expired request under the longer source lifetime. Deterministic native
cases and the source-lifetime/early-credit negative controls record these gaps.

Final correctness:24 native cases passed in44.91s; strict test/bench Clippy
passed. The mailbox, joint-lease, waiting-admission and publication-deadline
refinements passed directed scenarios and bounded safety traces. The original
five-second watchdog is retained. On failure it only prints real aggregate
counts; it does not reset progress or count timeout processing as successful
Scheme execution.

The final candidate retains one VM, four application workers, one completion
Actor, one Tokio worker, active3, window128 and the unchanged workload/constructor.
The CLI client argument remains50, but the continuous profile actually has one
bounded producer and independent ticket observers; it is not50 independent
producers. The legacy top-level comparison label is not a description of these
capacity arms. `capacity_flow` and the explicit profile flags identify them.

Earlier controls aborted at the unchanged watchdog. The final publication-fence
candidate was therefore validated independently, once per offered rate. These
are not completed same-VM performance pairs; no percentage gain is claimed.

| Metric | 500,000 planned/s | 1,000,000 planned/s |
| --- | ---: | --- |
| Planned messages | 5,000,000 | 10,000,000 |
| API staged admissions in original10s window, /s | 214480.20 | Incomplete |
| Successful messages | 624387 | 407855 at watchdog abort |
| Completed/s, including final drain | 41625.60 | No valid complete result |
| Upstream pending at planned end | 2500000 | No complete planned-end receipt |
| Pre-admission expiry | 787388 | 4208362 at abort |
| Accepted observation expiry | 3588225 | Accepted failures not fully classified at abort |
| QueueFull | 0 | 0 at abort |
| Successful p99, ms | 5000.045875 | No valid complete result |

The500k arm completed all five million outcomes:624,387 successes plus4,375,613
expiries. Its failure rate is87.51226%; zero `QueueFull` is not useful capacity
acceptance. API staging admission is not Scheme execution and must not be called
successful throughput. The two rate columns do not mix successful-only latency
with all-arrival success rates. End-to-end observation latency can exceed the
request's entry-eligibility deadline; those tails remain reported, not clipped.

The1M arm exited70 at the five-second no-success gate. The abort receipt records
1,053,044 API admissions,645,188 accepted failed observations,4,208,362 upstream
expiries and4,738,594 not-yet-attempted planned arrivals. One admitted client
outcome was still unobserved. No complete throughput/p99 or clean shutdown may
be inferred. The500k arm reported a joined owner, zero roots, quiescence and
removed files; the failed1M arm's cleanup is unverified.

Frozen executable SHA-256:
`dd76c96d6e4e9950cdf5db4a29421437fcb09ac6f4eb5b0f1cb900979deba232`.
Frozen image SHA-256: `c9bc224c647635d31b929169cd08baa415a7a41886d65143324121145d25b5b7`.

Original and corrected source snapshots, complete outcomes, aborted outcomes and
negative controls remain ignored local evidence. Historical native-only and
immediate-admission references remain separate cohorts. No default, timeout,
queue or payload reduction is used to obtain a passing result. There is no
performance acceptance, commit, push or PR update from this candidate.

The result supports the cancellation/ownership corrections, not a production
throughput claim for the added completion Actor. This candidate must not replace
a frozen performance baseline. The typed path still enters native construction,
submission, readiness and projection boundaries per request; simply adding an
Actor does not remove them. Further work must consider an owner-native typed
exchange protocol with independently credited inputs/results and cancellation
acknowledgement. That is a design direction, not an implemented or measured
million-message result, and it must retain the general Scheme request path.
