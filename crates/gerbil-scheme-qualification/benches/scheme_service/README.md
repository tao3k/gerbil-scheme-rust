# Scheme-owned asynchronous service benchmark

Run from the repository root in the admitted native SDK/Cargo environment:

```text
just bridge-scheme-service-benchmark --test
just bridge-scheme-service-benchmark
```

This is one directed synchronous/service pair, not a scheduler matrix. Both arms
use the same four Scheme VMs and image, two Tokio workers, four request slots and
32 work units per owner, batch eight, quantum one, the same numeric constructor,
Scheme order handler and bounded projector. Both enable the service/wake
infrastructure. Defaults are unchanged.

The full run alternates AB/BA for ten samples per arm/load: 8,192 48-line orders
and 64 four-line probes on a nominal 500 microsecond schedule. Probes are serial;
a delayed probe delays subsequent actual arrivals. The mixed workload retains
real Scheme waits. It measures completion of accepted work, not offered QPS.

JSON events report configuration/startup, individual samples, aggregate results
and native cleanup. Throughput uses median native bulk completion duration; p99
pools the corresponding observations. Probe actual latency starts before
admission; scheduled latency retains arrival delay. Pending-before-projection
includes Scheme execution, waits and interleaving, and is not isolated CPU time.
Per-owner completion maxima, rather than submission order, define finish spread
and progress at first owner finish. Actor peer minima are observed progress,
not a proof of starvation freedom.

The five-second progress gate is unchanged. Correct results, stable VM identities,
returned credits, successful Scheme quiescence, four joins, zero external roots
and removed copied image files are required. Keep raw logs, frozen binaries and
source archives outside Git. See the repository status report
`docs/30-39-status/30.33-scheme-owned-service-and-native-wake.org` for matched
measurements and negative results. This is not an old-default Python comparison
or a claim that every CPU workload benefits.

## Explicit short layout and completion-ready comparison

```text
just bridge-scheme-service-benchmark --test --split-layout-ab
just bridge-scheme-service-benchmark --split-layout-ab
```

Both arms are asynchronous Scheme services on the same VMs. The control keeps
single-element batch probes and FIFO client completion observation. The candidate
uses a tagged native order/result layout for probes and the actual opt-in
`owner_flow_ready_backpressured` API for bulk completion observation. This is a
combined pipeline comparison, not independent attribution of each component.
Both arms impose the same explicit client bulk limit of three callbacks per
owner through projection, retaining the existing four native slots/work 32.
This leaves capacity for the one probe producer in this workload. It is not a
pool-wide reserved class quota against arbitrary independent producers.

The application's tag/version `[77,1]` is disjoint from every valid one-element
batch quantum header. It accepts at most four order lines and returns one native
numeric vector. Larger work is rejected at that entry; the full generic service,
batch Actor, evaluator and continuation capabilities remain available. Scheme
selects and executes the service worker; Rust does not execute the order logic.
Native short construction, pending and projection spans are retained separately.
The pool still shares Scheme workers and GC: this layout is not preemption or
complete latency isolation from CPU-bound non-yielding handlers.

The combined pipeline is not automatically admitted for performance. To isolate
short layout while retaining FIFO client observation and the same three-callback
bulk cap in both arms:

```text
just bridge-scheme-service-benchmark --test --short-layout-only
just bridge-scheme-service-benchmark --short-layout-only
```

This targeted follow-up addresses the mixed-throughput regression observed in
the combined candidate; it adds no VM/thread matrix. Keep both comparisons and
all negative captures under their own source identities.

## Classified route algorithms

```text
just bridge-scheme-service-benchmark --test --class-route-ab
just bridge-scheme-service-benchmark --class-route-ab
```

This pair keeps native short layout and FIFO completion identical. The fixed
control uses the same three-batch/one-short bound as the candidate's actual
`SchemeServiceRouter`; only the candidate selects destinations through the
explicit short queue-pressure and batch declared-work algorithms. Owner clocks
come from `NativeSession::owner_index`, with normalized progress and actual
assigned totals retained. Configuration is opt-in and shared by router clones.
Independent routers/unclassified native callers do not share its class quotas.

