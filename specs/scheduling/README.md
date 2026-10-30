# Scheme–Rust scheduling baseline and model map

## Latest integrated PR baseline

The [2026-10-30 replay checkpoint](checkpoints/owner-scope-0402ms-20261030/README.md)
records the accepted frozen `19e164f` replay at 0.402 ms p99 and a separately
built integrated PR tree at 0.350 ms p99. Both complete five million messages
at the original 500k/s rate with zero rejection. Peak debt is still above 128;
neither is full performance admission. The historical-original pair's new
49.849 ms negative replay is retained alongside these results.

## Historical best observed 500k/s checkpoint

The [frozen 0.298 ms observation](checkpoints/owner-scope-0298-20261029/README.md)
remains the historical best observed single-owner latency reference. Its binary,
private VM image and original result have a verified persistent local copy.
The 1,415 > 128 guard failure remains explicit; this is not full admission or
a claim of proven code-optimization benefit. Existing automated gates are unchanged.

The [static observer diff audit](owner_observation_diff_audit.md) identifies
generated hot-path work and same-VM reuse confounds in the rejected 770.831 ms
observer. It adds no observer or workload run.

## Historical 6d4ec27 source checkpoint

The user-authorized [source-owned timestamp checkpoint at 0.317 ms p99](checkpoints/native-source-plan-0317ms-20261027/README.md)
remains the historical source reference for source-debt research. Its 351 > 128
outstanding failure and the same run's anomalous 23.044 ms control are preserved;
the checkpoint is not fully admitted and does not change defaults. The earlier
0.391 ms original remains the historical reference. Do not infer a speedup from
the delayed control or promote the rejected notification-coalescing experiment.

The [owner wait coverage audit](owner_wait_coverage.md), including the
[explicit-generation Scheme diagnostic receipt](owner_wait_scheme.json), distinguishes green-thread
dependencies, native call envelopes and owner CPU absence. It records the exact
restored source identity and missing temporary evidence; its cold report/model
checks are not performance acceptance or a replacement baseline.

The [rejected peak-observer receipt](owner_peak_recovery.json) preserves the
failed runtime experiment and exact rollback boundary. Its
[`owner_recovery.qnt`](owner_recovery.qnt) counterexample distinguishes recovery
under continuing arrivals from drain after a finite source stops. The collector
and cold analysis are test-only; their runtime integration was removed.

## Earlier original-path checkpoint

Use [the original native mailbox arm at 0.391 ms p99](checkpoints/native-mailbox-500k-20261027/README.md)
as the reference for subsequent native optimization. Both arms completed five
million messages at 500k/s with zero rejection, but the opt-in ready-edge
candidate had worse p99 (0.453 ms) and debt. Neither passes the separate original
128-message outstanding guard. Candidate source is retained for reproducibility,
not selected as a default or optimization baseline. The checkpoint includes the
actual measured dependency cohort, complete structured results and negative
evidence; it makes no new production/CI admission claim.

## Original source and component baseline

This work starts on `research/single-vm-service-cost` at `fc82d5a`.
Production Rust, C and Scheme engine sources are unchanged. The first task is
to establish a measured single-VM baseline before changing scheduling.

## Million-message baseline

`just bridge-single-vm-million-baseline` invokes the existing Rust benchmark
with `--million-short-cpu`. It completes and checks exactly 1,000,000 service
messages through the public backpressured API on one resident Tokio worker,
one Scheme VM and one outstanding request. Each message contains one native
order with four lines. It cycles the original 2,048-message input corpus;
each submission still clones the Rust input and creates fresh Scheme objects.
The 32-message warmup is excluded from the million. Counts are service
messages, not HTTP requests, orders pooled across runs, or invoice lines.

Every result is checked with the existing independent Rust oracle. A rejected
request or native error fails the run; it does not disappear from an emitted
successful cohort. Whole-million p50/p95/p99/p99.9/max are computed over actual
per-request samples. Oracle validation and the existing atomic progress
observer remain inside throughput time. Phase clocks and native boundary
diagnostics retain their existing meanings. There is no per-message task spawn.

