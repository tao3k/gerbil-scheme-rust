# VM execution alternatives

## Ownership and selection

The intended alternatives share the same Scheme AOT program and application
semantics, not the same mutable Scheme heap:

1. Rust schedules requests onto dedicated OS workers. Each worker initializes,
   enters and destroys its own single-processor VM. Tokio owns asynchronous
   admission and backpressure; CPU-bound Scheme execution stays on the VM owner.
2. A qualified SDK supplies one VM with its Scheme SMP scheduler. Foreign entry
   retains its checked owner boundary; Scheme schedules actors internally.

The second alternative remains provisional pending SDK repair and qualification.
The default bridge does not automatically select either alternative. An explicit
experimental Darwin image backend is now available through `InstancePool`; see
[the library API and lifecycle contract](instance-runtime.org). Portability,
hot unloading and general production failure handling remain unqualified.
Do not add a selectable executor that silently delegates to the existing single
owner and reports that serialization as independent-VM parallelism.

Selection must be explicit and capability-checked. Changing executor while
running requires stopping admission, draining accepted work, releasing roots and
joining owners before constructing the replacement. Scheme roots, closures,
module globals and VM pointers cannot migrate between backends. Only owned host
values can cross that boundary. The existing process-global runtime is one-shot:
it cannot implement an in-process switch by cleanup followed by initialization.
Until independent lifetime support exists, selection is a startup decision,
not a live-switch promise.

## Current SDK boundary

On 2026-10-10 the selected Homebrew SDK is Gerbil `1cfb032`, with Gambit source
`ea114fc3d2f120abbe20393c1f9aeafdd3f8c89f`. Its build receipt enables
`multiple-vms` and `smp`, but not `multiple-threaded-vms`.

The matching installed header and local official source show:

- In `include/gambit.h.in`, `___SINGLE_THREADED_VMS` selects
  `___GET_REAL_PSTATE()` as `&___GSTATE->vmstate0.aligned_pstate[0].pstate`.
- In that branch `___SET_REAL_PSTATE(ps)` is empty. Ordinary generated foreign
  entry does not select an independently supplied per-worker processor state.
- `___PSA` also omits explicit processor arguments in this configuration.
- `___setup_vmstate` in `lib/setup.c` initializes VM memory and processor state;
  calling it alone does not establish the complete Gerbil program lifecycle or
  make generated foreign entry select that VM.
- The bridge's C and Rust runtime lifecycles both retain a process-global,
  non-restartable owner. Removing their guards would not fix SDK entry routing.

Therefore multiple VM storage is not sufficient evidence for parallel entry
through this SDK's ordinary ABI. This is a configuration/ABI finding, not proof
of the separately reported upstream multi-threaded-VM bug.

`just bridge-vm-capabilities` observes compile-time capabilities from the same
bridge compilation that uses the selected SDK. It does not initialize Scheme,
resize a VM, change SDK flags, or admit an executor. A future true thread-local
entry bit is still only a prerequisite, not a completed multi-VM qualification.

The linked probe completed with both regressions passing on the current SDK:

```text
VM-ABI multiple-vms=true thread-local-entry=false max-processors=1 independent-worker-admission=unqualified
```

Workspace Clippy, formatting, filename and diff checks also passed. The
`bridge-contracts` recipe passed, including 37 safe API/source tests, 444,031
accepted/released handoff requests with clean drain, and the AOT build's 21 unit
and 20 integration tests (five previously ignored build scenarios unchanged).
The capability query itself creates or admits no independent VM, and no
performance claim follows from that observation. The separate private-image
diagnostic below does create isolated runtime instances.

## Required implementation gates

Before independent workers are admitted, the runtime supplier must provide a
qualified independent-context foreign-entry ABI. On the current SDK, an
isolated runtime-image strategy would be a different architecture requiring
separate approval and portability qualification, not an ordinary FFI fix.

After the entry boundary is resolved, bridge-owned qualification must establish:

- Independent module initialization and mutable globals for each VM.
- Simultaneous Scheme execution on distinct OS workers, not merely concurrent
  queue callers or distinct processor identifiers.
- GC/root isolation, owner-affine destruction and no cross-VM handles.
- Setup failure rollback, panic/exception containment, cancellation, bounded
  backpressure, drain and restart/switch lifecycle behavior.
- Matched 1,000 / 10,000 / 100,000 request scenarios, with payload size,
  initialization cost, throughput, latency distribution and memory reported.
- A Quint model of the bridge ownership/admission/switch protocol, without
  claiming to re-prove Tokio or treating finite simulations as runtime evidence.

No kernel modifications, processor macro overrides, unchecked `Send`/`Sync`,
per-request processes, or reduced performance thresholds are authorized by this
design. Downstream Orgize should not change executor until the upstream gates
pass.

## Private runtime image diagnostic

The `instance-probe` qualification feature now builds a Darwin private runtime
image from the compiler-owned full Gerbil AOT graph and the installed static
Gambit archive. The host loads a separate image file for each dedicated OS
worker. Each image follows the official `___setup_params_reset` / `___setup` /
`___cleanup` embedding lifecycle without SDK macro overrides or kernel changes.
This isolates global runtime states; it is not a successful secondary-VM bind
inside a single shared Gambit runtime. It does not require a multi-threaded VM.

