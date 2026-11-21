# Native owner availability: bounded, correlated observation

The acceptance target remains 500,000 planned messages/second, zero rejection,
and the historical 0.418 ms p99. Keep the original logical-window guard, five
second deadlines, original workload and negative results. Do not advance to 1M/s
or change public defaults. The historical 0.418 ms described successful messages
with 4.64288% rejection; zero rejection plus that latency is a stronger target,
not a previously established result for every arrival.

## Remove observer work from the message path

Full Scheme phase observation reduced completion throughput to 348,972.15/s and
is unsuitable for performance admission. The replacement generates the ordinary
native protocol without per-phase records or worker callbacks. One C observer
borrows the existing aligned atomic verified-completion word and retains the
fixed owner's Mach thread port. It queries kernel state only when completion
progress is older than the existing source window's service duration
(`window / rate`, 256 microseconds at the fixed 128/500k contract). There is no
new sampling configuration matrix, Rust per-message scheduler or business Actor.

Inline C remains in Scheme's `begin-foreign` / `c-lambda` module. C stores named
native facts in structs; Scheme projects named native tables/scalars after the
reader joins. The launcher reuses the existing object API and has no positional
stage/column decoder. No Scheme object or GC-managed pointer reaches the observer
thread, and the observer never writes progress. The ordinary completion operation
retains its original verification and watchdog checks.

The existing private-image `select` wrapper marks native entry/exit with an atomic
epoch while preserving arguments, return status and errno. An unchanged odd
epoch identifies samples inside that region; waiting samples there support a
native-select wait, while runnable samples do not prove a blocking select.
Short blocks between samples remain unobserved.

The reader is stopped and joined before the borrowed progress word or thread port
is released, including exceptional unwind. Live report reads are rejected.
Existing QNT lifecycle/frame models are reused; negative cases detect returning
before join, reading a live report and returning with an elevated priority scope.

## Avoid a remote CPU-counter attribution error

Mach `TH_STATE_RUNNING` represents `TH_RUN` (runnable/running), not proof that a
thread is currently executing. Furthermore, remote cumulative CPU can omit an
in-flight interval: XNU's `thread_read_times` reads lifetime recount data, whereas
current-thread accounting adds time since the last snapshot. Source inspection:
[thread.c](https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/kern/thread.c),
[recount.c](https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/kern/recount.c).
These references explain the interpretation; they are not a pin of the host's
installed kernel. Therefore a flat remote CPU count alone is not evidence of
descheduling. The initial interpretation of the 26.40 ms sampled gap was too strong.

Owner self CPU/GC reads are now merged into the existing verified-completion
transaction: a fast predicate runs per completion, and actual reads occur once
per source-window duration or at the end of a witnessed gap. The observer reads
only the atomic word/kernel port; it never reads mutable VM GC fields. The owner
records its own CPU/GC with before/after clock brackets. Sampling uses bounded
native records, not another queue. No fabricated heartbeat or capacity release
is introduced.

Classify a gap only when the completion-start keys agree and the self CPU window
brackets that exact gap. QNT retains mismatched-window and remote-only negative
cases. Observer sample spacing and maximum poll spacing are reported, so a blind
interval cannot be silently called nonblocking.

## Matched native observation

The corrected self-clock run completed all 5,000,000 messages at 500,000.03/s,
zero rejection/timeouts/errors, with clean shutdown. Its p99 was 96.709 ms and
peak pending source debt 66,133: it did not pass acceptance.

One paired completion gap lasted 22.967292 ms. The enclosing owner self-clock
window consumed 0.378666 ms CPU and recorded zero GC increments or GC time.
Nine kernel samples were runnable, with no sampled select region or kernel
waiting. This supports runnable off-CPU delay at those sampled points. Maximum
within-gap sample spacing was 7.935875 ms and maximum global polling gap was
21.112375 ms, so it does not prove every instant's state or explain all tails.
The original roughly 48k messages were unmaterialized source debt, not payloads
inside worker mailboxes; 500k/s times the original 95.966 ms publication lag is
consistent with that debt. Larger mailboxes cannot eliminate missing owner time.

## Native control type contract and retained failures

The first scoped-QoS experiment failed before the candidate ran. Its launcher
used unconstrained integer literals for `numeric_vector`, producing S32 control
vectors. The native order-control entry requires S64, so it reached the batch
envelope rejection instead of the QoS operation. Explicit `numeric_vector::<i64>`
fixes the actual mismatch. Wrong-type failure records are retained.