The current route heuristic regresses mixed throughput and short p99 despite
better normalized owner progress. It is experimental, not a default promotion.
See `docs/30-39-status/30.35-class-specific-native-service-routing.org` for the
actual routing API, native quota/cancellation/close checks and complete negatives.
Cross-instance routing and Scheme-owned short-queue scheduling are separate
problems; this comparison does not implement or qualify the latter.

## Scheme short scheduling only

```text
just bridge-scheme-service-benchmark --test --short-service-ab
just bridge-scheme-service-benchmark --short-service-ab
```

Both arms use the same four VMs, two Tokio workers, class router (one short / three
batch callbacks), native short layout, batch eight, work budget 32 and FIFO client
collection. Only the candidate opts into `SchemeServiceRequest::short_service`.
The service rejects this declaration unless it has one work unit, no Actor lane,
and an endpoint negotiating ABI v2. ABI v1 remains usable for general requests.

Scheme owns separate short work and completion queues, alternating eligible
short/general work on each persistent worker and returning at most one completion
of each class per owner turn while short work remains outstanding. This is a
cooperative work budget, not a wall-clock guarantee for arbitrary Scheme code.
An executing handler is not interrupted or moved to another thread.

`probe_post_admission_wait_ns` measures the interval from the admission future's
return to native construction entry, saturated at zero when execution starts
before the future returns. It excludes the measured admission interval, but still
combines host queueing, wake delivery and OS scheduling. Do not subtract aggregate
p99 values or label this interval isolated Scheme execution or wake latency.
See status report 30.36 for native acceptance and retained negative A/B evidence.


## Shared-resource refinement and bounded input feeding

```text
just bridge-scheme-service-benchmark --feed-ab
just bridge-scheme-service-benchmark --feed-ab --phase-observation
```

This is one additional directed pair, without a VM/thread matrix. Both arms use
explicit Scheme short service, identical class routing, native short layout,
FIFO client observation, four VMs and the same 4/32 slot/work bounds. Only the
per-owner native feed bound changes from one to four. The opt-in API publishes
up to that many immediately available FIFO service inputs before advancing
Scheme; it never waits to fill a batch. Defaults stay at one. Accepted permits
remain held until native completion/projection.

`--phase-observation` enables owner-side counters for advance, native wait,
submit/construction, take/projection/retirement and resident supply. Samples carry
per-owner wall,
SDK thread CPU and GC deltas, plus native diagnostic snapshots around each short
constructor. Without this flag, no phase clock or native counter reads are added.
Snapshot APIs themselves use the existing owner queue; treat observed runs as
diagnostics, separately from unobserved throughput admission. Wall/CPU/GC intervals
overlap and cannot be summed or subtracted into isolated OS scheduling cost.

The existing `specs/owner_lifecycle.qnt` refinement shows why reserved short worker
capacity is insufficient: native entry, VM CPU, GC and OS progress remain shared.
Its candidate cooperative CPU policy is model-only; bounded native feeding does
not implement it or preempt arbitrary Scheme handlers. See status report 30.37
for the model-to-code mapping, native gates and retained directed results.


## Scheme cooperative unit/class policy only

```text
just bridge-scheme-service-benchmark --test --budget-ab
just bridge-scheme-service-benchmark --budget-ab
```

Both arms use the same four VMs, native O2 image, two Tokio workers, explicit short
service/router/layout, native feed one, slots/work 4/32, batch eight and FIFO
collection. Only the Scheme cooperative budget changes: disabled versus one
consecutive unit while both classes wait. The order handler defines one order as
a unit and releases its lease around the same real one-ms wait. Actor children
receive the same request class; Rust only configures and dispatches.

Each sample checks exact native participation: zero grants in the disabled arm;
64 short grants and one bulk grant per order plus one reacquisition per real wait
in the full candidate. All queues/leases must return idle. Cumulative native
class-bypass maxima must not exceed one. Those are fairness observations among
registered cooperative unit waiters; host entry, uncooperative handlers, GC and
OS scheduling are separate and can still dominate request p99. State/continuation
ownership and existing output/progress/cleanup checks remain required. Keep the
negative captures and source identities; see status report 30.38.

