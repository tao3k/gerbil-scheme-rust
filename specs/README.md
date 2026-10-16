# Cross-runtime FFI/AOT handoff

This model covers the bridge-owned candidate protocol, not Tokio's scheduler
or Gerbil's actor implementation. Neither scheduler is reimplemented here.
The currently shipped bridge remains a single, thread-affine runtime owner.
Passing the candidate model does not enable parallel foreign calls.

`bridge_handoff.qnt` checks bounded admission, exclusive request ownership,
root retention across cancellation, exactly-once terminal completion and the
drain-before-cleanup barrier. It separately tracks the Rust awaiter, native
completion and result notification. Dropping a Rust receiver never implies
that admitted Scheme work has stopped. Notification settles a dropped receiver
without delivering a result, and cannot precede root release.
Multiple results may remain rooted simultaneously. The foreign copy/release
critical section is a subset of live roots, not the entire root population.
Retaining a completed result does not reserve a compute worker; a regression
scenario reuses the same worker while two results remain rooted.
Worker parking cannot retire an active owner;
queue-budget reduction cannot discard admitted requests. Resource domains and
budgets are parameters. The small and wide instances are finite checker
fixtures, not production worker counts or request limits.

## Scope and evidence

`actor_batch.qnt` separates runnable actors from simultaneous processor leases.
Actors may outnumber processors; finite checker domains are not production caps.
Its processor-admission module distinguishes a requested configuration from
successful VM processor activation. A configuration-only admission mutation is
rejected. `just actor-model-check` covers 13 deterministic tests and three sets
of 1,000 sampled traces. Both actor lifecycle and processor admission have passed
12-transition bounded symbolic checking, not implementation refinement or a
performance proof. `just processor-model-verify` checks admission separately.
The runtime probe checks `##current-vm-processor-count`, not the configuration
getter. An activation failure must prevent SMP benchmark admission.

- `just model-check`: type checking, deterministic protocol tests, and 1,000
  simulated traces of at most 50 transitions for each finite instance.
- `just model-verify`: symbolic safety checking through 12 transitions. It is
  not an unbounded theorem, a liveness proof or an implementation refinement.
- The intentional early-cleanup mutation must violate `safety`; its direct
  simulator check returned 1 with an invariant violation, not a syntax error.

