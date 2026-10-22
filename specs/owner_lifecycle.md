# Fixed-owner lifecycle extension

## Why this model exists

The existing `bridge_handoff.qnt` covers admission, native completion, receiver
abandonment and the drain barrier. Its foreign-copy control owner is an abstract
handoff design, not the current independent-VM implementation. We retain its
checks rather than reimplementing those schedulers here.

`owner_lifecycle.qnt` adds the actual library's shutdown boundary: replies may
arrive while close is retiring native roots, and an already entered call may
try to create a lazy Rayon pool. Neither late reply metadata nor a new pool
may repopulate the host after retirement. The model contains no Scheme evaluator,
GC implementation, Actor/green-thread scheduler, Tokio scheduler or SMP policy.

## Source correspondence

| Model transition/property | Actual boundary |
| --- | --- |
| `admit`, `close` | `InstanceService::runtime`, closed flag, native pool admission |
| `reserve`, `admitReserved`, `releaseReservation`, `configureCapacity` | `instances/capacity.rs`: shared owner reservations, the existing admission fence and quiescent reconfiguration |
| `capacityHeld` | Optional permit on `pool::Request`, retained through native execution, delivery and abandoned-reply rollback |
| `complete` | `instances/native_dispatch.rs::run_native`, fixed-owner completion |
| `observeTimeout` | Ticket observation/draining; accepted Scheme work is not cancelled |
| `publish` | `instance_runtime.rs::Registry::insert`, closed check under values lock |
| `createExecutor` | `instance_batch.rs::rayon_pool`, closed check under pool lock |
| `canUse`, `release` | Library identity/owner validation and owner-native release |
| `join` | `instances/pool.rs::shutdown`, completed owner drain and native cleanup |
| `clearRegistry`, `retire` | Registry clear, executor slot removal, resource-file retirement |
| `imagesRetained` | Default retention of initialized code; not a live Scheme heap |
| `codeLease`, `releaseCodeLease`, `authorizeCodeRelease`, `quarantineCode`, `releaseCode` | `instances/mapping.rs`: explicit code ownership, provider authorization, actual join and quarantine |

## Safety and abstraction boundaries

- Each request starts with an assigned owner. `selectOwner` can change it only
  before reservation/publication, for the stateless route. The selected owner is
  fixed while credits are held and committed immutably at publication. A handle
  can be used only on that committed owner while the host is open. Foreign library identity validation is an
  additional native gate, not modeled as a second host here.
- Timeout changes observation state without withdrawing admitted native work.
- A join requires all accepted native tasks to finish. Registry metadata is
  cleared before host retirement; a completed reply copy may still arrive later.
- The closed host has no roots, registry entries, live executor slot or resource
  files. Initialized code images remain retained unless a provider authorizes
  release after clean native join with no remaining code leases. Root/file removal
  is not complete RSS reclamation.
- Publication and executor creation abstract their lock-protected linearization
  points. An operation overlapping close may linearize before close even when
  its return happens later. The model is not a proof of Rust memory ordering or
  lock implementation. `executor` describes the host slot, not every outstanding
  `Arc` or OS thread; native tests join clients before checking thread retirement.
- `complete` assumes real fixed-owner execution and correct rooting. `join`
  assumes successful SDK cleanup. Partial setup, GC corruption, Scheme failures,
  callback safety and the underlying loader require separate native evidence.
- There is no reopen/reinitialize transition: this model cannot authorize
  reinitializing a one-shot Gambit image. Requests use finite single-use IDs;
  production bounds and token exhaustion have separate checked implementations.

The safety predicate checks closed-state retirement, ownership, result/root
conservation constraints and absence of late publication, late executor creation
or foreign handle use. Shared credits additionally conserve reservations plus
accepted work that has not completed, with a per-owner capacity bound. Seven
mutations deliberately remove one boundary each: late publication, late executor
creation, early join, foreign handle use, unsafe code release and early credit
return on observation timeout, plus premature producer retirement. The code-lease extension also permits cloning an existing
lease after join, but cannot create a first lease after code release.

The model treats each owner code group as one resource. It does not model OS
loader reference counts, module-destructor ordering or allocation failures.
The native adapter transfers SDK-owned module close references, and drains them
after actual owner join and the last lease. Native counterexamples and actual
image observations are required independently; model success cannot admit unload.
The Rust integration test requires exit 1 **and** `error: Invariant violated`; parser, tool
or dependency failures cannot satisfy a negative control.