## Owner entry and native port wake

```text
just bridge-scheme-service-benchmark --budget-ab --phase-observation
just bridge-scheme-service-benchmark --ingress-ab
```

The diagnostic observer pairs caller/owner timestamps for class readiness,
native capacity, publication, wake write, dequeue, owner turn and construction.
Only explicitly observed requests read these clocks. A bounded 256-interval
owner history reports overlap with completed native phases and flags lost
history. Full enclosing CPU/GC counters are not clipped to request overlap.
The native select trace uses a separately calibrated monotonic clock; its
calibration uncertainty is retained. Observed runs diagnose costs and are not
unobserved performance acceptance.

The ingress pair uses budget one in both arms, identical native work, short
queues, class routing, FIFO collection and feed one. Its candidate explicitly
selects `SchemeServiceWake::NativePort`: Scheme directly waits on the existing
input port for host input and Scheme completion notifications. It also omits
empty completion collection when no host service jobs remain. These are coupled
protocol changes, not independently attributed speedups. The watcher bridge
remains the default; mode changes require an idle service and join its old
reader before transferring ownership. Gambit retains IO, timers, Actors,
continuations and green-thread scheduling. No Rust green scheduler is added.

## Independent notification comparison

```text
just bridge-scheme-service-benchmark --notification-ab
just bridge-scheme-service-benchmark --notification-ab --phase-observation
```

Both arms retain native port wake, budget one, the same class route, four VMs,
two Tokio workers, workload and FIFO collection. Only the candidate opts into
`SchemeServiceNotification::ShortIndependent` through the actual dispatcher API.
Each explicitly declared short-service publication makes its own nonblocking atomic byte write, bypassing the
legacy coalescing claim and writer mutex. Full pipes already contain a wake;
errors remain failures. Coalesced notification remains the default. Bulk publication retains coalescing and close writes independently. The change
does not alter Scheme scheduling, native compilation, timers or continuations.

Observed candidate probes must have real Written/Full notification receipts.
Paired owner history now retains enclosing GC CPU as well as GC wall time; old
captures lacking this field have unknown GC CPU, not zero. These clocks still
do not isolate kernel wake delivery from runnable-thread scheduling delay.

## Capacity-reserved owner reselection

`just bridge-scheme-service-benchmark --reselection-ab` compares pinned admission
with explicit `SchemeRoutingAdmission::Reselect` on the same four native VMs and
two Tokio workers. Both arms use native short execution, class routing, FIFO
result observation, feed one, cooperative budget one, native port wake and
coalesced notification. Only the admission strategy changes; no default changes.
The same existing CPU/mixed corpus, 8,192 orders, batch eight and 64 probes with
nominal serial 500-us releases run in AB/BA order (ten samples per arm/load).
`owner_reselection` records the strategy in each sample. The opt-in versioned
capacity notification is initialized during warmup and remains enabled in both
timed arms, avoiding an asymmetric pool activation cost.

The native service fixture separately checks direct capacity release on B while
A remains blocked, FIFO follower progress, declaration reads, cancellation,
timeout, native capacity-epoch invalidation and router/pool close conservation. Successful model or smoke checks are
not throughput/p99 acceptance. Keep raw captures outside the repository and
retain negative and incomplete results under distinct source identities.


## Owner OS scheduling diagnostic

`just bridge-scheme-service-benchmark --owner-qos-ab` selects one exclusive
pair on the same four native VMs and two Tokio workers. Both arms use capacity
reselection, native short execution, native port wake, cooperative budget one,
feed one, coalesced notifications, and identical request/work/class bounds.
Only the fixed owners' requested Darwin OS class changes: original versus
`USER_INITIATED`. The existing qualification image control saves each owner's
original class and relative priority and restores it after every sample.
Application and restoration return codes and requested classes are asserted;
an unspecified original class is rejected. No library default is changed.

Add `--phase-observation` for matching request-entry, owner-phase and native
select observations in both arms. This observer perturbs execution and is not
interchangeable with an unobserved capture. A successful pipe write proves
neither immediate owner execution nor which descriptor woke a native select.
QoS is an experimental OS intervention, not a Scheme scheduling algorithm or
a portable latency guarantee. Preserve throughput, short/batch tails and owner
fairness regressions. `--test --owner-qos-ab` is a correctness smoke only.