A suspected constructor-evaluation-order issue was tested separately. The same
typed launcher passed the restoration contract with both the original image and
an image that explicitly sequenced the mutation before receipt construction.
That order issue was not reproduced on this SDK and is not claimed as the
failure's cause; the unnecessary native change was removed from the candidate.

The live error-scope contract now reads `[0,25,21]` at admission and `[0,21,21]`
after restoration, then closes cleanly. It preserves the original assertions.
The supported scope uses the existing qualification QoS operation, including
its saved relative priority; no production default changes. Its work-error
restoration path was executed. Rust panic restoration is implemented but was
not separately exercised by a native panic-injection test.

## One targeted same-VM QoS comparison

Each arm used the exact same VM/image, four native workers, three active frames,
the same fresh four-record inputs/full oracle and five million arrivals over ten
seconds. Both retained the lightweight observer. Scope error restoration passed
before the trials. User-initiated QoS is an explicit experimental native scope,
restored after each call, not a replacement default or real-time CPU guarantee.

| Metric | Default scope (class 21) | User-initiated scope (class 25) |
| --- | ---: | ---: |
| Completed / rejected | 5,000,000 / 0 | 5,000,000 / 0 |
| Completion throughput/s | 500,000.02 | 500,000.02 |
| p99 | 17.781 ms | 0.382 ms |
| p99.9 | 28.439 ms | 1.289 ms |
| Peak pending source messages | 16,211 | 1,027 |
| Maximum publication lag | 32.421083 ms | 2.053416 ms |
| Matched completion gap | 20.248333 ms | 0.945375 ms |
| Owner CPU in paired window | 0.341791 ms | 0.958375 ms |
| GC count in paired window | 0 | 1 |
| Kernel runnable / waiting samples | 53 / 0 | 2 / 0 |
| Select-region samples | 1, none kernel-waiting | 0 |

Errors/timeouts are zero, shutdown clean, and the scope restores class 21. The
observed p99 benefit supports the owner-availability hypothesis. The remaining
captured candidate gap has actual CPU and one GC; do not conflate it with the
default scope's off-CPU gap. It does not prove that every remaining tail is GC.
Both logical-window guards still fail. No candidate is accepted or submitted.

## Frozen evidence

Local evidence lives under `target/replay-native-mailbox-20261027/`; source
snapshots and raw logs are not uploaded. Base revision alone does not identify
these uncommitted experiments.

| Artifact directory | Executable SHA-256 | Image SHA-256 |
| --- | --- | --- |
| `native-owner-self-clock` | `d1f17c4ac2e0b89cf694b7080e0affd22afed8681e7dac0cf73be3651a306a23` | `e777ad5799caa1adcc9958b8e470715e0c25cd6e67369a6fa13c71fbae501fde` |
| `native-qos-causal` | `c27a19f2ffaeaf7067e0fa44f2ce4f98605b26c54b5d56a1be0d2cf40edef744` | `e777ad5799caa1adcc9958b8e470715e0c25cd6e67369a6fa13c71fbae501fde` |

The latter source manifest is
`da1fa3cd1322cd86db3de26669a15a0163e21ac8201d6eed7047e0db7f3b1c3b`.
The prior raw-only observer, failed QoS attempt, wrong-type contract audit and
both correctly typed contract controls remain beside these directories. No
passing receipt replaces a retained failure. Unobserved confirmation is separate
from these diagnostic results; keep throughput, p99 and transient debt together.

## Unobserved confirmation: priority candidate rejected

The follow-up used one live VM and the exact same frozen native image, with both
the kernel observer and phase observer disabled. It retained all workload,
timestamps, verification, deadlines and capacity settings. The explicit scope's
type/error/restoration contract passed before the experiment.

| Metric | Default scope | User-initiated scope |
| --- | ---: | ---: |
| Completed / rejected | 5,000,000 / 0 | 5,000,000 / 0 |
| Throughput/s, including drain | 495,198.69 | 500,000.03 |
| p99 | 1,259.837 ms | 51.452 ms |
| p99.9 | 1,274.164 ms | 72.688 ms |
| Peak pending source messages | 638,512 | 38,342 |
| Final drain | 96.959333 ms | 0.001459 ms |

Both arms have zero accepted timeout/native error and clean shutdown. The
unobserved control is itself severely delayed and cannot establish improvement
against the historical baseline. The candidate's absolute p99 misses 0.418 ms;
the observed 0.382 ms is not an admitted performance result. No repeat matrix
was run to select a favorable number. All flags remain opt-in.