`model-verify` passes `specs/apalache.json` to the official checker. It selects
`search.invariant.mode=after`: all possible transitions are translated before
checking the invariants, reducing repeated SMT queries. The model, invariant
set and 12-transition bound are unchanged. No transition/invariant filter or
random-transitions option is enabled. This trades additional solver memory for
fewer queries; it does not reduce verification coverage. See the official
[Apalache invariant-mode documentation](https://apalache-mc.org/docs/apalache/tuning.html#invariant-mode).

On 2026-10-10, the unchanged small handoff instance completed this command with
exit status 0 and `NoError`: all 17 generated state invariants held through step
12. Reported checker duration was 1,648,608 ms. This is a finite bounded safety
receipt, not an unbounded proof, implementation refinement, or latency result.
- `just smp-check`: standalone official SDK capability and actor
  checksums. This is not concurrent FFI admission or a speed measurement.

The processor-admission model also covers SDK capacity clamping: a request for
12 processors resized against capacity 1 remains unadmitted. SMP scheduling,
multiple independent VMs and multiple OS threads within one VM are separate
build capabilities. The newly added clamp transition changes the model; older
symbolic receipts do not prove this revised transition system. Deterministic
tests and simulation are reported separately from a fresh bounded proof.

The earlier simulator execution passed ten protocol scenarios and both trace
samples. The expanded handoff model adds dropped-receiver, queued-cancellation
draining and premature-notification mutation scenarios. The 12-transition
symbolic run was deliberately interrupted when the
verification scope was narrowed; it is not admitted as a pass. Do not describe
random simulation or partially completed symbolic checking as exhaustive proof.
Quint 0.33.0 and its official Rust evaluator 0.7.0 were used locally.

The model separates native completion/root release from host notification,
but still abstracts copying and root release as one transition. It does not
establish actual GC/thread affinity, callback memory
ordering, cancellation of a host task, or successful runtime shutdown. Those
require native implementation traces and negative integration controls.
Fairness, stalled foreign calls and eventual progress are not yet proved.

`just bridge-test` extends the existing one-shot ABI test, without another
runtime initialization. Its dropped-receiver control checks actual bytevector
copy, owner-side root drop, failed host delivery, rejection of the released
root token and a successful subsequent Scheme call. `just bridge-handoff` adds
a real Tokio task-abort control after Scheme rooting, plus concurrent transport
batches and owner-side drain/cleanup checks. Neither test is an SMP parser test
or implementation refinement of every model transition. The C/Scheme callback
and parallel processor shutdown paths still need corresponding
trace-driven controls. Runtime namespace naming is responsibility-based;
Rust files have actually moved from `src/native/` to `src/runtime/`; there is
no legacy path attribute or forwarding module. The lifecycle and round-trip
test files/targets are `runtime_lifecycle` and `runtime_round_trip`.
The SMP probe file is `scheme/runtime-smp-test.ss`.
Scheme source, SSI and generated SCM are now `scheme/runtime.*`; the namespace
is `gerbil-scheme-rust/scheme/runtime`. The producer regenerates SCM and checks
both provenance hashes successfully. Tracked SSI is byte-identical to the fresh
compiler SSI. The fresh Scheme scalar module load/ABI control also returns zero.
The C lifecycle file is `ffi/runtime.c`, the build owner is `compiler.rs`, and
the tool environment file/module is `tool_environment`. Test and benchmark
targets no longer use a redundant native prefix. Cargo metadata resolves the
new targets and filenames, with no compatibility forwarding files.
The first real-ABI test compilation was interrupted before moving its target
files; no passing receipt is claimed for that run. The expanded handoff model
passes eleven scenarios in each of two instances and both 1,000-trace samples,
including a receiver dropped after native completion but before notification.
Parallel Scheme workers serialize the root publication/copy/release handoff;
the mutation control rejects overlapping foreign handoffs. These bounded
checks do not establish scheduler refinement, liveness or latency.
The renamed `runtime_lifecycle` real-ABI regression passes, including its
abandoned-receiver/root-release control. Gambit derives its linker identity
from the generated C basename; `runtime.scm` and `runtime.c` must agree. The
separate lifecycle shim is archived as `lifecycle.o` to avoid object collision.
The bridge contracts pass 32 safe-surface tests, four separately initialized
ABI targets and 20 build-owner tests. All renamed Rust test and benchmark
targets pass type checking; both touched packages pass all-target Clippy with
warnings denied. These receipts do not qualify downstream Orgize's
old pinned products or a concurrent Scheme parser runtime.

## Performance owners

`vm_execution.qnt` models only explicit route admission and drained switching.
Its three deterministic tests and 1,000 simulated traces of at most 50 steps
passed locally on 2026-10-10. Completed external qualification is an assumption
of `qualify`, not something established by the model. The request bound of three
is a finite exploration parameter, not a production queue limit. The private
runtime-image prototype has not established reusable lifetime, signal ownership
or real executor switching; no symbolic proof was run for this new model.

Rust qualification uses existing Tokio bounded channels, owner-thread handoff
and Divan benchmarks.
Scheme uses official actor/thread/SMP machinery and native benchmark APIs.
They share compiled parser semantics, not a scheduler or adaptation policy.
One process-global VM with multiple processors is not multiple independent VMs;
no multi-VM capability is admitted by this model or the standalone SMP probe.

Adaptation must respect available CPU, processor support and memory budgets,
then use measured queue wait, service time, throughput and allocation pressure.
Rust blocking-pool occupancy and Scheme processor occupancy are different
measurements. A blocking foreign wait can stall a Scheme processor even when
it is correctly offloaded from a Tokio asynchronous task. Benchmark that
boundary instead of proving the mature schedulers again.

Compare full 1,000/10,000-document batches under both execution owners, including
event encoding, copying, decoding, projection and completed-result disposal.
Keep warmup, corpus construction and runtime initialization outside the matched
timing boundary. Preserve independent complete 2,000-file ledger admission
below 100ms. Model-checker wall time is not parser latency. Do not run parser
benchmarks concurrently with compilation or symbolic verification.

Next implementation work belongs in the bridge's lifecycle/processor and
publication interfaces. Do not add unsafe `Send`/`Sync`, arbitrary parallel
foreign entry, a custom Tokio replacement, kernel changes or parser processes.
