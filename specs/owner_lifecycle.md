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
| `complete` | `instances/native_dispatch.rs::native_request`, fixed-owner completion |
| `observeTimeout` | Ticket observation/draining; accepted Scheme work is not cancelled |
| `publish` | `instance_runtime.rs::Registry::insert`, closed check under values lock |
| `createExecutor` | `instance_batch.rs::rayon_pool`, closed check under pool lock |
| `canUse`, `release` | Library identity/owner validation and owner-native release |
| `join` | `instances/pool.rs::shutdown`, completed owner drain and native cleanup |
| `clearRegistry`, `retire` | Registry clear, executor slot removal, resource-file retirement |
| `imagesRetained` | Default retention of initialized code; not a live Scheme heap |
| `codeLease`, `releaseCodeLease`, `authorizeCodeRelease`, `quarantineCode`, `releaseCode` | `instances/mapping.rs`: explicit code ownership, provider authorization, actual join and quarantine |

## Safety and abstraction boundaries

- Each request has one immutable assigned owner. A handle can be used only on
  that owner while the host is open. Foreign library identity validation is an
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
accepted work that has not completed, with a per-owner capacity bound. Six
mutations deliberately remove one boundary each: late publication, late executor
creation, early join, foreign handle use, unsafe code release and early credit
return on observation timeout. The code-lease extension also permits cloning an existing
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
Each has sixteen deterministic scenarios and 1,000 sampled traces of at most 64
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