The kernel accessor itself is not fully passive. The inspected XNU
`retrieve_thread_basic_info` path may run `SCHED(update_priority)` while reading
thread data. This is a specific possible probe-effect mechanism, not proof that
it alone caused this experiment's difference. The alternative proc-thread-info
path also calls `thread_info_internal` in the inspected
[bsd_kern.c](https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/kern/bsd_kern.c),
so switching to that wrapper is not evidence of a passive observation path.
Do not claim the original unobserved long tail has been fully attributed.

Frozen directory: `native-qos-unobserved`. Source manifest SHA-256:
`55a3282e31a4f4a4acbb8d4d9e55ed18a0962e0016baec8e2e396287d1db68f9`.
Executable SHA-256:
`32377c9fce56369359c9199728ed6aff16986d6c3ab959ecc3da85644c18520d`.
Native image remains `e777ad5799caa1adcc9958b8e470715e0c25cd6e67369a6fa13c71fbae501fde`.

## Due-time source wait: targeted protocol hypothesis rejected

Source inspection found a separate eligibility gap: when no request is active
and the next planned arrival is in the future, the original driver repeatedly
calls `thread-yield!`. The sparse 1k/s warmup therefore remains runnable even
when no application work is available. This is a source-driver issue, not proof
that production Gerbil Actor mailboxes busy-spin or that it caused the tail.

An opt-in Scheme variant uses existing `thread-sleep!` only with zero active
work and a positive remaining delay to the original next arrival. Due work is
supplied immediately, all original timestamps remain, and no queue, threshold,
batch size, requested priority or watchdog changes. No scheduler Actor or Rust
timer is added. `native_source_wait.qnt` refines the existing frame/source model;
negative cases cover sleeping with live work, a lost arrival wake and safe but
wasteful busy yield. Ten scenarios and 1,000 bounded traces pass. These do not
prove timer precision or a numeric latency target.

The unobserved same-VM pair, both at default priority, retained five million
planned arrivals per arm:

| Metric | Original source yield | Due-time source wait |
| --- | ---: | ---: |
| Completed / rejected | 5,000,000 / 0 | 5,000,000 / 0 |
| Throughput/s | 500,000.02 | 499,999.71 |
| p99 | 33.972 ms | 75.237 ms |
| p99.9 | 50.092 ms | 92.668 ms |
| Peak pending source messages | 26,511 | 48,341 |

Errors/timeouts are zero and cleanup is clean. The wait candidate fails the
historical guard and is not promoted. Timing granularity, wakeup costs and host
scheduling remain possible explanations; no additional run was used to assert
one without evidence. It is not an engine optimization success.

Frozen directory: `native-source-park`. Source manifest SHA-256:
`337d506e2b3481090cae092a2ce0cdd3509341dc05ac4a554f6d5ee338602d72`.
Executable SHA-256:
`dd9d2e086142368f733cbe134165558dea0b3e82017367a1d27976519d38a238`.
Image SHA-256:
`6fccfee5720dfda181de607e0a54d1ce2a6b20f023aa4ab95db49acbc74c9378`.
The original source and default dispatch stay selected. No candidate has met the
full 500k/s zero-rejection, p99 and bounded-debt requirements; no 1M/s experiment,
commit or push followed. The remaining causal boundary is unobserved owner
availability with enough coverage to separate runnable delay from native blocks,
without relying on a scheduler-mutating observer or weakening the benchmark.

## Kernel event capture: distinguish preemption from native blocking

The earlier Instruments failure was a sandbox cache-write failure. A bounded,
unsandboxed System Trace capability check succeeded. One attached trace then
recorded the ordinary default-priority native-access path with both custom
observers disabled. No per-message marker was added: the launcher calls the
existing native `OwnerClock` marker before and after an arm and reads its native
thread ID through the existing signed-64-bit command interface. The initial
stdin gate is before VM initialization and the watchdog. It only allows the
recorder to attach; it does not suspend another task or alter arrival timing.

This is diagnostic evidence, not performance admission. Instruments remains an
external observer. A 1.34-second Clippy check inadvertently overlapped part of
the measurement; it finished successfully, but the overlap is retained rather
than calling the host quiet. No clean-host or unobserved performance claim is
made from this run. The System Trace table also contains system context-switch
metadata despite the process attachment; analysis selects only owner thread
8649915 in the recorded benchmark process.

