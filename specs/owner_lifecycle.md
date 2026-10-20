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
| `complete` | `instances/native_dispatch.rs::native_request`, fixed-owner completion |
| `observeTimeout` | Ticket observation/draining; accepted Scheme work is not cancelled |
| `publish` | `instance_runtime.rs::Registry::insert`, closed check under values lock |
| `createExecutor` | `instance_batch.rs::rayon_pool`, closed check under pool lock |
| `canUse`, `release` | Library identity/owner validation and owner-native release |
| `join` | `instances/pool.rs::shutdown`, completed owner drain and native cleanup |
| `clearRegistry`, `retire` | Registry clear, executor slot removal, resource-file retirement |
| `imagesRetained` | `instances/image.rs` and `retirement.rs`: initialized images are not unloaded |

## Safety and abstraction boundaries

- Each request has one immutable assigned owner. A handle can be used only on
  that owner while the host is open. Foreign library identity validation is an
  additional native gate, not modeled as a second host here.
- Timeout changes observation state without withdrawing admitted native work.
- A join requires all accepted native tasks to finish. Registry metadata is
  cleared before host retirement; a completed reply copy may still arrive later.
- The closed host has no roots, registry entries, live executor slot or resource
  files. Initialized code images deliberately remain retained. Root/file removal
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
or foreign handle use. Four mutations deliberately remove one boundary each:
late publication, late executor creation, early join and foreign handle use.
The Rust integration test requires exit 1 **and** `error: Invariant violated`; parser, tool
or dependency failures cannot satisfy a negative control.

The small domain has three requests and two owners; the wide domain has five
requests and two owners. These are exploration parameters, not runtime limits.
Each has eight deterministic scenarios and 1,000 sampled traces of at most 64
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

The eight-transition symbolic run completes with `NoError` on the unchanged
three-request domain. The twelve-transition attempt is stopped incomplete as
later SMT queries become expensive. It is not a pass, and the eight-step result
does not replace that target. Symbolic wrappers preserve all generated output
in a fresh temporary directory outside the checkout.
