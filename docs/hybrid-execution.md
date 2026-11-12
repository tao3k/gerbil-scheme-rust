# Native Scheme execution and Rust dispatch

## Execution contract

Scheme application code follows the native Gerbil -> Gambit C -> native compilation chain. Rust owns host admission, cross-instance dispatch and lifecycle. Complete Scheme runtimes execute the application on fixed OS owner threads, retaining macros, GC, closures, continuations, dynamic bindings, Actors and green threads.

The default Python ledger Rust backend starts four independent native Scheme instances. Tokio or Rayon workers submit jobs to those owners; they do not execute or migrate a Scheme VM. Each VM uses one Scheme processor. A same-process native crash can affect the whole process; independent heaps are not crash containment.

The old Scheme-to-Rust IR compiler, ledger typed-fold source and VM-free ASCENT qualification adapter have been removed. There is no retained compatibility alias or automatic no-VM fallback. The support crate's unused `rust-backend` feature has also been removed. The ledger's `rust-backend` feature remains and explicitly selects `instance-backend`.

## Native values and ownership

Use the existing native type interfaces. Stateful sessions pin to one owner; sessions on that owner share its Scheme state. Stateless requests declare independence, follow round-robin owner assignment and reject native references. A native handle belongs to its originating owner and library. Scheme objects and continuations never migrate between heaps.

`session.value(data)` retains an actual Scheme graph. Reusing that handle can avoid repeated Python traversal and native input construction. It preserves identity and therefore does not promise immutability. The benchmark's payload is read-only; mutable application state requires its own owner/order contract.

`prepare_request(data)` is an immutable host-owned input description. Each invocation constructs fresh Scheme objects. It is useful for repeated portable inputs and cannot contain native references. It does not execute Scheme in Rust.

`project_many(..., release=True)` materializes all Python values before optional owner-batched retirement. Projection failure preserves native procedures, cycles and continuations. Release delivery can retire a subset; idempotent release supports retry. Original bounds and observation deadlines remain active; timeout does not cancel accepted native work.

`OwnedValue` and the optional C ABI still describe owned transport and callback lifetimes. Their `NO_VM` flag can describe copied host values or an independent C callback; it does not select a Scheme backend or authorize evaluating Scheme without a VM. `SCHEME_VM` callbacks require their live owner VM through final release. Flags do not acquire leases or establish runtime safety by themselves.

## Build and packaging

The existing Gerbil build owns expansion, compilation, dependency closure and module grouping. Cargo builds and packages native artifacts. Python consumers use the installed wheel and native APIs. Do not add a Python compiler, handwritten Rust Scheme evaluator, subset lowering pipeline, case registry or silent provider fallback.

The separate linked `scheme` provider retains a process-global Scheme runtime and is an explicit single-owner control. It also executes Scheme. It does not qualify independent parallel entry into a shared VM.

## Performance and qualification

The working owner-resident payload observation is 10.10x throughput versus rebuilding the same read-only inputs, with scheduled short-request p99 72.045 ms. This is a bounded matched observation, not a fixed gate, intrinsic Gambit speedup or one-to-four VM scaling result. See [the baseline](30-39-status/30.19-owner-resident-payload-baseline.org).

A scalar projection experiment did not establish a stable endpoint gain and was not admitted. Continue studying temporary roots, native traversal and GC allocation while keeping generic types and failure semantics. See [the non-admission report](30-39-status/30.20-scalar-projection-experiment.org).

Qualify changes through real installed Python calls, native types, GC, multi-shot continuations, exceptions, initialization/retirement, matched same-VM performance, p99 and finite fairness observations. Source checks, models, local runs, CI and portability are distinct evidence. The private-image backend remains experimental on Darwin.

Raw logs, performance captures, experimental patches and source snapshots are kept outside the repository. Maintain English summaries, reproduction commands and executable tests in source control.


## Opt-in completion-driven Rust admission

The optional Rust `gerbil_scheme_python::owner_flow` API drives owner-affine FIFO
lanes through the existing native async ticket API. One caller future rotates
admission across owners, replenishes bounded per-invocation credits and places
completed results in input order without spawning per-request or per-owner Rust
tasks. It does not execute Scheme, replace its scheduler, reserve global queue
capacity or add a Python dispatch mode. Existing selections and defaults remain.