The final `OwnerClock` event is at trace time 9,905,801,083 ns and native time
69,625,474,826,916 ns. It gives the clock origin 69,615,569,025,833 ns. The
measured-call before bracket is 69,615,463,838,333..69,615,463,840,916 ns.
Thus native execution is covered from approximately 105.188 ms after entry to
the ending marker, not from the beginning of warmup. Missing coverage is not
silently classified. The table below clips every owner interval to that covered
measurement window and excludes teardown.

| Kernel owner state | Total | Longest interval |
| --- | ---: | ---: |
| Running | 9,577.696 ms | 1.004 ms |
| Preempted | 208.304 ms | **21.249084 ms** |
| Runnable | 6.426 ms | 4.181417 ms |
| Interrupted | 111.083 ms | 0.024458 ms |
| Blocked | **0.105794 ms** | **0.034250 ms** |
| Unknown at capture start | 2.186541 ms | 2.186541 ms |

The longest preempted interval starts at trace time 1,391,361,041 ns. Its event
narrative explicitly reports the owner forced off an E core by a thread with
priority 31 while the owner's priority is 28. This is concrete OS scheduling
evidence for that interval, rather than an inference from remote CPU counters.
A separate 89.316875 ms blocked interval starts at 9,905,879,375 ns, **after**
the final measurement marker, and is excluded. Counting that teardown wait as
native request blocking would invert the conclusion.

The full run completes 5,000,000/5,000,000 messages at 500,000.02/s, zero
rejection, errors and timeouts, with clean shutdown. However p99 is 44.021 ms,
p99.9 is 74.733 ms, peak source debt is 39,426 and the original window guard
fails. Neither zero rejection nor average throughput establishes acceptance.
This trace identifies a long owner preemption in the same run as the tail; it
does not associate a particular request ordinal with that preemption or explain
all running-time/GC costs contributing to the p99.

The protocol consequence is to keep OS owner availability separate from Gerbil
Actor readiness and native wait eligibility. At 500k/s, 21.249084 ms without an
owner permits roughly 10,624 planned arrivals even if mailbox execution itself
is instantaneous after resumption. More Actors, a larger queue or an extra
reclaim Actor cannot provide CPU during that interval. A latency guarantee must
include an owner-availability assumption as well as a service-rate bound;
backpressure alone cannot guarantee both zero rejection and a fixed planned-time
p99 during unbounded off-CPU intervals. This does not justify changing deadlines,
arrival timestamps or rejecting late work to improve the histogram.

`native_owner_availability` in `native_source_wait.qnt` reuses existing frame/root
custody and separates external arrival, OS CPU availability and native blocking.
Its negative service scenarios retain safe ownership with growing source debt
while the owner is preempted. Actor readiness cannot manufacture OS execution.
The model is a boundary explanation, not a numeric OS fairness proof.

The measured Scheme protocol remains unchanged. Previous priority and timed-park
candidates remain unaccepted; this evidence does not promote either. No further
candidate or numerical matrix was added. The unresolved engineering requirement
is an owner lifecycle/CPU-service contract whose unobserved behavior meets the
existing gate, followed by any measured running-time construction/GC work. Do not
attempt to solve a demonstrated off-CPU interval by adding Scheme-side polling.

Frozen directory: `kernel-owner-trace`. Executable SHA-256:
`c735b1f5a9739b54ae2b879a088de88ef00f230f2e840184d1cc2cda47c6645d`.
Image SHA-256 remains
`6fccfee5720dfda181de607e0a54d1ce2a6b20f023aa4ab95db49acbc74c9378`.
The raw trace, native receipts, exported tables and owner-only interval summaries
stay in ignored local artifacts. No 1M/s trial, default change, commit or push.

## Next step: a phase-independent scheduling floor and disabled-hook removal

No new load run is needed to establish a conservative scheduling-only floor for
the existing trace. Reuse the original 2,000 ns arrival period and disjoint owner
`Preempted`/`Runnable` intervals. Even if every message completes instantly when
the owner resumes, a gap of length `D` contributes at least
`floor(max(D - L, 0) / period)` arrivals delayed beyond limit `L`, independently
of the unknown phase of the periodic source. Count only intervals proven inside
the original source schedule; never equate a call marker with source startup.

The existing receipt and before/after call brackets bound the source start above
by 69,615,474,827,291 ns and its scheduled end below by
69,625,463,836,333 ns. The ending marker brackets the clock origin in
69,615,569,025,833..69,615,569,028,542 ns. The common guaranteed coverage is
therefore trace time 0..9,894,807,791 ns. This also excludes teardown and unknown
initial owner state. Independent XML selection confirms all 60,717 owner rows
and 1,155 preempted rows before clipping.