Only five adapter exports are public. `nm -gU` confirmed that export list and
`otool -L` confirmed no shared Gambit library dependency. Setup and teardown
alone are serialized because they install process-wide OS hooks; no mutex wraps
Scheme work. Tokio admits batches to dedicated, thread-affine Scheme owners.

The 1,000 / 10,000 / 100,000 job smoke scenarios passed for both one owner and
the machine-sized 12-owner pool. Every multi-owner case checked:

- Twelve distinct Gambit global-state identities.
- Twelve simultaneous callbacks originating inside Scheme execution, following
  a GC in each VM. This checks overlapping Scheme entry stacks, not merely
  queued callers. It does not establish a particular multicore speedup.
- A different Scheme mutable global marker in every VM, checked after GC and
  after the complete work batch.
- Exact synthetic checksum totals and owner-affine teardown/join.

The workload is a synthetic checksum, **not Org parsing**. These receipts do
not satisfy the parser's 100 ms admission threshold or complete production
multi-VM, cancellation, switching, or SMP qualification.

The first prototype stalled because the host used an iterator awaiting closure
of an initialization channel while the workers retained its sending ends.
It was terminated, not admitted. Collection now consumes the exact expected
number of identities with the existing five-second startup failure gate, and
workers drop their initialization senders immediately after publication. All
initial six smoke cases subsequently passed, followed by all twelve light/heavy
smoke cases; the startup defect was in the diagnostic
host, not proof of a Gambit initialization defect.

Image code remains loaded until process exit. Process-wide signal/timer hook
ownership, safe unload and bounded image reuse are unresolved. There is no
production executor API or hot-switch claim. Image copies are kept as diagnostic
artifacts, and their cost must be included before adopting this approach.
The prototype preserves the SDK's setup defaults. A future SDK with a different
default processor population needs separate per-image resource qualification;
this receipt does not admit nested SMP pools or automatic compatibility with
the repaired SDK.

Commands use Cargo/Just directly:

```text
just bridge-instance-probe --test
just bridge-instance-probe
just bridge-instance-lint
```

The original `actor-aot` path remains separate and available via
`bridge-actor-benchmark`. Its requested/active SMP gate is unchanged; it cannot
be replaced with a private-image result or silently downgraded to one processor.

The qualification build now excludes its own staging outputs from Cargo's
rerun inputs while retaining original producer sources and SDK source/header
inputs. A functional regression checks this source/output boundary.

### Complete local timing diagnostic

The final 20-sample diagnostic covered 12 scenarios: three job totals, two
work amounts (32 and 4,096 checksum rounds/job), and one versus machine-sized
12 owners. All scenarios checked exact results and completed owner teardown.
The host records actual completed batches, not synthetic progress. Startup and
each awaited response have a five-second failure gate; a stalled response exits
nonzero and explicitly reports cleanup as unverified.

Medians from this busy-host run (milliseconds):

| Jobs | 32 rounds, one owner | 32 rounds, 12 owners | 4,096 rounds, one owner | 4,096 rounds, 12 owners |
|---:|---:|---:|---:|---:|
| 1,000 | 0.8463 | 0.9277 | 10.71 | 9.304 |
| 10,000 | 1.662 | 3.913 | 77.75 | 23.38 |
| 100,000 | 6.363 | 1.695 | 1,468 | 234.6 |

The full run processed 8,880,000 timed synthetic jobs plus 444,000 preflight
jobs, not that many individual foreign calls or parser documents. Each worker
receives one partition batch; jobs are computed in Scheme. Timing includes
per-batch GC, Tokio handoff, checksum validation and completed-batch reporting.
It excludes image copying, loading, runtime initialization and teardown. Divan's
one coordinator thread is not the worker count; Scheme work runs on the
machine-derived dedicated owners.

Large outliers remain: for example, the heavy 100,000-job single-owner sample
range was 710.7 ms to 3.03 s, and the 12-owner range was 75.06 to 366.8 ms.
These are diagnostic observations under concurrent host work, not stable
speedup claims, production performance admission or the historic Org corpus.
Earlier light-work diagnostics were also sometimes slower with 12 owners;
do not hardcode machine-sized fanout as the best choice for every workload.

Both-feature Clippy, formatting and diff checks passed. The original Scheme
actor path's nine single-processor smoke controls passed after these changes.
Its machine-processor case still failed the strict 12-requested/1-active gate
at the first scenario. That rejection is not replaced by image-pool evidence.
The new switching protocol model passed typecheck, three deterministic tests
and 1,000 traces of at most 50 steps; external lifecycle qualification remains
an assumption and no symbolic proof was claimed.


## Fixed-owner Scheme lifecycle receipt (2026-10-19)