Focused same-VM comparisons show throughput gains versus Tokio/Rayon in the
recorded short CPU cohorts, with unstable p99, direct and fairness outcomes.
The candidate is not promoted. See the [architecture, negatives and next capacity
boundary](30-39-status/30.27-completion-driven-owner-flow.org).


## Optional pool-wide capacity and Scheme batch policy

Configure `InstanceDispatcher::configure_owner_capacity` before the first
unbounded dispatcher admission to share per-owner request credits across every
clone/API. Backpressured native and numeric-batch methods await that capacity;
accepted requests retain it until their owner finishes execution and delivery,
including abandoned-reply rollback. Observation timeout never returns credit early.
The optional `owner_flow_backpressured` driver limits pending reservations per
owner/producer and bounds admission futures by the common deadline. Defaults and
existing Python dispatcher modes remain unchanged.

Native Scheme batches retain their explicit whole-turn exception/drain semantics.
Request slots are not work/time credits: increasing batch size can worsen short
request latency. See the [shared-capacity protocol, focused fairness controls and
remaining service-budget boundary](30-39-status/30.28-shared-owner-capacity-and-batch-fairness.org).


## Optional independent work budget

Use `configure_owner_budgets(request_slots, Some(work_units))` to constrain
outstanding declared work separately from the number of native requests.
Every numeric batch API charges its item count. Generic native/byte jobs retain
a unit charge; applications can explicitly declare a larger charge with
`submit_native_weighted_backpressured_on`. This is a declared resource contract,
not automatic cost inference. `configure_owner_capacity` retains slot-only behavior.
Neither API changes the production default or existing Python mode selection.

Work credits are acquired before a request slot under one finite wait budget.
Cancellation returns partial unaccepted reservations. Both resources remain held
through accepted execution, reply delivery and abandoned-reply rollback. Quiescent
reconfiguration and pool close cover both resources and preserve generation fencing.
FIFO weighted admission can still delay a short request behind a large one.

A batch remains one native owner turn: internal Scheme yields do not admit another
Rust queue entry. Item bounds cannot guarantee an arbitrary handler's service time.
Observed service spans include native construction, Scheme/GC, projection and root
retirement; `submit_numeric_batch_profiled_backpressured` exposes the existing
phase markers after weighted admission. No batch splitting, evaluator restriction
or host preemption is introduced. See the [targeted batch-8 qualification](30-39-status/30.29-independent-work-budget-and-batch-service.org).


## Explicit bounded producer on a native owner

`submit_numeric_batch_stream_backpressured` and its profiled counterpart stage a
bounded sequence of explicitly independent numeric batches on one fixed owner.
The producer advances one original Scheme call per native return, checking the
inbox and separately acquiring the same request/work credits for every turn. It
removes a Tokio replenishment round trip per batch; Scheme still owns execution,
Actors, green threads, continuations and GC. Arbitrary stateful/dependent calls
retain the ordinary owner-pinned APIs and their ordering contract.

A separate registration permit limits each owner to one producer. Staged turns
cannot exceed the native queue capacity, and the existing transfer bounds apply
in aggregate to input/output. Staged input is Rust-owned pending work, not accepted
Scheme work. Completed numeric rows hold only bounded owned Rust data. Every
turn releases native credits after the common execution/projection/root-retirement
path; the final cursor result does not retain native roots.

The admission budget is also the start deadline for unadmitted turns, measured
from API entry. Close/deadline rejects unadmitted turns with no native completion
clock. Entered calls drain normally; one batch's Scheme error does not cancel
later explicitly independent batches. A receiver timeout never preempts Scheme.
Waiting registration futures retain no owner-channel sender, so native close does
not depend on rescheduling those external futures. Pending credit acquisition
uses semaphore wakers and the blocking owner inbox/deadline, not periodic polling.

This API is opt-in. The matched work-eight pipeline experiment, gains, fairness
regressions and unresolved startup failure are recorded in
[status report 30.30](30-39-status/30.30-bounded-native-owner-producer.org).


## Native construction inside explicit owner producers