The original three-arm diagnostic remains available without the new flag.
Its historical 0.366979 ms is the median of six sample p99 values, from 2,048
messages per sample. It is not an observed whole-million p99 or a guaranteed
floor. The new cohort is a distinct benchmark contract; no cross-cohort
percentage is presented as a library speedup.

The VM has one native Scheme execution thread. The benchmark also has Rust,
timer and watchdog threads and is not pinned to an exclusive physical core.
Record both owner-thread CPU and process CPU, with their actual scopes.
GC wall time is not CPU execution. The component benchmark is closed-loop;
zero rejection here does not establish zero rejection under the unchanged
12-VM fixed-arrival load.

The five-second request/no-progress guards and 300-second external capture
limit are retained. Capture stdout, stderr, executable/image hashes and exact
arguments outside Git. Verify the image belongs to the executable's Cargo
build before freezing it; `GERBIL_SERVICE_BENCH_IMAGE` can then explicitly
select that frozen image. The original 12-VM million-message gate remains
separate and unchanged.

## Architectural intent

`scheme_rust.sysml` describes typed Rust inputs/results, the fixed owner C
bridge, a private Scheme heap, Gerbil Actor/green-thread scheduling, native
slot/work budgets, optional ingress/client budgets, optional bounded Rayon
preparation and a twelve-VM deployment target. Native Scheme values never
appear in the cross-owner Rust ports. Rust routes owned input; Scheme retains
evaluation, continuations, GC, exceptions and internal scheduling.

The current default can publish a projected result before the native permit
has returned. Retirement-before-publication is an explicit existing policy,
not a fact about all current paths. Client-consumption quota is another
resource. Optimizations must keep these boundaries distinct. Native cancellation
does not destroy an executing Actor or release its roots early.