## Bounded owner-resident supply

`just bridge-scheme-service-benchmark --resident-ab` compares the existing host
refill with `submit_scheme_service_stream_on` on the same four Scheme VMs and
two Tokio workers. Both arms retain the same fixed bulk partition, three active
bulk callbacks per owner, native short/reselection probe path, native the explicit native wake policy,
budget one, feed one, coalesced notifications, slots four and work 32. No owner
OS priority changes. Candidate staging is 32 original batches/owner, 256 declared
work units/owner, at most 128 staged callbacks; control retains the existing
128-ticket flow ceiling. These are distinct bounds, not an equal-memory claim.
Each original input, native constructor, Scheme handler and projection is kept.

`--test --resident-ab` is an exclusive correctness smoke. Add
`--phase-observation` for matching native phase/entry diagnostics in both arms;
these measurements are distinct from unobserved performance. `owner_resident_supply`
identifies the candidate. Shutdown must join all four owners, balance roots and
remove copied files. Preserve throughput, short/batch p99, scheduled-probe,
overlap and owner/Actor fairness negatives. See the maintained
[protocol and evidence](../../../../docs/30-39-status/30.44-bounded-owner-resident-service.org).

## Pooled resident supply

`just bridge-scheme-service-benchmark --pooled-ab` compares fixed-owner resident
supply against the public pool-source API. Both arms use the same four native VMs,
two Tokio workers, original batch-eight Scheme workload, three active callbacks
per owner, native short service, progress-aware short reselection, the explicit native wake policy,
budget one, feed one, coalesced notification and native slot/work bounds.
The input ceiling is 128 callbacks/1024 declared work units across the pool:
control stages four fixed groups of 32/256; candidate stages one shared group of
128/1024. Chunk boundaries differ: control drains each owner's group independently;
candidate drains the shared group before re-registering. This pair measures the
whole supply algorithm, not solely mutex or routing overhead. No default changes.

`--test --pooled-ab` is a correctness smoke. Four owners need not all receive work
in a short opportunistic sample. `unused_owners` is explicit, and any unused owner
counts as zero in normalized minimum progress. Finish spread covers participating
owners and must be read alongside that list. Preserve all short/scheduled/batch
p99, overlap, owner totals and Actor fairness negatives. Phase observations are a
separate diagnostic, never interchangeable with unobserved throughput. See
[protocol and measured evidence](../../../../docs/30-39-status/30.45-pooled-resident-service.org).

## Applied policy receipts and native wait boundaries

`just bridge-scheme-service-benchmark --pooled-wake-ab` keeps pooled resident
supply and capacity/progress short routing in both arms. It compares NativePort
against Watcher with cooperative budget one. `--test --pooled-wake-ab` is a
correctness smoke. Every comparison now has an exhaustive requested-policy map,
applies its settings explicitly, reads `scheme_service_settings_on` on every
owner, and asserts the native receipts before timing. The emitted
`native_policy_receipts` are read from Scheme, not inferred from the strategy name.
Existing defaults remain unchanged.

Historical `--pooled-ab` runs before this repair implicitly used Watcher/budget
zero because an outer match omitted that comparison. The report preserves their
numbers with a configuration correction. Current `--pooled-ab` applies and verifies
the intended NativePort/budget-one settings; it is a separate baseline.

With `--phase-observation`, `native_wait_event_width` is six. Rows are
`[kind, start_ns, end_ns, requested_ns, thread_cpu_ns, wake_flags]` after the two
existing overflow/error headers. Kinds retain their previous meanings. Wake flag
bits are: 1 the registered wake pipe is in the read set; 2 bytes are readable
before the syscall; 4 the returned ready set includes that pipe; 8 bytes remain
readable after return. The descriptor is identified by the registered pipe's
`fstat` identity; probes consume no bytes and own no new descriptors. Entry flags
are sampled before syscall timing. The observer preserves arguments and errno.