The explicit numeric batch-stream APIs use a single-entry Scheme constructor when
its image exports `gerbil_scheme_rust_numeric_batch_create`. All numeric child
vectors and the quantum/envelope are constructed and protected by Scheme; only
the complete envelope transfers an external strong root. At batch eight this
reduces input root registrations from eleven to one. The older native image path
retains its existing rooted Scheme implementation. Ordinary batch APIs and Python
backend selection retain their current behavior.

`NativeSession::numeric_batch_envelope` exposes this opt-in typed construction
with the existing count, quantum and aggregate input bounds. It reports missing
image support explicitly. `copy_numeric_element` retains an independent selected
child root across numeric sizing/copy, preserving identity if a mutable parent
slot changes; protecting the parent alone would not suffice.

Profiled turns now distinguish combined construction, projection and root
retirement, with optional GC/owner-CPU counters. `BatchStreamRow::offered` records
the cursor offer before enqueue, not native acceptance of staged turns. No
unprofiled per-turn clocks or Scheme-thread scheduler are introduced. See
[30.31 native construction and owner skew](30-39-status/30.31-native-batch-construction-and-owner-skew.org)
for the directed controls, withdrawn candidate and retained tail/fairness negatives.

### Host OS scheduling and Scheme wake ownership

OS thread service class is separate from Scheme Actor/green-thread scheduling.
The existing owner-pinned native closure API can execute an explicit host policy
on the correct fixed OS thread; selecting such a policy does not transfer Scheme
continuations or scheduling to Tokio. There is no automatic QoS policy or changed
pool default. Diagnostic Scheme commands used by qualification are private
fixtures, not new public runtime opcodes.

The native batch call is synchronous. Scheme yielding or sleeping inside it
lets other Scheme threads run, but the Rust owner pump checks its inbox after
that full foreign call returns. Reducing the inter-batch refill gap cannot by
itself remove OS wait overshoot or make host requests enter during that call.
The explicit Scheme-owned service now provides that retained-root, bounded-work,
wake and close/drain protocol. Ordinary synchronous calls keep their semantics.

The directed native measurements and negative QoS results are recorded in
[the wait attribution report](30-39-status/30.32-native-wait-attribution-and-owner-qos.org).

### Explicit Scheme-owned asynchronous service

A build-owned rooted unary endpoint created with
`make-gerbil-rs-service-endpoint` can be admitted by the unsafe
`IsolatedProgram::with_scheme_service` descriptor. The library resolves this
capability privately before VM initialization; ordinary `call_export` cannot
invoke its lifecycle commands.

`InstanceDispatcher::submit_scheme_service_backpressured_on` accepts a
`SchemeServiceRequest`: construct a native payload on its fixed owner, execute
inside Scheme, and project an owned Rust result after completion.
`NumericServiceBatch` reuses the existing native batch constructor/projector.
A `SchemeServiceLane` selects a persistent Scheme Actor for stateful sessions;
independent requests leave worker selection to Scheme. Lane affinity is not an
exclusive lease: applications retain responsibility for shared state.

Slot/work permits survive Scheme waits, observation timeouts and abandoned
receivers. A Scheme table strongly holds each accepted job until take/projection.
A task-owned native pipe wakes Scheme's event wait when the Rust inbox changes;
clear/recheck prevents sleeping past a published request. Scheme owns its workers,
continuations, GC and timers. Rust has no idle polling timer or Scheme scheduler.

Shutdown rejects new admission, prepares the application, drains accepted work,
joins Scheme children and retires roots before SDK cleanup. Failed quiescence
quarantines heap/code with an unsuccessful receipt. This is opt-in; defaults and
ordinary native APIs are unchanged. See the
[service protocol and directed measurements](30-39-status/30.33-scheme-owned-service-and-native-wake.org)
for native acceptance, reused model limits and retained performance regressions.

### Native short layout and ready completion observation

Applications can implement a distinct bounded native short schema through the
same `SchemeServiceRequest`, while keeping batch work on Scheme Actors. Schema
selection must be explicit; short data size does not authorize Rust to schedule
Scheme green threads. The qualification implements a four-line order/result
schema alongside the existing full batch path.

`owner_flow_ready_backpressured` in the Python/Rust client support library is an
explicit flow API for asynchronous Scheme services. It can observe completed
peers behind a pending head request and refill client window credits while
preserving original output positions. Native capacity and root retirement remain
owner-bound. Existing flow APIs and defaults remain unchanged. See
[30.34 short layout and ready completion](30-39-status/30.34-short-layout-and-ready-completion.org)
for the actual native acceptance, matched comparison and retained negatives.