The small domain has three requests and two owners; the wide domain has five
requests and two owners. These are exploration parameters, not runtime limits.
Each has twenty-three deterministic scenarios and 1,000 sampled traces of at most 64
transitions. A 12-transition symbolic check is a separate command. Neither finite
simulation nor bounded symbolic checking is an unbounded theorem, implementation
refinement, fairness proof or throughput result. Stuttering is allowed; arbitrary
stuck native code need not ever join.

## Native evidence remains independent

The memory qualifier exercises real Actors, hygienic DSL evaluation, retained
procedures/continuations, binary native values, exception recovery, GC and close.
Historical sustained failures are preserved. Native diagnostics establish a
process-cwd race in the SDK normalizer and an independent host-signal ownership
failure. The repaired O2 artifact completes the identical 600-second reuse and
eight-cycle lifecycle targets and independent fault controls. These are native
receipts, not consequences of this model. Retained code images, rising lifecycle
RSS, longer endurance and additional OS-global boundaries remain unqualified;
see status report 30.23 for the artifact, actual results and remaining scope.

Use `just bridge-python-dispatch-memory <python> --case reuse --seconds 600`
and `--case lifecycle --cycles 8` against an identified native O2 wheel. Generated
captures and checker traces belong outside the repository. The resource observer
uses the existing native `collect` API and Darwin process/dyld observations; it
does not replace native type calls with JSON. JSON is diagnostic output only.

Before the code-lease extension, the eight-transition symbolic run completed
with `NoError` on the three-request domain. That historical result is not
transferred to the extended source; its scenarios, simulations and five mutation
controls have been rerun independently. The twelve-transition attempt was stopped incomplete as
later SMT queries become expensive. It is not a pass, and the eight-step result
does not replace that target. Symbolic wrappers preserve all generated output
in a fresh temporary directory outside the checkout.


## Shared owner capacity extension

The capacity state refines this existing model; it does not model another runtime.
`capacityHeld` equals live reservations plus accepted work not yet completed.
Observation timeout preserves both terms. Only native completion returns accepted
credit; cancelling a reservation that was never admitted returns reservation
credit. The closed admission fence rejects conversion of a reservation into native
work. Quiescent capacity changes cannot discard accepted native work.

The finite scenarios cover timeout conservation, reserved-close rejection, native
completion/reconfiguration and an early-return mutation. FIFO semaphore mechanics,
waker reentrancy, generation identity checks and OS scheduling remain native/code
audit obligations. No temporal fairness or starvation theorem is claimed. Native
tests independently cover FIFO waiters, clone/API participation, cancellation,
continuations across forced GC, panic and in-flight close. The older symbolic
results above are not transferred to this extended state.

## Independent declared work budget

Request slots and work credits are separate per-owner resources. Numeric batch
APIs charge one work unit per item; generic native and byte jobs retain a unit
charge unless the caller uses the explicit weighted native API. These are declared
units, not a CPU-time estimate. No universal time bound follows from an item count.

The same lifecycle model assigns job 1 a weight of eight and other jobs a weight
of one. `workHeld` includes full work reservations made before slot acquisition,
slot reservations and accepted native work. `reserveWork`, `reserveSlot` and
`cancelWork` cover two-stage reservation and cancellation; both budgets must be
quiescent before reconfiguration. The finite work-bound invariant and timeout
scenario retain the entire charge until native completion. Partial multi-permit
allocation inside Tokio, FIFO order and waker behavior are outside the abstraction;
actual native cancellation, timeout, panic and close tests cover those boundaries.

A Scheme batch is a non-preemptible Rust owner turn. Its service span includes
native construction, Scheme scheduling/execution/GC, projection and final root
retirement. Scheme-internal yields allow peer green threads to run; they do not
service another Rust queue entry. Limiting the batch's item count bounds the number
of handlers in a turn, not arbitrary handler duration. Hard wall-clock preemption
is not introduced, and continuations, exceptions and drain semantics are unchanged.


## Bounded native owner producer extension

