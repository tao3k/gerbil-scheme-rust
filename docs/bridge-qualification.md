# Producer-first FFI/AOT qualification

Do not advance Orgize's bridge revision on the strength of scalar microbenchmarks,
model simulation, a filename migration, or a successful compilation. Qualify the
producer first. Consumer conformance and complete parser performance admission
remain separate subsequent gates.

## VM execution alternatives

See [VM execution alternatives](bridge-vm-execution.md) for independent worker
ownership, Scheme SMP, safe switching and the current SDK foreign-entry blocker.
`just bridge-vm-capabilities` reports linked ABI prerequisites without entering a
VM. Multiple VM storage alone is not admission for parallel Scheme execution.

## Executable coverage

Standalone-library admission separates three responsibilities: official Scheme
actors own compute scheduling, the bridge owner controls foreign entry and result
publication, and Rust/Tokio owns host admission and awaiter cancellation. Do not
replace either scheduler or equate actor population with processor count.

| Bridge obligation | Formal abstraction | Implementation evidence |
| --- | --- | --- |
| Retained results do not reserve compute workers | Worker reuse with multiple live roots | 10,000-root out-of-order release and value parity |
| Roots survive collection until explicit release | Root retention before terminal completion | Official `gxi` explicit-GC strong-root and release-isolation test |
| Receiver cancellation does not destroy admitted Scheme work | Cancel, release, notify and drain transitions | Tokio abort and subsequent-owner-call controls |
| Foreign copy/release stays owner-controlled | At most one handoff, multiple retained roots | Safe runtime affinity and checked rooted ABI |
| Configuration is not processor activation | Separate activation/admission model and negative mutation | Actual VM count must match the requested count before SMP admission |

QNT checks the declared finite protocol, not the C ABI or generated code itself.
Sampled traces are not exhaustive state exploration. Bounded symbolic checks
cover their stated horizon and finite domains, not unbounded liveness, scheduler
optimality or implementation refinement. Performance is a separate measured gate.

| Entry point | Actual boundary | Admission limit |
| --- | --- | --- |
| `just bridge-workspace` | All producer crates, safe projections, real ABI, compiler and harness tests | Correctness, not parser throughput |
| `just bridge-test` | One-shot setup, released-root rejection, terminal cleanup | One foreign-entry owner |
| `just bridge-root-contracts` | 10,000-root parity, out-of-order release, bignums and owner lifecycle | Correctness, not throughput |
| `just scheme-contracts` | Official module build and explicit-GC strong-root/release isolation | One source-owned `gxi` test process |
| `just bridge-handoff` | Tokio abort after Scheme rooting, concurrent admission, root release before host notification, drain before cleanup | One owner, not Scheme SMP |
| `just bridge-bench-smoke` | Divan fixtures and executable bulk/scalar paths | Smoke only; dev timing is not admitted |
| `just bridge-benchmark` | Release scalar FFI and 0/8 KiB/64 KiB rooted copy/allocation/release | Owner-local microbenchmarks |
| `just bridge-transport-benchmark` | Release Tokio multi-thread admission, Scheme decode/copy/release and host completion | Concurrent callers, one Scheme owner; not SMP execution |
| `just model-check` | Candidate lifecycle safety scenarios and sampled traces | Not implementation refinement or scheduler proof |
| `just actor-model-check` | Actor population, drain, processor activation and negative mutations | Finite safety fixtures, not scheduler optimality |
| `just bridge-actor-smoke` | Full-program AOT actor batches, failure drain and actual VM activation | Rejects an SDK that does not activate requested processors |
| `just smp-check` | SDK actor/SMP capability | Not concurrent FFI performance |

`just bridge-qualify` composes the local correctness/smoke gates without another
shell runner. Default workspace tests now include the ABI targets. CI executes
the workspace once with real test output rather than rerunning the handoff test.
The aggregate local gate likewise runs the crate's complete default test set
once; focused recipes remain available for diagnosis, not duplicate admission.
The FFI/AOT runtime is the default API; there is no opt-in native feature or
compatibility alias. Only external-program linking is a distinct capability.
Release timing runs must be separate from compilation, model checking and other
benchmarks. Do not aggregate build time into parser latency.

## Tokio integration fixture