Owner spans include full monotonic start/end boundaries, so select wall/CPU can
be matched to an enclosing phase instead of treating an overlap as clipped CPU.
A ready-at-entry syscall's non-CPU wall interval exposes OS/syscall delay, but
there is no exact kernel runnable timestamp. A written notification followed by
a ready-set return gives an upper bound on notification-to-return delay, not pure
scheduler latency. Outside-select non-CPU wall time remains unattributed unless
additional evidence identifies its blocking/descheduling cause. Keep all
instrumented negatives separate from unobserved throughput and p99 acceptance.

### Resident supply and OS point observations

The phase array appends `Supply` after the original Advance/Wait/Submit/Retire
indices. It covers owner-local producer steps, including lifecycle and capacity
checks; previously these Rust-side intervals had no separate phase. Disabled
observation still takes no phase clocks.

On the Darwin qualification target, `--state-observation` takes bounded nominal
one-millisecond point samples of the four named owners, plus request entry marks.
It reads no per-request Scheme diagnostics and enables neither whole-owner phase
monitoring nor the calibration workload. `--phase-observation` retains its full
observation and uses the same sampler. Observation remains off by default.

```text
just bridge-scheme-service-benchmark --test --observer-ab --state-observation
just bridge-scheme-service-benchmark --observer-ab --state-observation
```

This is an Off/State observer-cost pair on the same VMs and pooled workload.
It changes entry marks and polling together, so it cannot isolate the sampler's
individual effect. It is not a routing optimization. `--blocks=50` explicitly increases repetitions for
rare-event capture; it retains native inputs and the five-second progress gate.

`os_thread_samples` schema version 2 has ten-column rows:
`[owner, before_ns, after_ns, run_state, cumulative_cpu_ns, flags, policy,
current_priority, base_priority, max_priority]`. It declares columns, CPU
resolution, requested period, owner TIDs and overflow/error counts. Historical
seven-column rows use CPU microseconds and must not be silently reinterpreted.
The sampler completes a real initial sweep before timing and a final sweep after
completion. Fixed numeric rows are buffered during execution; JSON serialization
occurs after stopping. Inspection rights are released and the sampler joins,
including unwinding. No owner is suspended, reprioritized or migrated.

`THREAD_EXTENDED_INFO` states are points: 1 runnable or running, 2 stopped,
3 waiting, 4 uninterruptible wait, 5 halted. State 1 cannot separate on-CPU from
ready-queue time. Apple XNU's
[implementation](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/osfmk/kern/thread.c)
reports `TH_RUN` as state 1; extended CPU nanosecond fields are converted from
microsecond counters. Reading this information can update lazy scheduler
bookkeeping, so the observer is not assumed transparent.

The Rust bench emits `os_request_attribution` after timing. It validates owner
TIDs against native clock anchors and monotonic CPU counters. The last point
strictly before each request and the first strictly after it bound that owner's
CPU consumption, including a two-microsecond quantization allowance. Clock
uncertainty and resolution widen the required brackets. Missing brackets yield
null bounds. This is owner CPU, not isolated request or GC CPU. State points are
never interpolated: the full request interval remains unclassified, with actual
sampler gaps and query costs reported. Per-request GC attribution still requires
independent evidence. GC wall and enclosing CPU remain overlapping counters;
process GC CPU cannot be subtracted from owner wall time.

`owner_capacity --wait-trace` consumes the same six-column native wait rows as
this service bench and emits `wait_event_width: 6`; historical four-column
consumers must not reinterpret those records silently.

### Capacity-release boundary and observer calibration

`Release` appends phase six, after Supply. The original five indices remain
unchanged. It encloses native slot/work permit return after projection, including
host admission notifications; this destructor was previously outside Retire's
closure. `owner_phase_names` declares the phase order. No notification policy
or default is changed by this observation.

Full Phase runs also emit one `os_calibration` event before workload warmup. Eight
controls run on the same four initialized owners: concurrent 20 ms sleeps, then
concurrent 20 ms of measured thread CPU using Rust spin loops. They make no
Scheme call, assert unchanged GC wall counters, and retain the original progress
watchdog. These known controls validate CPU/state observation and expose
non-CPU wall time in a CPU-demanding control; they are not Scheme throughput or a substitute workload.
Unobserved performance runs execute neither calibration nor OS sampling.