The same model now tracks one registered producer per owner, staged Rust input and
producer-owned job identities. Staging does not accept native work or take work
credits. Each original batch must still reserve the existing work/slot resources
and cross the close fence. `producerNativeRefillTest` accepts consecutive original
turns at the same work bound without a host-scheduler transition between them.
The implementation only advances whole native calls after a foreign return; it
contains no Scheme evaluator or Scheme Actor/green-thread scheduler.

Close cancels unadmitted staged turns and their partial reservations. Accepted
native work retains its credits through actual completion. The producer registration
must retire before join or reconfiguration; the new premature-producer-exit mutation
violates that invariant. Both domains have 23 deterministic scenarios and the
integration gate requires seven actual invariant counterexamples. The finite
abstraction does not model partial Tokio allocations, inbox wakers, output byte
limits or native startup. Those remain direct source/native controls, including
close while a host registration future remains deliberately unpolled. No symbolic,
starvation-freedom, GC implementation or cold-start reliability theorem is added.


## Shared VM resources and scheduling refinement

`service_sharing` imports the existing lifecycle as `life`. It reuses admission,
assigned owners, work/slot credits, accepted roots, result publication, timeout,
close and join; it does not create a second resource-lifetime authority.
`service_sharing_cases` supplies executable scenarios for the small and wide
instantiations. The finite configuration uses two single-processor VMs, two
persistent Scheme workers per VM and four/five single-use requests. These are
exploration parameters, not changes to the production four-VM benchmark.

The refinement explicitly separates:

- Rust owner inbox and bounded native input feeding before Scheme advancement;
- Scheme class queues and worker assignment;
- the single execution resource shared by workers in each VM;
- cooperative work units, blocked green threads and wake transitions;
- VM-wide GC and owner descheduling by the OS;
- completion projection, abandonment, rooted draining and actual join.

`currentInit` describes cooperating class-routed callers with reserved admission
and opt-in eager feeding (bound two in the finite model), without assuming a CPU
lease or bounded duration for arbitrary Scheme. It is not the default feed-one
configuration or a guarantee against independent unclassified callers.
`sharedInit` adds a candidate cooperative one-unit execution boundary and class
debt: after short execution, eligible bulk work gets a turn. With one reserved
short request per modeled owner, an eligible short worker cannot be overtaken by
more than one new bulk CPU lease. This is an invariant over abstract leases, not
elapsed time, starvation freedom or an implemented general Scheme preemptor.

Eleven scenarios per shared domain retain the failures of queue-only isolation:
a short request can remain before native entry; a reserved worker can still wait
for the same VM's CPU; GC blocks both classes in that VM; an OS pause has no
queue-only deadline. Other scenarios exercise bounded eager feeding, candidate
class debt, root retention through abandonment/close, and four deliberate guard
violations. `service_routing_isolated` separately shows a stateless short request
finishing on another VM while the bulk VM is collecting. It does not migrate
stateful Actors or establish that dedicated capacity improves throughput.

The existing Rust integration gate checks both policies on both domains (1,000
sampled traces, at most 64 steps each; seeds 1 and 42). Three new mutation controls
must exit 1 with an actual invariant violation: premature advancement, CPU guard
bypass and unbounded input feeding. Syntax/tool failures cannot satisfy them.
Together with the existing lifecycle domains this is 79 scenario runs, 6,000
sampled traces and 12 mutation controls. No symbolic result from an older source
is transferred to this extension.

```text
cargo test -p gerbil-scheme-rust-project-harness --test integration_test owner_model::owner_protocol_scenarios_and_controls --locked -- --ignored --exact --nocapture
quint test specs/owner_lifecycle.qnt --main service_sharing_small --seed 1
quint run specs/owner_lifecycle.qnt --main service_sharing_small --init currentInit --step sharedStep --invariant sharedSafety --seed 1 --max-samples 1000 --max-steps 64
quint run specs/owner_lifecycle.qnt --main service_sharing_small --init sharedInit --step sharedStep --invariant sharedSafety --seed 1 --max-samples 1000 --max-steps 64
quint test specs/owner_lifecycle.qnt --main service_routing_isolated --seed 1
```

### Algorithm and native evidence links