The host uses the existing Tokio multi-thread runtime, bounded channel,
oneshot completion and task abort mechanisms. Worker count follows available
machine parallelism. Queue capacity is twice that count **for this fixture**;
it is not an adaptive production policy or a proposed bridge scheduler.
One dedicated OS thread creates, calls and destroys the non-Send Scheme runtime.
No Scheme handle or root crosses that thread boundary. No subprocess parses
requests, and no unsafe Send/Sync implementation is added.

Cancellation controls cover queued requests, live roots and released roots
before result notification. Each control aborts and joins the real Tokio task.
The live-root barrier waits until a real Scheme root exists, aborts and joins
the host task, then allows the owner to copy and release that root. The released
token must be rejected before notification fails. Every request also checks the
released token through the ABI before counting its release. Later full-payload
results must still succeed. Closing admission drains every accepted request before runtime
cleanup; reinitialization must return RuntimeFinalized.

Concurrent callers exercise complete batches of 1,000, 10,000 and 100,000 requests with
empty and 8 KiB results. Receipts include completed counts, total batch duration,
queue p95, service p95 and request p95. Service includes direct binary input
copy/allocation, rooting, output copy and release; it is not isolated FFI overhead.

The binary transport benchmark covers all three totals with machine-sized,
oversubscribed and pipelined callers, plus matched Rust-only controls. Totals
describe offered work, not a fixed worker-count policy. The 100,003-request
live case checks incomplete final pipeline windows under the same bounded
admission and exactly-once release contract. Three-sample exploratory runs do
not replace the declared 20-sample performance qualification.
Startup and source construction are excluded. Timestamp instrumentation and
payload verification remain enabled; result disposal is included in batch time,
not the per-request timestamp. These are diagnostic integration receipts, not
unprofiled production parser benchmarks or a statistical regression baseline.
Only real completed-request progress is printed.

## Release phase controls

`benches/tokio_transport.rs` uses a machine-sized Tokio worker pool, with
single-caller, CPU-count-caller and twice-CPU-count-caller controls. Each measured
binary batch crosses 1,000/10,000/100,000 requests with 0/16/8,192/65,536-byte
payloads. The single-caller and legacy hex diagnostic controls remain separate.
These request counts are not document workloads. Initialization and full-payload preflight are outside timing;
task creation, channel admission, Scheme service, result notification, size
validation, host result disposal and joining callers are inside. Divan's one
driver thread does not restrict the Tokio worker pool to one thread. Batch
duration distributions are not per-request p95 latency. The queue capacity is a
fixture bound, not a claimed adaptive scheduling policy. Scheme runtime cleanup
occurs once after all benchmark cases, outside timing, with a real drain count.
Actual Scheme actor/SMP FFI throughput remains an independent missing gate.

The C bridge preserves `___setup_params_reset` VM defaults; it does not assign
`params.parallelism_level` from either a constant or the host worker count.
Foreign-entry owner affinity remains required and is not a VM processor cap.
An executable source regression rejects both single-processor and CPU-count
overrides. Actual activation must still be measured, not inferred from defaults.

SDK flags describe distinct capabilities: `--enable-multiple-vms` provides
independent VM heaps/environments; `--enable-multiple-threaded-vms` allows more
than one OS thread within each VM; `--enable-smp` selects the SMP Scheme
scheduler. The latter does not implicitly enable multiple-threaded VMs.
Official Gambit configuration sets `CONF_MAX_PROCESSORS=1` when
multiple-threaded VMs are disabled, and its resize operation clamps the requested
population to that compiled capacity. Changing bridge worker counts or assigning
C preprocessor macros cannot safely change an installed runtime's ABI/capacity.

`just smp-check` requests the official machine CPU count and rejects an unequal
active count. Both standalone and actor AOT fixtures restore the original active
VM population, not the startup configuration level (zero is a policy value).
`just bridge-sdk-contract <source>` can execute a producer-owned Scheme contract
without copying it or adding an SDK discovery wrapper. Global growth/GC under a
multi-VM build is not proof of independent-VM isolation or SMP throughput.

The actor compute matrix crosses 1,000/10,000/100,000 jobs with 1/8/64 tasks per
requested processor, using 32 rounds per job. Processor budgets come from machine
parallelism; one-processor cases are explicit serial controls. This revised work
shape is not comparable to earlier 1,024-job, variable-round timings. Finite model
domains, benchmark coordinates and production scheduling limits are distinct.