### Complete owner scheduling capture

`--trace-once` requires an explicit Phase, Probe, Entry or State observation mode. It runs one A/B per CPU/mixed
load, retaining the normal 8192-order inputs, 64 short probes, warmups, four live
VMs, two Tokio workers, capacity bounds and original progress watchdog. It does
not shorten the input to the smoke workload. Configuration emits `trace_once`;
these four diagnostic samples are not performance repetitions. Normal runs and
`--test` retain their existing repetition counts unless this flag is explicit.

Observation resets emit an `OwnerClock` Points of Interest signpost from each
fixed owner. `kernel_clock_anchors` contains one `[before_ns, after_ns, tid]` row
per owner, bracketing emission with the declared native trace clock.
`kernel_clock_domain` declares the domain; Darwin now uses `CLOCK_UPTIME_RAW`,
matching Rust `Instant`, and `kernel_clock_resolution_ns` reports its resolution. The native wait
trace remains six columns and its existing event kinds are unchanged. A tiny
host-Clang object uses Apple's standard signpost macros; Gambit-generated C
continues through its configured native compiler. Each marker releases its log
object. Ordinary unobserved runs emit no markers. The explicit observer-cost pair uses
matched fences outside timing in both arms, including its unobserved arm.

Build through Cargo; use the executable printed by `--no-run` as `SERVICE_BENCH`
and choose an external evidence directory as `STATE_DIR`:

```text
cargo bench -p gerbil-scheme-qualification --features instance-dispatch --bench scheme_service --locked --no-run
xcrun xctrace record --no-prompt --template 'Time Profiler' --instrument 'Thread State Trace' --time-limit 20s --window 5s --target-stdout "$STATE_DIR/service.jsonl" --output "$STATE_DIR/service.trace" --launch -- "$SERVICE_BENCH" --pooled-ab --phase-observation --trace-once
xcrun xctrace export --input "$STATE_DIR/service.trace" --toc --output "$STATE_DIR/service-toc.xml"
xcrun xctrace export --input "$STATE_DIR/service.trace" --xpath '/trace-toc/run[@number="1"]/data/table[@schema="os-signpost"]' --output "$STATE_DIR/service-clocks.xml"
xcrun xctrace export --input "$STATE_DIR/service.trace" --xpath '/trace-toc/run[@number="1"]/data/table[@schema="thread-state"]' --output "$STATE_DIR/service-states.xml"
xcrun xctrace export --input "$STATE_DIR/service.trace" --xpath '/trace-toc/run[@number="1"]/data/table[@schema="syscall"]' --output "$STATE_DIR/service-syscalls.xml"
```

Require recorder/export success, target exit zero, clean native shutdown and
four owner TIDs in the scheduling table. A readable file alone is insufficient:
a rolling window can discard owner creation metadata and leave unassigned
thread rows. Do not attribute those rows to guessed owners. The window must
retain creation plus workload through recorder flush; five seconds worked for
the directed pair on the qualified host, not as a universal duration guarantee.

Resolve XML references within their exported node. Match markers by `(tid,
monotonic_ns)`, since different owners can share a clock tick. Intersect their
clock-offset bounds, accounting for the declared clock and measured resolution.
Add each request's reported anchor uncertainty and one native clock tick when
aligning its entry marks; the older short-capture qualified total was below 3 us. Longer adjusted-clock
captures can drift; reject an empty intersection rather than widening it.
Clip scheduling/syscall intervals only after validating full per-owner coverage;
reject unknown/missing/overlapping intervals. Native CPU counters enclose their
whole measured phase and cannot be clipped by prorating wall overlap.

See [complete scheduling evidence and rejected phase routing](../../../../docs/30-39-status/30.49-owner-scheduling-and-gc-cpu-qualification.org).


### Low-intrusion probe controls

```text
just bridge-scheme-service-benchmark --observer-ab
just bridge-scheme-service-benchmark --observer-ab --blocks=50
just bridge-scheme-service-benchmark --pooled-ab --probe-observation
```

