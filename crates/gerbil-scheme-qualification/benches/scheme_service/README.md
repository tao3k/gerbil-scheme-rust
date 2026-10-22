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
submit/construction and take/projection/retirement. Samples carry per-owner wall,
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