The producer performance entry point is `crates/gerbil-scheme/benches/ffi.rs`,
with lifecycle cases organized under `benches/phases/`. Its workload units are
ABI calls, bytes and live roots, not Org documents. Orgize owns the separate
1,000/10,000-file parsing and query workloads. A bridge transport request is not
an Org file, and the consumer's 100 ms threshold is not a scalar FFI budget.

Lifecycle controls cover 1/32/256/1,024 simultaneous roots: oldest-token length
lookup and complete oldest-first/newest-first create/release population cycles.
Lookup fixtures are outside timing; population construction and disposal are
inside cycle timing. The latter is not isolated release cost. These cases expose
registry scaling without changing runtime affinity or claiming SMP throughput.
The registry now uses the official Gambit strong-reference table with numeric
token equality rather than scanning and reconstructing a linked-list prefix.
No token reuse, weak roots, foreign-thread mutation or new ABI is introduced.
Legacy statistical budget tests still live under `tests/unit/*_benchmark.rs`;
their eventual consolidation must preserve semantic and budget controls rather
than silently delete them. They are not the release measurement entry point.

All transport phases use the same A5 payload at 0, 8 KiB and 64 KiB. Fixture
construction, root creation for copy-only phases, and full-payload preflight
checks are outside the timed closures. Result/root disposal is inside.

- `rust_bulk_copy`: Rust allocation, memory copy and disposal; no FFI.
- `rooted_reused_buffer_copy`: one checked ABI copy into an existing buffer;
  no Rust allocation or root-length lookup. Final payload parity is checked.
- `rooted_bulk_copy`: safe owned result, including length lookup, uninitialized
  Rust allocation, ABI copy and Rust disposal.
- `rooted_create_drop`: Scheme hex decoding, allocation, rooting and release;
  no result copy. This is not a pure allocation benchmark.
- `rooted_round_trip`: complete create/copy/release/result-disposal path.

Binary phases live in `benches/phases/binary.rs`: `create_drop` copies borrowed
input into an independent Scheme bytevector; `reused_output` uses the safe
`copy_into` API without output allocation; `round_trip` returns owned Rust bytes.
They avoid hex encoding/decoding and `CString` entirely. The older hex conversion
phases remain representation controls, not the production binary-transfer path.
`GerbilRuntime::bytevector_from_bytes` accepts all byte values, including NUL.
Input storage is borrowed only during the call. The Scheme body pointer is used
only inside an allocation-free C section and never exported as a Rust slice.
The existing root release and owner-affinity contracts remain unchanged.

Bytevector conversion regression gates run through `just bridge-contracts`.
The Rust syntax-tree gate requires exactly one bulk entry and rejects loops
and scalar reads in bytevector `to_vec` and rooted string `to_string` methods.
Mutation controls restore byte/character loops or replace the bulk ABI and must fail the gate. This is a
targeted source contract, not a whole-program complexity proof. The shared
copy helper is checked from zero to one million bytes for at most one callback invocation,
full payload parity, and fail-closed handling of uninitialized output. Live FFI
checks validate exact-size copying and untouched output on type/length mismatch.
Single-element `u8_at` remains valid for explicit single-element access.
String conversion uses official `string->utf8`/`utf8->string` primitives and
the existing rooted bytevector copy. Independent temporary roots are dropped;
UTF-8 validation remains checked on both sides. The
[complete conversion inventory](bridge-value-conversions.md) lists unsupported
families rather than treating borrowed handle shapes as complete conversions.

The Tokio bridge qualification owner uses official `blocking_recv_many` with a
reusable request vector and the channel's machine-derived capacity as its upper
bound. It processes available requests in FIFO order without waiting for a full
batch, and still releases each root before notification. This is a consumer
integration pattern and benchmark optimization, not a new public scheduler.
Drain receipts report receive batches and maximum observed batch size; accepted
requests must all complete and release before cleanup. A same-payload pure Rust
copy control retains queue admission, host output allocation, notification and
result disposal while excluding Scheme allocation/rooting/copy costs.
Official semantics: https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.Receiver.html#method.blocking_recv_many