`--probe-observation` retains the nine request entry marks and three native
counter snapshots per short request, plus owner clock anchors outside sample
timing. It disables whole-owner phase monitors, OS polling, calibration and
native select/sleep recording. It adds no bulk-request counter snapshots.
`probe_native_snapshot_ns` separately reports the wall cost of each of the three
diagnostic reads. Construction includes reads one/two and projection includes
read three; do not attribute their full wall spans to type conversion. These
costs also include any off-core delay inside the read.
`native_probe_snapshots` checks actual snapshot availability; missing native
counters reject an observed native-short sample. `observation_mode` distinguishes
Off, Entry, State, Probe and Phase. Native trace reset/readback asserts no wait events in Probe.

`--observer-ab` compares observation Off with Probe on the same live VMs, using
pooled supply, native short layout, progress-aware routing, NativePort wake and
budget one in both arms. Clock fences execute outside timing in both arms to
balance their Scheme allocations. The observer still has measurable cost;
close p99 values do not establish transparency at rarer percentiles. This flag
cannot be combined with either observation flag or another comparison.

`--blocks=N` changes only repetition count (1 through 50). The default remains
five blocks, with AB/BA order alternation. Smoke and `--trace-once` retain their
fixed single block and reject an explicit block count. Long captures reuse the
same VMs, workload and five-second progress gate. Do not run a long capture in
a rolling kernel-trace window that discards owner identity metadata.

For one reduced kernel capture, replace `--phase-observation` with
`--probe-observation` in the xctrace command above. All clock, identity, coverage
and cleanup gates still apply. Kernel acquisition remains intrusive and is not
an unobserved throughput result. See [low-intrusion tail qualification](../../../../docs/30-39-status/30.50-low-intrusion-owner-tail-qualification.org)
for reproduced unobserved tails, GC-associated construction and remaining limits.


### Diagnostic-free request windows and short-route control

```text
just bridge-scheme-service-benchmark --observer-ab --entry-observation
just bridge-scheme-service-benchmark --short-route-load-ab
```

Entry records request entry marks and no per-request native diagnostics or CPU/GC
reads. Off/Entry uses the same pooled supply in both arms. The explicit
observer-cost comparison records one `sample_native_clock` outside timing in both
arms. `probe_owners` comes from the actual completed reply. In these samples,
request end is sample native release time plus `index * 500000` plus
`probe_scheduled_ns[index]`; start subtracts `probe_actual_ns[index]`. Existing
stage spans then locate construction and projection. Check this bridge against
Entry's independently anchored marks before attributing observation-Off requests.
Ordinary observation-Off runs do not enable native clock collection.

For a small full-size capture without Time Profiler, use the already built
`SERVICE_BENCH` and an external `STATE_DIR`:

```text
xcrun xctrace record --no-prompt --instrument 'Thread State Trace' --instrument 'Points of Interest' --time-limit 15s --window 5s --target-stdout "$STATE_DIR/entry.jsonl" --output "$STATE_DIR/entry.trace" --launch -- "$SERVICE_BENCH" --observer-ab --entry-observation --blocks=1
```

The instrument-only command uses a Blank template. Preserve the normal sample
payload and five-second native progress gate; validate owner identities, every
clock anchor and complete request coverage. This is diagnostic acquisition,
not an unobserved throughput result. Larger windows/runs are not automatically
qualified, and a normal target exit does not establish recorder finalization.

`--short-route-load-ab` uses pooled native bulk supply in both arms, comparing
`ReselectProgress` with `Reselect` for the short router. Both still reserve real
native capacity before publication. This isolates occupancy ranking, not credit
safety, migration or Scheme scheduling. Results were inconsistent, including
negative throughput/fairness results; do not promote a new default.

Feed configuration requires a separate bounded native-retirement fence after
control results arrive. A result ticket can arrive before the control's host
capacity permit drops. The bench now waits for all work/slot/producer credits
before the existing idle-only feed API; it does not retry rejected settings or
weaken the API's validation.

See [same-run diagnostic-free attribution and retained route negatives](../../../../docs/30-39-status/30.51-diagnostic-free-owner-tail-and-route-qualification.org).


