# Owner wait evidence and coverage

## Source identity and missing evidence

The remote `research/single-vm-service-cost` branch was read on 2026-10-29 and
resolved to `6d4ec27ccf166b89279999cce66e33ce1e34c643`. A fresh isolated checkout
of that exact revision is the source authority for this audit. The unrelated
older primary checkout and its uncommitted changes were not modified.

The earlier temporary research checkout, uncommitted observation changes,
diagnostic executable, VM image and raw logs were unavailable when this work
resumed. They were not reconstructed from conversation summaries. Earlier
conversation timing results are historical claims, not verified input artifacts
for this audit. A new build cannot stand in for the missing frozen binary.

The historical [0.317 ms checkpoint](checkpoints/native-source-plan-0317ms-20261027/README.md)
and its **351 > 128** failure remain unchanged. No performance run, new baseline,
capacity claim or performance admission is made here. The source observation
implementation at this revision contains a remote sampler; it is not the later
owner-only implementation described in the conversation. This work neither
enables nor recreates that sampler. The ordinary operation-62 path is unchanged.

## Audited sources

| Authority | Evidence | Limit |
| --- | --- | --- |
| `6d4ec27` qualification source | `instance-mailbox-backpressure.ss`: workers receive frames, execute the original handler and oracle, publish results; source receives, verifies identity and latency, publishes progress, clears roots and returns the frame | A Scheme receive is a green-thread dependency, not proof that the OS owner is blocked |
| Same revision, `ffi/owner_wait.h` | Image-local `select` wrapper forwards arguments, preserves errno, resolves the real function once, and records calls only when explicitly enabled | Disabled recording, overflow, unmatched scope and uncertain image binding prevent negative coverage claims |
| Same revision, `build/instances.rs` | SDK archive is linked into a private image with a restricted export list | Source/link intent is not the missing frozen image's actual relocation evidence |
| Installed SDK build receipt | Gerbil source `1cfb032c7a1612637da6205f5f8a15683eba3ba0`, Gambit base `ea114fc3d2f120abbe20393c1f9aeafdd3f8c89f`, patchset `618d76f9eb725e27ce13194514ce5b8ca5489038` | The base source is not the complete deployed patchset |
| Installed SDK `libgambit.a`, undefined-symbol inspection | `os_io.o` references `_select`; no undefined `poll`, `ppoll`, `nanosleep`, `pthread_cond_wait` or `pthread_cond_timedwait` was found in this bounded symbol query | Symbol presence is not an executed call, all-wait coverage or proof of a specific runtime backend/configuration |

The SDK receipt's exact upstream Gambit base files were fetched read-only:

- [`lib/_thread.scm`](https://github.com/gambit/gambit/blob/ea114fc3d2f120abbe20393c1f9aeafdd3f8c89f/lib/_thread.scm):
  `##thread-send` enqueues before signaling; `thread-receive` delegates to
  `##thread-mailbox-next-or-receive`. Scheduler runnable queues and native device
  waits are separate layers. A receive by one green thread need not block the
  OS owner while another green thread can run.
- [`lib/os_io.c`](https://github.com/gambit/gambit/blob/ea114fc3d2f120abbe20393c1f9aeafdd3f8c89f/lib/os_io.c):
  `___device_select` contains configuration-dependent `select`, `poll`/`ppoll`
  and pure-sleep branches. The general source possibilities must not override
  the narrower installed archive evidence above. A pure-sleep branch is not
  asserted to be present in the deployed SDK.

Do not infer a compile-time scheduler variant from one active VM processor.
Only the existing fixed-owner execution path is under investigation; there is
no runtime scheduler redesign, SDK patch or change to processor counts.

## Three distinct meanings of waiting

1. **Green-thread dependency:** the source needs a reply and gives the existing
   Scheme scheduler an opportunity to execute a worker. A queued/runnable worker
   does not need an additional Rust scheduler or timer to become Scheme work.
2. **Native call envelope:** wrapper entry-to-exit wall time includes execution
   before entering the kernel, kernel work/blocking, and any runnable scheduling
   delay before the owner resumes and records exit. It is not exact kernel
   blocked time. Even a zero-timeout poll can have a long observed envelope.
3. **Owner CPU absence:** matched enclosing CPU readings can bound how much of
   a verified-progress gap was not owner CPU. They cannot divide the remainder
   into native blocking and runnable OS wait.

For a progress gap `[g0,g1]`, valid same-owner enclosing readings must satisfy
`before.end <= g0 < g1 <= after.begin`, with internally ordered wall brackets
and nondecreasing CPU. Their CPU increment is an **upper bound** on CPU inside
the gap. A conservative lower bound for wall time without owner CPU is
`max(0, g1-g0 - enclosing_cpu_increment)`. An enclosing CPU increment larger
than gap wall time is possible because it includes work outside the gap; it
does not justify negative time or an exact in-gap CPU price.

For a completed same-scope native call, clip its envelope to the gap before
reporting overlap. Missing, reversed or different-scope intervals are unknown.
Even complete `select` coverage establishes nothing about uncovered waits.
Neither scalar `select_samples` nor zero `select_waiting_samples` gives this
coverage or proves a lost wakeup. Root/frame retirement remains tied to actual
source consumption, never a wrapper return or a progress timestamp.

## Code change and checks

`benches/gerbil_actor_arrival/wait_evidence.rs` implements the cold bounds and
report validation. The existing benchmark projects its `wait_evidence` field
only after native return. It consumes the same closed owner report, retains
the original readings, rejects incomplete/mismatched records, and explicitly
leaves `kernel_blocked_ns` and `os_scheduling_wait_ns` null. It adds no native
call, timer, thread, clock read, notification, pointer retention or frame state.
The helper accepts nanosecond values; its JSON output keeps explicit units.

The same helper is tested through the existing Cargo qualification crate with
no native features. Eight tests cover matched brackets, unknown/invalid data,
overlap clipping, unsigned limits and projection that preserves unknown causes.
`serde_json` is reused as a test dependency, not a new VM transport. These checks
do not initialize a VM, build a new native image or validate the complete
feature-enabled benchmark executable.

`owner_wait_coverage` extends the existing QNT file and imports the existing
owner-availability model. It introduces no second allocator/scheduler. Ten new
directed scenarios and seven inherited owner cases pass with the TypeScript
evaluator and seed 1. They preserve the counterexamples to classifying a wrapper
envelope as exact blocking, excluding waits from absent records, or equating a
wake/runnable state with actual owner execution. This is interpretation and
ownership validation, not measured latency or native liveness acceptance.

## Next targeted observation contract

The next falsifiable question is whether a specific progress gap intersects a
completed native wait envelope in the **same owner generation and arm**. Reuse
the existing image-local wrapper and lifecycle boundaries rather than adding
another scheduling layer. Before implementing capture, require:

- a proven image binding and observation scope, detached before owner retirement;
- owner-local entry/exit pairing, including errno, interruptions and error exits;
- bounded retention with explicit loss/overflow, so absent records remain unknown;
- original timeout arguments and result/readiness facts, without consuming data;
- same-gap overlap and CPU brackets, with blocking/scheduling ambiguity retained;
- an unobserved control that preserves original timestamps, two measurement
  points, five-second gates, oracle, defaults and frame-return order.

No new wait capture is implemented in this slice. Recreating the unavailable
owner-only recorder from memory or reactivating the old sampler would change
the observation cohort. A runtime candidate is deferred until that boundary is
closed; changing arithmetic, adding wakes, returning credits early or enlarging
queues is not justified by the present evidence. No commit, push or PR #18 merge
is included. Structured provenance is retained in `owner_wait_coverage.json`.

## Follow-up: bounded owner-local association core

`ffi/owner_wait_scope.h` implements the association state, with named C records
and no SDK dependency. In the initial core-only slice it was **not included**
by an adapter or Scheme graph; runtime sources remained byte-identical to
`6d4ec27`. The later, compile-gated wrapper binding is described below. The
core-only stage validated bookkeeping before changing an observation cohort.

The caller owns a zero-initialized state for its image/owner lifetime. It must
invoke this single-owner API on the actual fixed owner and retain that state
until every entered call has exited. The numeric owner identity is a checked
association token, not OS thread authentication or permission for concurrent
access. Real thread affinity, image binding, foreign-call termination and Scheme
continuation behavior still require validation at the future integration point.

| Event | Association effect | Boundary |
| --- | --- | --- |
| Open | Advance generation; reset one scope | Reject nested open, unresolved old calls and generation exhaustion |
| First original progress callback | Prime a complete progress interval | Earlier initialization calls are excluded, not assigned zero cost |
| Wait enter | Capture supplied wall/CPU and timeout kind; issue a generation/sequence ticket | No nested wait, clock read, allocation or new notification |
| Wait exit | Pair that ticket; accumulate call wall/CPU, timeout category and actual result/error category | Old, duplicate and wrong-owner tickets cannot finish the current call |
| Original progress callback | Seal the previous interval; retain the largest gap and its aggregates | Must match the preceding progress timestamp; cannot seal while a wait is pending |
| Normal close | Stop accepting events; permit a cold snapshot | An entered call must have exited first |
| Abort | Preserve failure and require the entered call to acknowledge exit | No new generation while the old call remains unresolved |

The core retains only current-interval aggregates and one largest complete gap;
it has no row array, growable buffer or capacity setting. Counter exhaustion
returns `EOVERFLOW` and invalidates the evidence without wrapping. A later
malformed event also invalidates an earlier favorable gap; later short gaps
cannot erase the largest gap. The snapshot includes generation, owner, aborted
status, invalid status and an explicit count of unsealed tail calls. A failed
scope remains reportable after its pending call exits, but cannot satisfy the
complete-gap predicate. An empty scope is unknown, not zero wait.

The first progress interval is intentionally unpriced because open does not
invent an original source timestamp. This mechanism cannot explain first-request
delay without a separate validated source-epoch binding. The unsealed tail is
not reassigned to the previous interval, and retaining the largest interval is
not full-arm call coverage. Even a complete association record has no field for
exact kernel-blocked time or exact runnable OS wait. A syscall error is retained
as an observed outcome; complete observation does not imply successful service.

Only the caller-supplied timestamps are used. No real clock, Scheme object,
progress pointer, callback, native thread, mutex, root, service capacity or frame
is retained or modified by the core. No business handler receives a new probe.
It is an observation helper, not a producer, completion queue or scheduler.

`tests/owner_wait_scope.rs` compiles and runs the exact header's C contract
through the existing Cargo crate using the host C compiler. Its six groups
cover timeout/readiness/error outcomes, generation/sequence reuse, failed close,
aborted-call acknowledgement, stale exits, empty scopes, clock ordering, sticky
invalidity, tail attribution and counter exhaustion. The executable must emit
`OWNER-WAIT-SCOPE-CONTRACT-OK`. The same fixture passes ASan and UBSan; strict
Clippy and formatting checks pass for the Rust test runner. This does not prove
concurrent safety, actual FFI unwinding, SDK integration or GC/continuation behavior.

`owner_wait_scope_lifecycle` extends the existing QNT file by importing
`owner_wait_coverage`, with explicit projections of its existing owner state.
It introduces association metadata only, not a second frame allocator. Eight
new directed scenarios plus ten inherited coverage cases pass on the TypeScript
evaluator, seed 1. The `safety` invariant holds for 200 sampled traces of up to
40 steps. The model uses mathematical integers; C counter exhaustion is checked
by the actual C fixture, not inferred from that model. These checks do not
establish timing, fairness or unbounded liveness.

Two model development errors are retained in the follow-up receipt: nested
import names were not directly available, requiring explicit owner projections;
and a new `pending` field collided with the inherited source-debt state, so it
was renamed `callPending`. Neither required changing runtime ownership.

The integration remains gated on one actual, owner-local binding to the existing
wait/progress callbacks and explicit detach/unwind handling. In that slice no
callback was wired, sampler enabled, image built or benchmark arm run. Its exact
source identity and validation scope remain in `owner_wait_scope.json`, a
historical slice receipt, not a current source manifest. The 0.317 ms reference,
351 > 128 failure, original workload, defaults and five-second gates remain
unchanged. No commit, push or PR #18 merge was included.

## Follow-up: actual select wrapper binding

The existing `ffi/owner_wait.h` now includes `owner_wait_scope_binding.h` and
entry/exit/cancellation hooks only under `GERBIL_OWNER_WAIT_SCOPE`. The normal
Cargo image build does not define it. No new runtime option, default scheduler,
queue or Scheme handler probe is introduced. The existing build input list
tracks the two diagnostic headers for any later selected image build.

With the switch absent, the preprocessed test translation unit containing the
wrapper is byte-for-byte identical when using the original `6d4ec27` header or
the current header. Both use the same current host compiler and fixture. This
proves removal of the new association code in that comparison context; it is
not frozen-binary identity, a whole SDK-image comparison or a performance result.
An image compiled with the switch is always diagnostic, even without an active
scope: its wrapper still contains the selected TLS/cleanup machinery. Such an
image must not be used as an unobserved performance control; CLI flags alone
cannot establish its source/image identity or instrumentation state.

When selected, binding state belongs to the calling native thread's TLS. Open
returns a generation/owner handle. It rejects nested scopes and simultaneous
legacy row recording for that owner in both activation orders, including an
aborted call whose exit is still pending. This does not establish absence
of the separate Scheme-owned remote sampler; its exclusion remains a future
VM integration obligation. A checked, lock-free image-local atomic reserves a unique
generation only at cold open. No shared generation operation occurs at progress,
wait entry or wait exit. TLS admission may involve the compiler's TLS runtime;
there is no claim of zero allocation during scope admission.

The image-wide generation is necessary: thread retirement can reuse the same
TLS address, so address plus a restarted thread-local sequence is insufficient.
Handles remain local to the loaded image lifetime; this does not solve stale
function pointers or handle reuse after unloading/reloading a different image.
The image must remain leased until all entered calls exit and cleanup finishes.

Wait entry reads checked owner wall/CPU clocks and records a stack-local ticket.
Wait exit records the actual return and saved errno. The wrapper forwards the
original descriptor sets and timeout pointer and restores the syscall errno
after all diagnostic work. A native call error is an observed error outcome,
not an invented timeout or successful service result. No payload is consumed.
The diagnostic adds clock reads **per selected wait**, never per handler; their
observer cost is not priced or claimed free. Per-call wall/CPU values contain
acquisition skew and are raw increments, not an exact per-call CPU partition;
subtracting them does not establish exact blocked time.

The pthread cleanup handler is installed around the real system call. Deferred
pthread cancellation aborts the observation, acknowledges the pending ticket
without inventing an exit timestamp, and permits a failed cold snapshot before
thread retirement. The outer test cleanup requests an ordinary finish rather
than forcing abort, so it cannot mask a missing inner cleanup. Asynchronous
cancellation and Gambit nonlocal FFI exits have **not** been validated by this
test; POSIX cleanup does not establish arbitrary Scheme continuation safety.

The binding's progress method requires its explicit handle and the original
previous/current timestamps, and reads no clock. It is **not yet wired to the
Scheme verified-progress callback**. Passing an implicit current generation to
that callback would hide stale-arm errors; the future typed integration must
carry the activation identity, preserve both original sample points, and close
the scope on actual synchronous return/unwind. The old remote sampler has not
been reactivated.

### Real native checks, without a VM

The Cargo `owner_wait_scope` test now builds a private Darwin dynamic library
containing the **actual** `owner_wait.h` wrapper with the diagnostic switch. Its
restricted export list hides `select`; the test enters through a named test
export, and `dlsym(RTLD_NEXT, "select")` must resolve a different real system
function. Legacy SDK hooks are inert in this test image. This verifies wrapper
binding in this image, not relocations in the absent frozen Gambit image.

The native driver checks:

- a real readable pipe: readiness is returned and the wrapper consumes no byte;
- zero-timeout completion, actual `EBADF`, and actual `EINTR` from a periodic
  fixture-only signal, with the syscall errno retained;
- complete same-progress-interval call counts and timeout/outcome categories;
- failed nested admission, rejected null output and idempotent cold finish;
- a real blocked pthread cancelled with the image still loaded, failed capture
  copied before TLS retirement, and joined workers before `dlclose`;
- a foreign thread rejected when finishing another thread's handle;
- thread exit/recreation with strictly increasing image-wide activation identity
  and stale-handle rejection, independent of whether TLS addresses actually recur;
- checked generation exhaustion, with the caller's output handle unchanged.

The same native test runs optimized and with ASan/UBSan. Each driver process has
an unchanged five-second correctness bound; C compilation is outside that bound.
Both must emit `OWNER-WAIT-BINDING-CONTRACT-OK` with empty stderr. Two Cargo tests
pass: the existing core contract and this real wrapper contract. Strict Clippy
passes. The model's 20 directed tests pass, including the TLS-address reuse
counterexample and cancellation that cannot become a successful observation.
The reused lifecycle model is also extended by `owner_wait_binding_modes`:
four new mode-exclusion scenarios and ten inherited lifecycle scenarios pass;
its safety invariant holds for 200 sampled traces of at most 40 steps. A negative
scenario explicitly retains the invalid overlap of the two recorders.
These checks do not initialize Scheme, build a new Gambit image, test Gerbil
Actor/GC/continuation integration, collect p99, or establish production admission.

The three C fixture sources have explicit `.gitignore` exceptions; the broad
generated-C rule had previously hidden the core fixture from an ordinary diff.
No generated object, log, trace, binary or patch is made a repository artifact.
Structured scope and source hashes are in `owner_wait_binding.json`. No commit,
push, PR #18 merge, default change or performance baseline replacement follows.

## Explicit generation through the existing Scheme progress callback

This follow-up extends the earlier C-only slice. `owner_wait_scheme_callback`
reuses the existing lifecycle model; its 13 directed checks include inherited
cases and are not 13 additional independent proofs. A stale business
continuation still runs the original heartbeat publication, but its closed
observation identity cannot write into a later activation. Explicit source-epoch
priming includes the first completion interval without changing the original
heartbeat word, deadline or clock sample points.

The opt-in `owner-wait-diagnostic` feature builds a separate private image with
`GERBIL_OWNER_WAIT_SCOPE`. Qualification operation 64 uses the same source plan,
input construction, handler, oracle, frame return and wait protocol as operation
62. Operation 65 is a correctness fixture on a real fixed owner. Normal macro
instantiations erase the new scope operations at expansion time. Comparing the
actual C wrapper translation unit with the diagnostic macro absent gives an
identical preprocessed result for the frozen and current wrapper headers. This
is not whole-image or historical-binary identity.

The existing progress callback carries the explicit generation numerically; it
does not convert a name or JSON on the progress path. Source snapshots reuse the
existing source timestamp and counters. CPU and cumulative GC cuts are sampled
only after the existing observation window has elapsed; syscall entry/exit cuts
remain separate. Observation has real cost and cannot qualify performance.
The measurement reports conservative enclosing CPU/GC bounds, not per-handler
CPU or exact OS scheduling time. A missing or invalid bracket stays unknown.

The maximum source observation retains `due`, `published`, `consumed`, `active`
and the latest progress ordinal. This separates unpublished source debt from
published work awaiting consumption. The largest progress gap and maximum
source debt are associated only if their timestamps and ordinals agree; they
are never joined merely because they came from the same run. A source check
following that progress is identified separately. Initialization, an unfinished
tail and failed/aborted scopes cannot become successful zero-wait evidence.

The native source inputs are frozen before the actual VM checks and targeted
load. No SDK, Gambit, default policy, wait protocol, five-second deadline,
capacity or workload change is part of this slice. The historical 0.317 ms and
351 > 128 remain the reference; this observed diagnostic cannot replace them.

### Actual owner validation and one targeted diagnostic

The [Scheme receipt](owner_wait_scheme.json) binds the frozen diagnostic image
and executable to these results. The real VM fixture passes on owner 0 twice,
with explicit GC, a captured original exception, three reentries of a closed
continuation while a later observation remains active, zero roots before
cleanup, and clean shutdown. The initial sandbox attempt failed opening
Gerbil's own `/dev/fd/4` stream during initialization; that failure is retained
separately from the successful unsandboxed check. All five-second limits remain.
Continuation reentry here follows observation-scope closure while the same
synchronous call still owns its borrowed progress word; it does not authorize
using that word after return or invoking Scheme after VM destruction.
The 13 C/cold-report tests and their focused strict Clippy check pass; these are
separate from the actual VM check and are not a full feature/CI qualification.

Exactly one main load ran after the original 1,000/s one-second warmup:

| Observed diagnostic metric | Result |
| --- | ---: |
| Planned / completed | 5,000,000 / 5,000,000 |
| Successful throughput | 500,000.025 messages/s |
| Rejected / timed out / failed | 0 / 0 / 0 |
| p99 / p99.9 | 0.298 / 1.634 ms |
| Maximum due but unconsumed | **1,415 > 128: failed** |
| Maximum unpublished source debt | 1,413 |
| Published but unconsumed at that peak | 2 |
| Clean shutdown | Yes |

This is observed data, not a demonstrated speedup over 0.317 ms. Its admission
receipt explicitly fails the diagnostic-observer and original-window checks.
The 128-message budget corresponds to 0.256 ms of arrivals, while the observed
peak represents 2.830 ms. The unchanged three execution frames remain bounded;
the peak is overwhelmingly unpaid source debt, not 1,415 queued payloads.

The largest progress gap is 0.838042 ms. Its enclosing owner sample has one GC
(0.806094 ms GC wall, 0.781 ms GC CPU), an owner CPU upper bound of 0.984041 ms,
and no `select` call. The non-CPU lower bound is zero, so this observation does
not establish OS descheduling or kernel blocking. GC accounting covers the
enclosing sample, not an exclusive construction/handler/consumption stage.

Crucially, that gap ends at source +0.722630 s, whereas maximum source debt is
observed at +9.288163 s. Their progress ordinals do not match. The GC event must
not be used to explain the separate backlog peak. The peak's 2.830 ms arrival
debt also exceeds the largest individual progress interval: the next analysis
must include accumulated recovery across completions, rather than only the
single longest pause. No historical root-cause closure is claimed for 351.

The project entrypoints remain Cargo targets:

```text
cargo test --release -p gerbil-scheme-qualification --features owner-wait-diagnostic --test owner_wait_vm
cargo bench -p gerbil-scheme-qualification --features owner-wait-diagnostic --bench gerbil_actor_arrival -- --owner-wait-scope --rate=500000 --seconds=10
```

The actual diagnostic used frozen copies of the already built executable and
the **same image tested by the VM fixture**; their digests are in the receipt.
These commands build current artifacts and cannot recover absent historical
binaries. No second load, candidate optimization, default change, commit, push,
or PR #18 merge followed the failed 128-message guard.

## Rejected peak-specific recovery and independent rank follow-up

The observer was implemented and tested, but its native run failed severely.
Its runtime integration and cold histogram scan have been reverted. Scheme/C
runtime sources match the preceding diagnostic's saved manifest exactly. The
collector now exists only in `tests/owner_wait_recovery.h`; the cold analysis
is imported only by tests. Neither is an active benchmark hook.

The rejected collector retained one selected source-debt episode around the
actual global outstanding peak: the last below-guard source check, the peak,
and the first check back at/below the unchanged guard. A later smaller episode
cannot replace those markers. A higher global peak invalidates the earlier
endpoint and its CPU bracket. Missing initial history or post-peak samples stays
unknown; closing an incomplete observation does not fabricate recovery.

In this test-only contract, each marker carries source counters and a progress ordinal. CPU/GC
cuts reuse the existing sparse sampling; no new clock, timer, wake, lock or
capacity update is added to the source callback. Window selection is bounded,
but has observation cost. Bounds for adjacent phases can overlap and must not
be summed as exclusive CPU/GC costs. Native wait coverage from the largest gap
is not coverage of this episode. The original window is passed explicitly;
there is no observer-owned default capacity.

For points A and B, the exact debt equation is
`outstanding(A) - outstanding(B) = consumed(B) - consumed(A) - (due(B) - due(A))`.
The recovery report preserves those integer counts and elapsed time before
presenting observed publication/consumption rates. Returning a frame does not
settle new arrivals, and an observed recovery rate is not a saturated-capacity
measurement. `owner_recovery.qnt` reuses the callback lifecycle and existing
service-headroom equation instead of introducing another Actor scheduler.

The original percentile function and per-message histogram update are unchanged
from `6d4ec27`. The rejected diagnostic added a cold full-histogram certificate
AFTER the original elapsed timestamp. It independently sums all buckets and
checks the rank using the complement identities `N-floor(N/100)` and
`N-floor(N/1000)`. For 5,000,000 successes, the p99 rank is 4,950,000 and the
p99.9 rank is 4,995,000. A 0.298 ms result means rank p99 fell in the
[0.297, 0.298) ms bucket; it is a bucket upper bound, not an exact latency.
A smaller p99 and a worse maximum/source-debt peak are logically compatible.
The synthetic distribution in the model proves that compatibility; it does
not reconstruct the preceding diagnostic's unavailable raw histogram.

### Preserved negative result and actual peak evidence

The [rejected-observer receipt](owner_peak_recovery.json) retains identities,
the selected episode, rank evidence and rollback status. Only one main load
ran; it is not a paired measurement of observation overhead.

| Rejected observed metric | Result |
| --- | ---: |
| Completed / rejected / timed out | 5,000,000 / 0 / 0 |
| Successful throughput | 464,313.165 messages/s |
| p99 / p99.9 | **770.831 / 777.597 ms: failed** |
| Observed outstanding peak | **372,198 > 128: failed** |
| Unpublished / published-unconsumed at peak | 372,196 / 2 |
| Clean shutdown | Yes; does not override performance failure |

The rank certificate sums to 5,000,000; p99 rank 4,950,000 has 4,949,985
messages below its bucket and 4,950,000 through the bucket. This severe tail is
not a percentile-rank arithmetic error in this run. This does NOT certify the
earlier 0.298 ms result: its raw histogram was not retained, and its observed
result never replaced the historical 0.317 ms baseline.

For the actual peak, the enclosing cut is 256.542 us with 256.375 us owner CPU
and zero GC. Debt had accumulated beforehand: the 9.634453 s growth interval
has 4,817,226 arrivals but only 4,445,029 consumptions, a 372,197-message deficit.
Its CPU upper bound is 6.986165 s, leaving a non-CPU lower bound of 2.648288 s;
enclosing GC wall is 0.231339 s. The available evidence does not distinguish OS
scheduling from unobserved native blocking across that whole episode, and does
not establish whether the observer caused the regression. No machine-only or
GC-only attribution is made.

Crucially, the peak occurs at source +9.730647 s, while the first return to 128
is at +10.768433 s, AFTER the last planned arrival at +9.999998 s. The reported
drain sees only 129,772.315 arrivals/s averaged over that interval and consumes
488,294.883/s. This cannot prove recovery under sustained 500,000/s arrivals.
`sourceStopDrainCannotAdmitSustainedRecoveryTest` preserves that admission
counterexample, and the cold test checks it against this captured episode.

Rejected executable/image copies and their source snapshot are isolated outside
the checkout; hash-verified rejected outputs were removed from this task's build
cache to prevent accidental reuse. The earlier frozen artifacts remain intact.
There was no second load, new baseline, default/wait/gate change, commit, push,
or PR #18 merge after the failed result. Future work must budget observation
cost and distinguish recovery while arrivals continue from finite-source drain
before claiming restored headroom.