### Explicit short and batch route algorithms

`SchemeServiceRouter` accepts an explicit `SchemeRoutingConfig` over selected
owners and existing native budgets. `submit_short` prioritizes available short
quota and request queue pressure; `submit_batch` prioritizes available batch
quota and outstanding declared work. Router clones share class limits and
load/cancellation guards. `RoutedSchemeTicket` retains the actual selected owner.
Requests with a Scheme lane must keep the original fixed-owner API.

This routes native work across VMs; Scheme owns execution and intra-VM scheduling.
Class admission close drains through the existing native lifecycle protocol.
The current opt-in heuristic has mixed-throughput and short-tail regressions, so
it does not replace defaults or the prior accepted path. See the
[class-routing qualification and negative results](30-39-status/30.35-class-specific-native-service-routing.org).

### Explicit Scheme short scheduling

Cross-instance routing and intra-instance service scheduling are separate choices.
`SchemeServiceRouter::submit_short` selects an owner under class quotas;
`SchemeServiceRequest::short_service()` independently opts that request into the
Scheme short queue. The latter requires one declared work unit, no Actor lane and
service ABI v2. Existing general requests and defaults remain unchanged. Rust
rejects an unsupported endpoint before native construction rather than silently
falling back. Scheme preserves execution, continuations, native waits and GC.

Generated SCM files contain unmodified compiler bytes. External `.scm.sha256`
receipts validate the SHA-256 of each **entire file**, plus framed build-input
identity. `just bridge-regenerate` owns updates. No hash is embedded in a comment
inside its own artifact. This is integrity/input-provenance validation, not proof
that independently compiled output is byte-identical: Gerbil embeds a compilation
timestamp. The native build and qualification gates remain separate.


### Shared resources, explicit feed bounds and observation

Separate short queues and worker quotas do not isolate a VM's CPU or GC.
`InstanceDispatcher::configure_scheme_service_feed_on(owner, maximum)` is an
opt-in input-publication bound, with idle native-capacity validation. It feeds
immediately available FIFO service inputs before advancing Scheme and never
waits to fill a batch. The default remains one. This does not interrupt arbitrary
Scheme code, migrate Actors or schedule Scheme green threads in Rust.

`observe_scheme_service_on(owner)` enables optional owner phase diagnostics;
`scheme_service_observation_on(owner)` returns a ticket for an owned snapshot.
Counters cover advance, wait, submit, retire, owner-resident supply and
capacity release wall/thread CPU/GC spans. Supply and Release append phases
five and six; prior indices remain unchanged. Release includes permit return
and host admission notification after projection. The Rust bench can additionally sample own-owner OS run states;
these are point observations, not exact kernel dispatch timestamps. See
[the directed supply and OS qualification](30-39-status/30.47-owner-supply-and-os-run-state.org).
Native waits may include Scheme work, blocked select and OS descheduling; these
spans do not identify pure OS delay. Observation is disabled by default. Known CPU/sleep controls validate observed
runs before warmup; see [capacity release qualification](30-39-status/30.48-capacity-release-and-notification-qualification.org).