`--await-trace-start` is an explicit pre-VM attachment handshake. It emits
`trace_attach_ready` with the real PID and waits for one stdin byte `R`. An
external controller can launch the benchmark normally, attach the instrument-only
recorder before owner creation, wait for `--notify-tracing-started`, then release
that byte. Bound the controller wait separately. EOF/bad tokens fail before VM
creation. The original native watchdog starts after this acquisition barrier;
no native progress is fabricated and the default invocation has no barrier.
An attached trace still needs the same identity, clock, coverage and cleanup
checks and is not proof that unprofiled extreme tails have been reproduced.

## Owner-local supply ownership comparison

```text
just bridge-scheme-service-benchmark --test --supply-admission-ab
just bridge-scheme-service-benchmark --supply-admission-ab
```

Both arms use the same four live VMs, native O2 image, pooled source, two Tokio
workers, NativePort, cooperative budget one, feed one, per-owner slots four/work
32 and three active bulk callbacks. Routing, input order, batch eight, native
constructors, Scheme handlers, result projection and bounds are identical.
Observation is Off by default in both arms; this is not the observer-cost pair.
`supply_admission` declares the actual API argument for every sample.

The control uses `SchemeServiceSupplyAdmission::Queued`. The opt-in `OwnerLocal`
policy is passed to `submit_scheme_service_pool_stream_with_admission`. It keeps
pending work units with their waiter, takes ready credits without creating a
waiter, and retains stream active credit inside the native capacity guard rather
than allocating a second boxed `StreamJob`. A registered producer caches immutable
owner identity and reads a pool-close token instead of upgrading/locking the global
admission table per step. Close invalidates that token under the same table fence;
callbacks still run outside the lock. Genuine waits keep their FIFO future;
unit changes cancel and refund the old reservation. Projection/failure must finish
before the native guard returns active/work/slot credits. No rooted work migrates.
The ordinary pool and fixed-owner APIs keep the existing policy/defaults.

This is a combined ownership/acquisition comparison. Fewer boxes do not establish
end-to-end speedup. A full pipeline can add failed immediate attempts before the
normal wait, and the capacity guard has an additional optional credit field.
Retain CPU p99 and owner finish-spread regressions alongside any throughput gains.
See [the optimization audit](../../../../docs/30-39-status/30.52-owner-supply-ownership-and-optimization-audit.org)
for the targeted result and the remaining early-owner-binding/notification work.

### Shared capacity and late owner binding

```text
just bridge-scheme-service-benchmark --test --pool-protocol-ab
just bridge-scheme-service-benchmark --pool-protocol-ab
```

Both arms retain four live VMs, O2 Scheme code, native short types, two Tokio
workers, one cooperative work quantum, slots four/work 32, and `OwnerLocal`
bulk supply. The control submits shorts through watch-backed `ReselectProgress`;
the candidate uses `SchemeShortPool::submit`, letting a running owner claim an
unassigned Rust payload and real credits before native construction. Configure
this bounded facility once; it freezes native budget identities until shutdown.

Admission ends at host publication for the control and owner claim for the
candidate. Compare end-to-end latency, not interchangeable component labels.
The candidate emits `deferred_short_owner_claim=true`, `owner_reselection=false`
and `producer_progress_routing=false`. `class_routing` is the class-aware strategy
selector, not evidence that the candidate submits through the host router.
Observation remains off. One joint comparison cannot isolate the two changes.

The initial capture before the final owner-failure close-fence repair retained
CPU throughput +1.11%/p99 -9.29%, but mixed throughput
-0.46%, p99 +1.86% and maximum finish spread +19.91%. No default or overall
performance promotion follows. [Protocol, native controls and negative results](../../../../docs/30-39-status/30.53-shared-capacity-and-late-owner-binding.org)
record the source identity and remaining ownership/scheduling boundaries.

After the final owner-failure close-fence repair, one affected pair was recaptured:
CPU throughput +0.55%/short p99 -79.60%, mixed throughput +3.32%/short p99 -15.20%.
CPU maximum owner finish spread regressed +34.55%, and CPU short-owner placement
remained concentrated. Both captures retain their own source/binary identities;
the changed outcome is not attributed to the close repair or promoted to a
stable/default speedup. See the final comparison in the same protocol report.