An additional benchmark-only admission candidate uses official
[`Sender::reserve_many`](https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.Sender.html#method.reserve_many).
Its per-caller window is `max(1, queue_capacity / active_callers)`; workers and
queue capacity remain machine-derived. It reserves only the final partial
window when a caller has fewer remaining requests. No Scheme handle crosses
threads, and every owned result is awaited and disposed. The integration gate
checks uneven 1,003/10,007-request totals, full payloads, discarded unused
reservations and an abandoned committed receiver: 44,025 accepted requests are
ABI-confirmed released before cleanup. Queue timing includes permit wait.
This is not a new public scheduler or the default admission policy: two matched
runs improved the Rust-only control but regressed complete Scheme batch medians.

The checked legacy hex-input ABI now rejects lengths at or above `isize::MAX`
before constructing a Rust slice or adding a C-string terminator. A sys-level
negative control uses an impossible span without runtime initialization and
requires `InvalidValue` with the output root unchanged. This is a pointer/length
boundary repair, not a new hex transport path or a performance improvement.

### Installed-source review before further optimization

Read the sources shipped under the active installation's `src/`, rather than
downloading or replacing the SDK to investigate optimization. The current
review used `/opt/homebrew/opt/gerbil-scheme@0.19/current/src/` and its matching
`include/gambit.h`. These are implementation references, not proof that this
bridge has already qualified every installed runtime capability.

| Source | Existing mechanism | Bridge decision |
|---|---|---|
| `std/ffi.ss`, `include/gambit.h` | u8vector body access and subtype-aware `___U8VECTORSIZE` | Match these accessors; no independent heap layout or escaping body pointer |
| `std/io/bio/cache.ss` | Capacity-bucket buffer reuse, bounded object cache and native locking | Evaluate the existing cache under the full AOT graph; do not build a second custom pool |
| `std/os/socket.ss` | Caller-provided bytevector and explicit start/end ranges | Keep buffer lifetime and byte ranges explicit at the boundary |
| `std/sync/threads.ss` | Error-propagating joins and thread-group shutdown | Reuse the supported Scheme lifecycle helpers when qualifying actor workers |
| Tokio `mpsc::Receiver` documentation | Bounded channel, receive-many and close/drain | Use the official sync/async boundary; do not replace Tokio scheduling |

Before adopting Scheme buffer caching, verify the official cache's dependency
graph and exact returned capacity, distinguish capacity from logical result
length, and test that retained result leases cannot be overwritten by reuse.
The small embedded bridge must not silently grow a private substitute for the
full std cache or claim cache-backed performance without a matched benchmark.

`tokio_transport` adds binary batches of 1,000 and 10,000 requests with 8 KiB
results and CPU-sized / twice-CPU-sized caller populations. These exercise Tokio
workers and one currently qualified Scheme owner, not parallel Scheme VM entry.

Differences between phases are diagnostic, not an additive cost decomposition:
they have different call counts, allocation behavior and cache/GC histories.
Use matched repeated release runs before claiming a regression or speedup.

## Remaining downstream update gates

The CI installer pins the newest published archive for each supported platform.
As of 2026-10-10, Darwin uses upstream 1cfb032 with patch618d76f9eb72;
Linux's latest published archive still uses 2591dcd with patchf5cedd8168cb.
These are not a matched cross-platform SDK pair. A multiple-VM release label
does not qualify this bridge's multi-VM lifecycle or parallel foreign entry.
Previous local receipts used the older SDK and must not be relabeled as results
against the new Darwin archive. SHA256 and archive metadata checks remain mandatory.

1. Exact-head producer Linux/macOS correctness and explicit ABI CI must pass.
2. Record matched release scalar/bulk baselines with source, SDK, machine and
   timing-boundary identity. Smoke success is not a performance result.
3. Qualify actual Scheme processor admission/publication, GC affinity, cancelled
   awaiters and drain/shutdown before enabling parallel Scheme workers. The
   current single-owner fixture does not satisfy this gate.
4. Measure Rust/Tokio and Scheme actor execution separately at 1,000/10,000
   requests, including tail latency, completed counts and transport costs.
   Do not infer single-VM SMP from multiple process domains.
5. Retain the independent complete 2,000-file ledger threshold below 100 ms.
   Engine EventFold specialization belongs to gerbil-parser, not a second Rust
   parser or a downstream copy of the interpreter.

These remaining gates are not declared complete by adding this document or the
Tokio integration fixture. No Orgize manifest or dependency revision is changed
by this producer-only work.