The [native instance lifecycle and matched A/B receipt](30-39-status/30.03-instance-owner-qualification.org)
records executed continuation, GC, dynamic binding, cleanup and surviving-instance
checks, plus 20 ABBA samples per variant/load. Each private runtime now explicitly
requests and verifies one processor. This remains the private-image diagnostic;
production `GerbilRuntime` ownership and admission are unchanged.


## Library integration and native Scheme JSON receipt (2026-10-19)

The opt-in `gerbil-scheme/isolated-instances` feature now exposes fixed native
owners and a cloneable `InstanceDispatcher` in the library itself. The
[library-path qualification](30-39-status/30.04-instance-library-qualification.org)
executes SDK JSON parsing/writing and Scheme aggregation through that API,
checks continuation and queue/drain contracts, and records equal-load ABBA and
fresh-process initialization/cleanup costs. All Scheme execution and retained
state remain in their owner-local native runtime.

## Native typed calls and resource boundaries (2026-10-19)

The [native typed qualification](30-39-status/30.05-native-instance-resources.org)
uses the existing Scheme type/root ABI through `submit_native_on`, without JSON.
It records initialization fault injection, 65,536 reused-owner tasks with Scheme
exceptions and Rust panics, a deliberate Scheme heap retention control, and
replacement costs. Fixed pool reuse reaches a stable post-GC live plateau;
initialized images and copied files remain retained after shutdown, and RSS
reclamation is not established. See the [runtime API](instance-runtime.org) for
the owner-local root lifetime and phase-aware initialization contracts.


## Terminal retirement and Scheme-owned scheduling (2026-10-19)

The [retirement and green scheduling receipt](30-39-status/30.06-instance-retirement-and-green-scheduling.org)
separates allocator retention from loaded images and fixes two terminal resource
issues: copied image paths after known cleanup, and SDK fdset scratch buffers
left in finalized image state. ABI v3 reports both native I/O bytes retired and
filesystem cleanup errors. Native code remains mapped; copied-path deletion is
not hot unloading or a promise of backing-store reclamation.

Rust routes typed work to fixed owners. Scheme schedules persistent local
mailbox actors and green threads inside each owner. Qualification measures
batching, actor count, explicit yielding, queue-to-projection latency, timer
wait overlap, and four-instance I/O with forced collection. The native compiler,
collector and Scheme execution semantics remain in place.


The [public native batch API qualification](30-39-status/30.07-native-batch-actor-api.org)
connects bounded native numeric batches to a reusable Scheme mailbox service.
It measures matched order aggregation workloads, owner completion tails and
intra-Scheme fairness. Rust routes instances; Scheme schedules green workers.


The [tail attribution and paired same-VM A/B](30-39-status/30.08-native-batch-tail-attribution.org)
separates owner dispatch, native construction/GC, Scheme execution and result
projection. It retains a small Scheme scheduling repair and documents the
rejected coordinator-reuse experiment and the remaining p99 boundary.


## Private-image heartbeat ownership

The paired Darwin SDK uses `ITIMER_REAL` and `SIGALRM`. These are process-wide,
while each statically linked image has its own Gambit interrupt state. Installing
a second image's signal handler directed heartbeats to the last image; cleaning
up an image could disable every image's timer. A bounded downstream control
passed with one VM and failed with two live VMs before either VM was retired.

`ffi/instance_timer.h` supplies image-local bindings for this SDK timer seam.
Host callers still resolve libc's original timer and signal functions. Each
image owns one joinable OS notifier in addition to its single Scheme execution
and GC owner. The notifier only calls the SDK's asynchronous interrupt primitive;
it never enters Scheme or allocates Scheme objects. Its complete thread return
is joined before native heap cleanup and image retirement. All notifier threads
must therefore be included in process resource accounting.

Heartbeat masking, pending notification coalescing, periodic and one-shot timer
state remain private to the image. Only the VM owner changes the virtual mask;
foreign OS threads retain their own signal mask. The adapter targets the paired
SDK's ordinary `sigaction` handler, and rejects incompatible handler modes.
Clock, thread creation and join failures cannot establish clean native teardown.
This does not prove the SDK interrupt implementation or general-purpose signal
handler equivalence for arbitrary foreign modules.

The first candidate used libdispatch. It was rejected: the unchanged five-second
mapping resource gate retained two global workqueue threads after all VM owners
had exited. A separate delayed-return control also distinguished a signaled
completion from complete callback return. Neither gate was relaxed. The joinable
notifier returns process thread counts to their original baseline, including
repeated native initialization, GC, typed calls, continuation use and unmapping.

Reproducible controls are part of the existing native CI job:

```sh
cargo test -p gerbil-scheme --features tokio-dispatch --test unit_test instance_timer --locked -- --test-threads=12
cargo bench -p gerbil-scheme-qualification --features instance-dispatch --bench instance_resources --profile dev --locked -- --mapping-release
```

The standalone timer control preserves a live host `SIGALRM` timer, checks two
independent images, owner masking and pending delivery, invalid intervals,
retirement while another image continues, and actual `dlclose`. A test-only
notifier return tail checks that teardown waits for thread exit. These controls
complement application semantic tests; they are not latency or source-refinement
proofs. The existing code-lease and quarantined-image rules remain in force.