Summing the conservative floor across the 1,170 covered off-CPU intervals gives
at least **88,538 arrivals (1.77076% of five million) beyond 0.418 ms**. Inverting
the same count gives a **2.923875 ms p99 lower bound** with ideal zero-cost
execution. This is a counterfactual lower bound for that recorded run, not a new
benchmark result, an unobserved-host guarantee, or a proof that all 44.021 ms is
OS delay. The trace and Clippy overlap remain material qualifications. The first
analysis that treated the before-call marker as source startup is retained as
`owner-latency-floor-anchor-assumption.json` and is superseded by the
phase-independent `owner-latency-floor.json`.

The existing `native_owner_availability` model now checks this counting bound
against every phase in a finite parameter range. It includes zero-cost execution
missing a deadline, a sub-deadline gap that cannot establish failure, and an
odd-length interval where using a ceiling would overstate the count. Thirteen
scenarios pass. These checks validate arithmetic and assumptions, not OS fairness.

The same trace contains 9,694 owner CPU samples, 6,135 with a top frame of
`mach_absolute_time`. The native image disassembly resolves its caller to the
existing source-clock operation. These are sample counts, not an exact CPU-time
share: the 1 ms sampling cadence and 2 microsecond periodic arrivals can alias,
and idle source pacing also reads the clock. Do not infer that business execution
spends 63% of its time reading clocks, or replace the precise clock with an
approximate clock to improve the histogram. CPU samples are not sufficient to
justify rewriting the order handler or changing source pacing.

One source-grounded observer cost is safe to remove independently of those sample
percentages: every ordinary verified completion previously called the disabled
diagnostic hook, whose guard accesses compiler-emulated TLS. The C bridge now
holds an optional atomic callback pointer. The fixed owner binds it only after
successful observer-thread creation and detaches it before stop/join releases
the borrowed progress word. The disabled path takes a null-pointer check and
does not call the diagnostic C unit or TLS accessor. The original wall-clock
read, five-second check and atomic verified-progress publication are unchanged.
Scheme business execution, the independent result oracle, Actor scheduling,
source timing, custody and capacity are unchanged.

`native_progress_binding` refines the existing observer lifecycle model. It
covers failed startup leaving the hook detached, ordinary progress without an
observer, detach-before-join, a retained-hook negative case and rebind after full
retirement on the same owner. Eight scenarios and 1,000 traces of 50 steps pass.
The underlying observer's nine scenarios also pass after adding explicit
next-call retirement. This removes an introduced diagnostic cost;
it is not a claim that the scheduling floor or historical p99 has been solved.

The first implementation put the new binder definition in the owner-only C
adapter. The standard probe image also links the Scheme observer unit, so its
link failed with an undefined binder symbol. The failed build is retained. The
corrected definition and callback slot live in the existing Scheme inline-C
bridge; the owner adapter reads the image-local slot. This preserves the common
image build path and does not add a probe-specific shim or production Rust API.

The corrected standard Cargo native-image build passes. The same frozen Rust
launcher binary is retained, SHA-256
`c735b1f5a9739b54ae2b879a088de88ef00f230f2e840184d1cc2cda47c6645d`;
the new native image is
`d6b8178100d5cd4d33407e15ce291a1959921391f42ec4eff93ae14684312031`.
Two successive observer-enabled calls on the same owner, each with 1,000
original messages, complete without rejection/error/timeout and clean shutdown.
These sparse calls validate lifecycle reuse; they are not the 500k/s performance
gate. Native disassembly confirms the null callback branch bypasses the
diagnostic hook after the original five-second check and release-store heartbeat.

### Targeted 500k/s pair: no admission

Strict release Clippy with warnings denied passes. After the local build and
model checks ended, exactly one pair ran with both observers and Instruments
off. Both arms use the identical frozen launcher, one owner, four resident
workers/three active credits, original 128-window guard, ten seconds and five
million planned arrivals. The control uses the retained `6fcc...` image and the
candidate uses `d6b8...`; each initializes its own VM with the same configuration.
No other task was paused and no quiet-host assumption is made.