The SysML source follows the official
[SysML v2 examples](https://github.com/Systems-Modeling/SysML-v2-Release/tree/master/sysml/src/examples).
It is checked with the installed tool using:

```console
sysml -strict -no-record-cache -validate specs/scheduling/scheme_rust.sysml
```

A syntactically valid deployment target is not a scheduler implementation,
a twelve-owner model-checking result, or a prediction of latency.

## Reuse of the existing Quint protocols

`single_vm.qnt` adds no replacement lifecycle or scheduler state machine.
It instantiates `gerbil_service_architecture` from `owner_lifecycle.qnt`
and `actor_batch` from `actor_batch.qnt`.

| Boundary | Executable source | Existing/refined model |
| --- | --- | --- |
| Native entry only on the fixed owner | `native_dispatch::run_native`, `NativeSession` | `nativeOwnerThenTokioTest`, foreign-entry mutation |
| Gerbil Actors share one VM executor | `scheme/actor-batch.ss` | `fourActorsOneProcessorTest` |
| Tokio/Rayon are distinct from Scheme scheduling | `service_loop::Pump`, producer APIs | bounded preparation, shared-CPU mutation |
| Durable publication and coalesced wake | `Wake::notify_producer`, `Wake::arm` | existing architecture publication/wake cases |
| Cancellation retains native work until drain | `TypedJob::complete`, capacity permits | `cancelledTurnStillDrainsTest`, early-cleanup mutation |
| Publication and native retirement are distinct | `Pump::retire_one`, `retire_before_publish` | existing publication proof cases |
| Continuation reentry and suspension | `service-budget.ss`, `service-runtime.ss` | existing registration/runtime proof cases |

The generic Actor test previously assumed processor 2 existed. The new
one-processor instantiation exposed that test-domain assumption. Its conditional
second start preserves the original two-processor path and adds the valid
one-processor path; no safety predicate or negative control is removed.

Run `just bridge-single-vm-models` for the adapter tests, original Actor
domains, bounded Actor safety exploration and strict SysML validation. The
single-VM host module includes seven new directed traces plus 27 imported
architecture traces; imported traces retain their original CPU configurations.
Do not describe all 34 as new single-core proofs.

The lifecycle/resource model currently fixes its owner domain to two owners.
Reuse it for single-owner and pairwise dependencies; do not silently advertise
it as a twelve-owner composition proof. The SysML twelve-owner deployment is
an explicit design target. Extending composition coverage remains separate.

## Evidence limits and next decision

The models can reject foreign Scheme entry, double-counted CPU occupancy,
lost durable notifications, capacity/result wait cycles and premature Actor
cleanup. They do not prove C/GC memory safety, physical image isolation, OS
scheduling fairness, a historical watchdog root cause, or a microsecond bound.
A million-message benchmark does not require a million-state model.

Choose optimization only after reading the new baseline. Existing evidence
makes owner/client handoff and the publication/retirement boundary candidates;
it does not yet establish their contribution in this million-message cohort.
Compare a candidate with the same profile, count, timing and image ownership,
then use the unchanged twelve-VM production gate. Do not change defaults or
promote an unaccepted candidate.

## First completed million-message cohort

| Metric | Observed |
| --- | ---: |
| Completed / requested messages | 1,000,000 / 1,000,000 |
| Throughput, messages/s | 28,008.109206 |
| Measured loop wall time | 35.703945334 s |
| Whole-million p50 / p95 | 0.026500 / 0.044166 ms |
| Whole-million p99 / p99.9 | 0.375917 / 0.465083 ms |
| Maximum request latency | 7.732000 ms |
| Rejected / native request errors | 0 / 0 |
| Owner-thread CPU delta | 33.915538917 s |
| GC user / system / wall delta | 6.398195 / 0.077232 / 6.453342 s |
| Collections | 17,923 |
| Scheme heap before / after | 6,353,208 / 6,353,208 bytes |
| Roots after / joined / image removed | 0 / 1 / yes |

The whole process, including setup, controls, warmup, summary sorting and
cleanup, took 36.70 s wall, 33.39 s user and 6.08 s system CPU. This is a
different scope from the timed loop and the owner-thread counter. Maximum
RSS includes Rust histogram buffers for all one million requests. It does not
constitute a long-run RSS leak test. Artifact hashes match after measurement.

Construction phase p50/p99 were 1.125/2.709 microseconds, projection
0.333/0.750 microseconds, start-to-constructor 8.000/37.416 microseconds,
constructed-to-projection 9.125/337.875 microseconds and client recovery
7.667/23.708 microseconds. These marginal percentiles are not one request
and must not be added. The service interval includes Scheme queueing, execution
and GC; the current data cannot assign its p99 to one cause. GC wall time is
about 18% of loop wall time, not an additive attribution of end-to-end p99.

The next design comparison should distinguish service-cycle allocation/GC,
owner execution and client handoff. Native item credits (8 for a bulk message)
and Scheme record execution cost (48 per bulk item) are different units.
Retirement/publication order is an existing policy candidate, not a selected
new default. This observation does not establish twelve-VM scaling or a
production optimization gain.

## Same-workload scaling versus production admission

The short-CPU single-VM cohort and the older twelve-VM mixed-message cohort
differ in native work per message, outstanding work and arrival policy. Their
p99 values cannot quantify a same-workload multi-VM penalty. The SysML model
therefore names two distinct twelve-VM targets:

- `scalingTarget`: the same short corpus and public API, 1,000,000 total messages,
  one outstanding request per owner, with per-owner throughput/CPU and global
  latency reported. The first completed comparison is documented below.
- `productionAcceptance`: the unchanged 1,000,000-message, 16:1 bulk/short fixed
  arrival contract. It measures offered, completed, rejected, latency and
  fairness separately. A closed-loop scaling result cannot admit this gate.

Three architectural alternatives remain explicitly distinguishable: synchronous
native batching, a resident Gerbil Actor service with durable publication, and
that service with optional bounded Rayon Rust-input preparation. The inherited
architecture tests cover wait cycles and duplicate scheduling/CPU accounting.
They do not claim a resident protocol is already implemented beyond the current
Scheme service API. The native-cycle diagnostic callback remains qualification
only and cannot bypass production admission or cancellation.

## Measured artifact identity

This measured cohort starts from engine revision
`fc82d5a0579f5e867a67a7850a81b6393b2fcc89` with only the benchmark/model changes
in this branch. The whole benchmark source, executable and selected image
were checked after measurement. The selected image was first checked against
the actual Cargo build output and executable default.

| Artifact | SHA-256 |
| --- | --- |
| Benchmark source | `164ae50db881515a98d76935feb674b479e83a0d93cef771ca14d380950b3889` |
| Measured executable | `b79d6f4956c1a3f4c07a6a7be4f432ffb7b176b87052fb41ccf334f48c530be0` |
| Loaded VM image | `271d52efd66b04999fbbbcca0c9d49cd5cc2d55e0d5cbd23b763dbaa40cb24ec` |

Host: Apple M2 Pro, 12 physical/logical cores, 32 GiB RAM, macOS 26.6.1
(25G76), Rust 1.95.0. SDK release identifier:
`0.19.20261009110849.1cfb032.patch618d76f9eb72`; selected SDK build `1cfb032`.
Dynamic Scheme compilation uses optimization level 2. The Rust bench uses
the existing optimized benchmark profile and `arrival-clock` Cargo feature.
The measurement does not create an arrival-clock stream: it is the explicit
closed-loop, one-outstanding single-VM mode.

Use `env GERBIL_DYNAMIC_MODULE_OPT_LEVEL=2 just bridge-single-vm-million-baseline`.
An external capture controller bounds the process to 300 seconds; the existing
benchmark itself enforces its five-second request/no-progress guards.
Raw captures, histograms, executables and native images stay outside Git.
Native image bytes can differ after rebuilding on another path/toolchain;
record their identities rather than silently mixing different Cargo outputs.

This branch publishes the measured baseline and research models. It does not
update PR 18, change executor defaults, claim a twelve-VM speedup, or qualify
remote CI merely because local checks passed.

## Comparable fixed-owner scaling

[Fixed-owner scaling](fixed_owner_scaling.md) records the new one/twelve-owner
million-message comparison, its artifact identities and the first backend-mode
read experiment. The baseline sections above describe the published unchanged
engine cohort; a local candidate does not replace that baseline or qualify the
mixed open-arrival gate. Run `just bridge-fixed-owner-scaling 1` and `12` as
separate processes. Run `just bridge-backend-choice-model` for the isolated
shared-mode primitive; existing owner/Actor protocols remain authoritative.

## Duration-driven short concurrency research

[Bounded short concurrency](short_concurrency.md) records the manual-grounded
class/projection/native-retirement contract and same-live-VM 1-versus-3-slot API
experiments at 500,000 and 1,000,000 planned arrivals/s for ten seconds per arm.
The retained [two-second burst](single_vm_open_500k.md) is historical and is not
the current duration-driven acceptance contract. Neither replaces the published
closed-loop or mixed fixed-arrival references. Current candidates are not admitted.

- [Bounded continuous short service](continuous_short.md): one resident producer,
  independent result consumption, per-request expiry and bounded host/native
  ownership; refines the existing short-concurrency model.
- [Gerbil Actor supply opportunities](actor_supply.md): distinguishes native
  active capacity from actual bounded publication before Scheme advancement.
- [Bounded Actor completion custody](actor_completion.qnt): native result
  snapshots retain independently projected/retired messages, including partial
  consumption followed by cancellation and close.
- [Native lifecycle and fixture ownership](native_lifecycle.md): preserves loader/SDK
  coordination and excludes application finalizers from the process gate.
- [Typed Actor input construction](native_construction.md): same graph and quota,
  one external constructor root; opt-in comparison on the original service policy.
- [Typed Actor evidence chain](typed_actor_evidence.md): source identity,
  ownership checks, complete outcomes and explicit failed capacity admission.
- [Gerbil-native Actor arrival reference](native_actor_reference.md): same business
  workload in raw mailboxes and the existing service, without per-message Rust entry.
- [Native ready-edge handoff](native_ready_wake.md): optional completion-edge
  coalescing over the original frames, CAS notification ownership and round-robin
  result consumption; model and native gates remain distinct.
- [Continuous backpressure](continuous_backpressure.md): wait for existing window
  credit without payload/root preparation, while consuming results independently.
- [Rejected typed mailbox service](mailbox_service.md): archived native completion
  Actor and failed capacity receipts, excluded from the active candidate.
- [Owner-local native type exchange](native_exchange.md): synchronous copied
  input, scalar identity and independently consumed completion slots.
- [Shared continuous lease](continuous_lease.qnt) and [publication deadline](publication_deadline.qnt): joint canceled-input custody and original-deadline commit checks.