| Modeled boundary | Actual implementation or independent gate |
| --- | --- |
| Bounded input feeding, FIFO and existing credit ownership | `instances/service_feed.rs`, `instances/service_loop.rs`; opt-in feed limit, default one |
| Split classes and worker alternation | `scheme/async-service.ss`; existing explicit short-service ABI |
| Per-owner admission and class route | `instances/routing/policy.rs`; cooperating callers only |
| CPU/GC/OS shared spans | opt-in `instances/service_observation.rs`, native diagnostics, directed `scheme_service` bench |
| Root conservation, failure, close and GC/continuations | real `tests/scheme_service.rs` and `tests/scheme_service/priority.rs` |
| Cooperative CPU lease and class debt | opt-in `scheme/service-budget.ss`, `instances/service_budget.rs` and native service tests; participating unit boundaries only, not a general preemptor or performance admission |
| Other-VM GC isolation | model routing contrast; no automatic stateful migration |

`configure_scheme_service_feed_on(owner, maximum)` publishes only immediately
available FIFO service inputs before advancing Scheme, bounded by configured
native slots. It never waits to fill a batch. Reconfiguration requires idle native
slots/work/producer state. This removes one avoidable host-entry gap; it cannot
interrupt an already executing Scheme handler or isolate same-VM GC.

`observe_scheme_service_on` is optional. Owner snapshots record wall time and SDK
thread CPU/GC deltas for submit, advance, wait and retire phases. A long wait span
can include Scheme execution, blocked native select and OS descheduling. GC wall
and thread CPU have different accounting and may overlap; neither residual nor
subtracted aggregate quantiles isolate OS scheduling. Native counter snapshots
and sampled stacks are diagnostic evidence, not consequences of model success.
See status report 30.37 for artifacts, directed A/B and remaining obligations.


### Native cooperative-policy follow-up

Status report 30.38 implements the modeled budget-one class transition for actual
participating Scheme units. Actor children share the request context; a modeled
request's units abstract those child executions rather than identifying every
production thread. Unit waits release and reacquire the Scheme lease. Native
controls cover two-class contention, exceptions, GC, multi-shot continuations
through disable/re-enable and close. The model source remains unchanged; this
correspondence is not a mechanically proved implementation refinement. Unwrapped
Scheme, GC and OS pauses remain outside the cooperative fairness claim.

### Native input port candidate

The existing publish/recheck/notification and root conservation model is reused
for ABI v4 native-port wake. The real API changes mode only after jobs and unit
leases drain, joins the old watcher before root-thread port ownership, and
returns a native receipt before Rust enables its empty-collection shortcut.
Four-VM tests switch both ways, check actual host deadlines, and retain GC,
continuation, failure and close controls. The current finite model does not
mechanically prove port buffering, reader transfer, elapsed deadlines or OS
scheduling; native controls remain separate required evidence. Defaults retain
the watcher bridge and disabled cooperative budget.

### Publisher claim and notification delivery

The unchanged lifecycle model separates publication (`wakeOwed`) from actual
`notify` delivery. It does not model a Rust publisher paused between its pending
flag claim and pipe write, or provide an unconditional elapsed-time guarantee.
The real-pipe fault control injects this claimed-but-unwritten state: legacy
coalescing returns without making the pipe readable. Independent notification
writes while that claim and the legacy writer mutex remain held. Concurrent,
full-pipe and reader-close controls validate the actual descriptor boundary.
Every independent publisher discharges its own notification duty; inbox recheck,
root conservation and close remain the existing protocol. No additional
scheduler model or assumed OS fairness is substituted for native acceptance.

## Stateless admission reselection refinement

The existing lifecycle model now stores `selected` and `committed` owner maps.
`reselectionStep` adds `selectOwner` to the existing lifecycle transitions, with
all original per-owner work/slot, root and close invariants retained. Selection
cannot move a reserved, staged producer or accepted request. Publication records
the committed owner, and safety requires it never to change for an accepted job.
The original `step` retains fixed-assignment traces for historical controls.

Scenarios cover an available other owner, reservation cancellation/reselection,
close/drain and an unsafe post-publication owner move. The aggregate Rust model
gate also samples the reselection step and requires the owner-move mutation to
produce an invariant counterexample. This does not model Tokio FIFO scheduling,
watch-channel internals, kernel wake latency or Scheme execution/GC. Real
semaphore, native multi-VM and matched benchmark controls remain separate gates.