| Metric | Original image control | Disabled-hook candidate |
| --- | ---: | ---: |
| Measured-arm outcome | Original five-second request deadline failed | Completed |
| Completed messages | No final receipt | 5,000,000 |
| Completed/s | Unavailable | 500,000.01 |
| Rejected / native errors / accepted timeouts | No final receipt | 0 / 0 / 0 |
| p99 | Unavailable | **107.832 ms** |
| p99.9 | Unavailable | 142.005 ms |
| Peak source debt | Unavailable | **71,547** |
| Original window guard | No final receipt | Failed |
| Final cleanup receipt | Absent after failure | Clean |

The control fails in native consumption with `native request exceeded five
seconds`, not a loosened client timer. Do not infer partial completion counts,
throughput or clean shutdown from its successful sparse warmup. The failure
does not by itself establish OS preemption, CPU overload, GC or a wait cycle as
its cause. The candidate misses the absolute historical gate, and the failed
control makes a relative performance comparison invalid. No retry or larger
matrix followed; no improvement percentage is reported.

The candidate source/image and failure records are frozen locally under
`native-progress-binding`. The three native-source edits for callback binding
are removed from the active research source after freezing. The existing
observer implementation remains the source baseline; retained models and this
analysis do not activate an unaccepted hot-path candidate. Cargo must rebuild
that restored source before any future run using its generated default image.
This does not claim the earlier observer cost is free; it prevents an unaccepted
cleanup candidate becoming a new performance baseline. No commit or push.

## Admission and recovery contract after rejecting the candidate

The native source remains byte-identical to the frozen control. The next change
is confined to post-run qualification: a receipt can report successful execution
and clean cleanup while failing every performance requirement. Previously the
launcher printed the outstanding-window failure but still exited normally.
It now emits a separate `performance_admission` object with all failed conditions,
without adding work inside the timed native call or any per-message Rust entry.

The fixed guard requires 500k/s for ten seconds, five million completed planned
messages, zero rejection, a positive planned-to-consumed p99 no greater than
418,000 ns and a measured maximum due-but-unconsumed count no greater than 128.
This last count includes unmaterialized source debt, not just native frame roots.
Warmups, observed runs and missing debt coverage do not qualify. Known trace-gate
and clock-anchor modes are also excluded; the launcher cannot detect every
external profiler, so source/binary/image identity and environment review remain
separate evidence obligations. A native exception has no final receipt and must
never be assigned a synthetic successful throughput or p99.

The opt-in `--require-native-acceptance` flag accepts only a single unobserved
`--backpressure-native-access --rate=500000 --seconds=10` arm, not a comparison or
trace mode. Invalid scope is rejected before pool initialization. The final
performance assertion runs **after** owner cleanup; it does not abort an active
VM merely because a valid receipt misses the performance gate. Default execution
flags and runtime scheduling are unchanged. The JSON receipt gives a numerical
guard verdict, not lifecycle, continuation, portability or production acceptance.

Five Rust unit tests cover the original inclusive limits, the actual 107.832 ms
candidate, the favorable-but-observed 0.382 ms result, missing receipts/warmup and
successful-only latency hiding rejection. The tests are in the existing
qualification crate and run with Cargo, without Scheme initialization, Python or
a standalone driver. They test the admission evaluator, not engine speed.

`native_service_recovery` reuses the same exact-unit model file to state another
necessary boundary. After a stall `D`, an ideal constant service rate `mu` needs
at least `ceil(D * lambda / (mu - lambda))` to clear the debt if `mu > lambda`.
For `mu <= lambda`, no finite catch-up is guaranteed. The illustrative 500k/s
arrival / 600k/s service case needs 106.245420 ms after a 21.249084 ms stall; the
1M/s service case needs 21.249084 ms. These service rates are hypotheses, **not
measured capacity**. Four model scenarios pass. Increasing the queue does not
enter the equation, and zero rejection does not imply bounded delay.

This selects the next algorithmic requirement rather than another threshold:
an owner service-availability contract and a measured recovery-rate reserve must
both be established before a native scheduling candidate can plausibly meet the
fixed target. The existing trace's clock-heavy samples cannot establish that
reserve because they include idle pacing and possible sample aliasing. No new
load run, priority change, enlarged queue, default change or performance claim
is introduced by this admission/model work.

Validation: the five Cargo library tests pass without features or VM startup;
the release bench builds through the existing native-image pipeline; strict
release Clippy passes with warnings denied. Two actual executable negative
checks (one-second duration and comparison mode) use a deliberately nonexistent
image path. Both exit 101 with the strict-scope diagnostic before image loading,
with empty stdout and no measured-arm/cleanup receipt. These are CLI admission
checks, not load runs. Logs remain local under `native-admission-contract`.