The reused lifecycle model now contains shared-resource counterexamples and a
separate candidate cooperative CPU/class-debt policy. The latter is not an
implemented general execution-time guarantee. See the
[shared-resource refinement and directed feed qualification](30-39-status/30.37-shared-resource-refinement-and-owner-feed.org)
and [model boundaries](../specs/owner_lifecycle.md#shared-vm-resources-and-scheduling-refinement).


### Scheme cooperative unit and class policy (explicit opt-in)

ABI v3 adds `configure_scheme_service_budget_on(owner, maximum_consecutive_units)`.
The returned fixed-owner ticket confirms native configuration. Zero disables the
policy (the default); 1..=256 sets a class streak bound. Configuration rejects
accepted Scheme jobs or active leases/waiters. Older service ABIs still execute
existing work and reject this unsupported policy before native entry.

Scheme applications import `:gerbil-scheme-rust/scheme/service-budget` and use
`call-with-gerbil-rs-service-unit` around an application-defined work unit, with
`call-with-gerbil-rs-service-suspension` around blocking operations inside it.
Actor batches propagate the request context to persistent children and release
an enclosing unit across their coordinator join. The real order handler uses
one complete order as a unit, releasing around its real one-ms native wait.

Scheme owns two FIFO lease queues. With both classes waiting, the configured
streak forces a class switch; otherwise the runnable class proceeds immediately.
This is fairness among participating, registered unit waiters. Rust does not
select or execute green threads. Arbitrary Scheme computation remains available
and is not preempted or transformed; an uncooperative handler, VM GC or OS pause
can still delay requests. No elapsed-time bound follows from the work-unit count.

`scheme_service_budget_on` returns typed native lease/contention counters, queue
sizes and active state. A resumed wait obtains another grant: grant counts are
not completed-order counts. Dynamic extents balance leases on exceptions and
continuation exits/reentry. Closing requires drained workers, leases and waiters.
See [native policy qualification and negative results](30-39-status/30.38-cooperative-scheme-units-and-class-fairness.org).


### Owner ingress and native input port (explicit opt-in)

ABI v4 adds `configure_scheme_service_wake_on(owner, SchemeServiceWake::NativePort)`.
The ticket confirms Scheme's idle reader transfer before Rust changes transport
state. `Watcher` remains the default. Scheme completions and host publication
notify the same native input port; Gambit retains IO/timer and green-thread
scheduling. The candidate skips empty host completion collection but does not
replace Scheme execution or introduce Rust scheduling inside an instance.

An explicit `SchemeServiceEntryTrace` on an individual request provides paired
caller/owner timestamps, notification outcome and bounded phase-history overlap.
History loss is reported; enclosing CPU/GC counters are not overlap CPU time.
No entry clocks run for unobserved requests. The directed `--ingress-ab` bench
keeps budget one in both arms, same VMs, native work, route, feed and collection.
See [attribution, native controls and all results](30-39-status/30.39-owner-entry-attribution-and-native-port-wake.org).


### Independent host notification (explicit opt-in)

`configure_scheme_service_notification_on(owner, SchemeServiceNotification::ShortIndependent)`
selects an atomic nonblocking pipe write for every explicitly declared short
service publication. That path borrows neither another publisher's pending claim
nor its writer mutex. Bulk publication stays coalesced; close also writes
independently. A full pipe is already readable and IO errors remain failures.
Descriptor lifetime stays with the shared wake object. Redundant bytes are safe
under the existing inbox recheck. `Coalesced` stays the default, independently of
native port mode. This removes short-publisher dependency, not OS or GC delay.
Scheme still owns all intra-instance execution. A prototype using host wait-phase
hints stalled and is not included in the library.
See [qualification, trade-offs and retained failures](30-39-status/30.40-independent-notification-and-mixed-tail.org).

SDK GC CPU counters measure process CPU over a VM's GC intervals, including
other host threads. They are not isolated owner/collector CPU and must not be
summed across owners or subtracted from GC wall for attribution. Owner thread
CPU is a separate clock. See the linked report for the verified clock sources.

### Targeted algorithm and contract audit

See the [routing, Scheme queue and contract audit](30-39-status/30.41-targeted-routing-and-scheme-queue-audit.org) for source-confirmed costs, admission reselection, eligible FIFO equivalence, cooperative unit semantics and directed acceptance obligations. These are candidate changes; the audit does not claim a measured performance improvement.

### Capacity-reserved stateless owner reselection (explicit opt-in)

`SchemeServiceRouter::new` and `SchemeRoutingAdmission::default()` retain
`Pinned`. Select `SchemeServiceRouter::with_admission(dispatch, config,
SchemeRoutingAdmission::Reselect)` to re-evaluate the existing class-specific
owner score whenever shared native capacity changes. Scheme still owns all
execution, native waits, green threads and continuations.

Reselection serializes admission within each class in FIFO order. It reserves
one owner's class quota, declared work credits and request slot before native
publication. A failed scan releases all partial credits before waiting on a
versioned capacity change; it does not hold reservations on every VM, poll on a
timer, or move accepted Scheme state. Direct dispatcher completions, partial
fixed-owner reservation cancellation, reconfiguration, owner failure and pool
close participate in that notification. Independent routers continue to have
independent class quotas, under shared pool native limits.

Application work/lane/execution declarations are sampled once before waiting.
`RoutedSchemeTicket::route_class` (`SchemeRouteClass`) and `execution_class`
(`SchemeServiceExecution`) describe separate choices. Existing route/general
execution controls remain valid. A short native execution declaration still
requires one nonzero stateless work unit and an admitted service capability.
An Actor lane must continue to use an explicit fixed owner.

Admission timeout/drop returns unpublished reservations. Accepted request/work
credits remain held through Scheme completion and native projection even after
a caller times out or abandons its ticket. The class FIFO prevents overtaking
within one router/class; it does not promise fairness between unrelated routers
or bypass an earlier large request in that same FIFO. Counted work is not a
wall-clock service bound. Unassigned reselection waiters are absent from
per-owner `loads()`; pinned owner-specific admission waiters remain included.

A router holds the exact capacity objects validated with its class budgets.
Quiescent native capacity reconfiguration invalidates old routers and unassigned
waiters, even when numeric limits stay the same. Create a new router against the
new capacity scope. Both constructor validation/capture and reservation identity
checks use the existing admission fence; old class quotas cannot silently cross
into a differently configured pool capacity epoch.

See the [native and matched qualification](30-39-status/30.42-capacity-reserved-owner-reselection.org).


### Explicit bounded service supply

`InstanceDispatcher::submit_scheme_service_stream_on` stages a count/work-bounded
sequence of independent stateless typed requests on a fixed owner. Its
`SchemeServiceStreamConfig` separates aggregate staged work from active native
requests. It shares the existing single-producer registration, native work/slot
budgets, capacity epoch and close fence. `SchemeServiceStream::cancel` or handle
drop rejects unadmitted work; already committed Scheme jobs drain through the
original typed service, projection and root retirement. `next` consumes FIFO
owned results; its wait timeout is not native cancellation. Affine/dependent
requests retain their ordinary owner-pinned APIs. No default changes.

This supplies Scheme's native service without returning to Tokio for each refill;
Scheme continues to schedule its Actors and green threads. See
[bounded supply qualification](30-39-status/30.44-bounded-owner-resident-service.org)
for bounds, cancellation races, validation and retained performance negatives.

### Pool-level independent supply and short-route progress

`submit_scheme_service_pool_stream` is an explicit alternative to fixed-owner
staging. `SchemeServicePoolStreamConfig` bounds the entire input count by the
existing queue capacity, aggregate declared work across the pool, and accepted
requests separately per owner. It freezes native type declarations once, rejects
affinity, validates every destination and registers producers in sorted order.
A registration barrier prevents partial startup from executing inputs. Dropping
the registration future cancels staged work and releases acquired registrations.

Only uncommitted Rust inputs share a FIFO. An owner obtains its active/work/slot
credits and passes the existing lifecycle fence before claiming the next input.
If another owner changes the FIFO's work charge, speculative credits are refunded
before retry. Empty/head-charge transitions wake waiting producers to retire or
reprice their reservations. Cancellation, expiry or a participating owner closing
rejects shared uncommitted work; accepted Scheme jobs retain owner, roots and
credits through projection. Result positions remain in input order. A completely
consumed result handle owns only weak transport signals. Application-owned outputs
retain their ordinary resource ownership.

`SchemeRoutingAdmission::ReselectProgress` additionally observes native occupied
request slots, including direct resident producers invisible to the router's own
ledger. Short-class pending counts retain priority; native occupancy then breaks
ties before the router's total/batch load. These are approximate capacity samples,
not OS-runnable timestamps or execution-duration predictions. Class quotas remain
per router; this does not create a globally reserved short queue. Existing routing,
Scheme execution, wake, budget and feed defaults remain unchanged.

See [pooled supply qualification](30-39-status/30.45-pooled-resident-service.org).

`submit_scheme_service_pool_stream_with_admission` adds the explicit
`SchemeServiceSupplyAdmission::OwnerLocal` experiment. Pending work units and
their FIFO waiter form one state. Free active/work/slot credits can be acquired
without a temporary future; genuine contention keeps the existing waiter.
Accepted stream active credit is retained by the native capacity guard through
projection/failure, removing the redundant per-job wrapper. Registered producers
cache their validated wake identity and read a pool-close token. Registration
prevents capacity replacement; close invalidates cached admission inside the
existing table fence, with callbacks outside that lock. This removes per-step
global admission upgrades/locks in the opt-in path. The ordinary API
continues to select `Queued`; no bounds or runtime defaults change. Both policies
still execute on the same fixed Scheme owners. This is not short-input stealing,
a new executor, preemption, or a complete tail-latency repair. See the
[ownership audit](30-39-status/30.52-owner-supply-ownership-and-optimization-audit.org).

### Native configuration readback

`InstanceDispatcher::scheme_service_settings_on` returns typed native
`SchemeServiceSettings`: the applied cooperative budget and `SchemeServiceWake`.
Service ABI v5 adds the read-only query; existing service versions remain accepted
for their existing operations, while older endpoints reject this new capability
explicitly. Values come from Scheme's actual service/budget state, not a host-side
cache or requested configuration. Reading does not change scheduling or defaults.
The fixed-owner capacity, native type, root and lifecycle rules still apply.

The Rust bench now configures every comparison through an exhaustive policy map,
reads every owner's settings and asserts them before starting the clock. See
[native policy and wake qualification](30-39-status/30.46-native-policy-readback-and-wake-boundaries.org)
for the historical configuration correction and the separate rebuilt baseline.


### Correlated kernel scheduling qualification

The Rust service bench now supports `--phase-observation --trace-once` for one
full-size diagnostic pair per load. Native clock signposts provide bounded
alignment to owner Running/Runnable/Blocked intervals. This is separate from
unobserved throughput qualification. Readable traces with missing owner TIDs
are rejected. [The scheduling report](30-39-status/30.49-owner-scheduling-and-gc-cpu-qualification.org)
records complete captures, genuine select blocking, GC-associated CPU work and
the withdrawn advisory phase-routing experiment. No routing or supply defaults
changed, and no phase-routing API is retained.


The explicit `--probe-observation` mode records short-request entry and native
counter snapshots without whole-owner monitoring, OS polling or wait tracing.
`--observer-ab` measures its cost against observation off on the same pooled
service. Longer repeated captures reproduced unobserved tens-of-milliseconds
tails; some large construction wall spans had little thread CPU and zero GC.
[The low-intrusion report](30-39-status/30.50-low-intrusion-owner-tail-qualification.org)
keeps the degraded runs and distinguishes those spans from actual GC CPU work.
No production routing/supply change is admitted from that attribution alone.


Entry-only qualification now omits every per-request native diagnostic read.
An outside-timing sample clock plus actual reply owner IDs also locates Off
requests in the same kernel recording. Darwin capture timestamps use raw uptime
to match Rust `Instant`; longer adjusted-clock captures that fail offset bounds
are rejected. [The follow-up report](30-39-status/30.51-diagnostic-free-owner-tail-and-route-qualification.org)
records same-run owner states, an unsuccessful short-route ranking experiment
and the feed-configuration retirement fence. Production defaults remain unchanged.

### Explicit unassigned short supply

`configure_scheme_short_pool` and `SchemeShortPool::submit` add a bounded Rust
queue for stateless one-unit short-service requests. Only a running fixed owner
claims native credits and constructs Scheme inputs. The returned ticket carries
that actual owner; accepted Scheme work survives observation timeout and drains
through projection/root retirement. Stateful lanes continue to use explicit-owner
APIs. Rust does not schedule Scheme green threads or move native values.

`ReselectProgressNotified` separately offers register-before-scan shared-capacity
notification with the existing occupancy ranking. Both facilities are opt-in.
[The directed protocol qualification](30-39-status/30.53-shared-capacity-and-late-owner-binding.org)
retains the mixed-load throughput/p99/finish-spread regressions; defaults remain
unchanged and no general speedup is admitted.

The configured short facility can explicitly select `SchemeShortWake::Selective`.
FIFO eligible wait registrations receive bounded wake obligations; running peers
can still claim and only real credit acquisition commits owner affinity. The
owner attempts visible work before registering for native wait. Ordinary inbox
interruptions and target failure relay wake duty without moving Scheme objects.
Broadcast remains the default. [Selective wake qualification](30-39-status/30.54-selective-wake-and-nonexclusive-claim.org)
records improved short placement, unresolved CPU finish spread/tail regressions,
and separate native-completion versus caller scheduling clock boundaries.
