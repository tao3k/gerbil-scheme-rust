# Single-VM Gerbil service costs

This component benchmark starts from source revision
`6f32a10f56420005b2effcc9465b45f309f14ccf`. It does not replace that revision's
12-owner open-arrival or capacity measurements. No executor defaults or
production protocol rules change.

Run through the existing Cargo/native build:

```console
env GERBIL_DYNAMIC_MODULE_OPT_LEVEL=2 cargo bench --locked -p gerbil-scheme-qualification --features instance-dispatch --bench single_vm_service -- --test
env GERBIL_DYNAMIC_MODULE_OPT_LEVEL=2 cargo bench --locked -p gerbil-scheme-qualification --features instance-dispatch --bench single_vm_service
```

The first command is a correctness smoke run. The second records three forward
and reverse blocks per profile. Native images still follow Gerbil/Gambit to C
and optimized native compilation. No Python benchmark or separate build driver
is used. Capture stdout/stderr outside Git. Record the measured executable and
image SHA-256, source revision, host/SDK identity and compiler configuration with
the capture; do not commit private paths or raw captures.

## Actual comparison boundaries

All arms retain the same live VM, full service endpoint, native numeric payloads,
order handler, fresh input construction and owned result projection:

| Arm | Actual execution |
| --- | --- |
| `owner_native_service_cycle` | The normal Rust native owner queue enters a qualification callback. Scheme drives submit, ready, its own condition/port wait, and take on the existing endpoint. |
| `caller_driven_service` | The public backpressured service API is driven by a current-thread Tokio runtime. |
| `tokio_worker_service` | The identical public API runs on one Tokio worker; the task remains resident for the sample. |

The first arm includes the Rust native owner queue and host result delivery.
It is **not a pure Scheme upper bound**. Unlike the historical op-6 handler
reference, it includes the actual Scheme service queue and workers. The second
arm still uses Tokio admission/deadline primitives; it is not a Tokio-free
synchronous service API. The third measures worker-driven execution without
artificially spawning a fresh task for every message.

Op 49 is qualification-only. Its caller must exclusively own the idle service,
use Snapshot completion, and have no outstanding Rust service tickets. The bench
drains the existing slot/work credits before switching arms. This callback must
not be used to bypass production admission, cancellation or close tracking.

There is one VM and one outstanding message, four service workers and four
Gerbil Actors per bulk lane. This intentionally isolates component costs; it
does not measure saturated production capacity or cross-owner fairness. The
three profiles are:

- `short_cpu`: one order with four lines and the existing explicit short tag.
- `bulk_cpu`: eight orders with 48 lines each through the real Actor batch.
- `bulk_mixed`: the same bulk corpus with a 1 ms Scheme green-thread suspension
  every seventh order.

Input content and the independent Rust invoice oracle are shared with existing
Rust benches. The corpus is prepared before timing. Every arm clones the Rust
input and constructs fresh native objects within timing. Short requests retain
the short service class. Bulk work keeps the same cooperative quantum of one.
Each arm validates every output and the Actor rank/worker contract. A malformed
order must raise through both the native-cycle and public service paths; later
successful reuse and zero roots at shutdown are required.

## Metrics and limits

One message is one service submission, not one order or invoice line. Both
messages/s and orders/s are emitted. Each full measured sample has 2,048
messages; each profile/arm has six samples (12,288 messages). Throughput divides
completed messages by loop wall time, including input cloning, native work,
client consumption, oracle validation and progress reporting. Phase orchestration,
boundary diagnostics, startup and teardown are excluded.

Per-request clocks measure:

1. Start before input cloning to constructor entry: preparation, admission and
   owner entry together, not an OS-wait attribution.
2. Native payload construction.
3. Constructed payload to projection start: Scheme queue/execution/GC and the
   arm's completion protocol together, not pure handler CPU time.
4. Native result projection.
5. Projection end to actual client consumption.

End-to-end p50/p95/p99/p99.9/max are independent from these phase percentiles;
phase percentiles cannot be added or treated as the same tail request. Oracle
time is separately reported and remains inside throughput timing. Diagnostics
are read only at sample boundaries, outside timing: collection count, heap
capacity, GC user/system/wall time and owner thread CPU. Missing optional
diagnostics remain null. GC wall time must not be called CPU execution.

The existing five-second request and no-progress limits remain in force. The
Scheme reference uses the existing wait mechanism and a single decreasing
five-second deadline; it does not add host polling. A watchdog failure is a
failure, not a benchmark result. Final forced collection, zero external roots,
clean owner join and image-file removal are mandatory. Successful component
measurement does not close the historical unassigned watchdog investigation or
qualify a production scheduling change.

The existing ownership/runtime models remain authoritative for the unchanged
production protocol. No new scheduler, lease or budget model is introduced by
this measurement-only entry point. Architectural changes require model updates
and targeted correctness/performance acceptance before publication.

## First completed component cohort

This historical cohort used the original channel observer. Its observer cost is
included in these numbers. It remains a dated record, not the corrected
observer's current baseline; observer changes are not library speedups.

Host: Apple M2 Pro, 12 physical/logical cores, 32 GiB RAM, macOS 26.6.1
(25G76), Rust 1.95.0. The SDK used the existing qualified Gerbil 0.19/Gambit
native build with O2. One VM was retained across all arms. This is a new
component profile, not a replay or replacement of the historical 10.14/13.04 ms
12-owner open-arrival baseline.

All 54 measured samples completed: 110,592 service messages, with 12,288 messages
per profile/arm. All outputs passed the oracle. Final collection found zero
external roots; the owner joined and its image file was removed. The executable
and loaded image SHA-256 were unchanged before/after measurement. Whole-file
source checksums and the raw capture remain outside Git.

Throughput below is total completed messages divided by summed sample wall time.
The p99 column is the **median of six individual sample p99 values**, not a
percentile computed by pooling requests. All measured samples, including the
slow ones, contribute to throughput.

| Profile | Arm | Messages/s | Sample p99 median (ms) | Sample p99 range (ms) |
| --- | --- | ---: | ---: | ---: |
| short CPU | Owner native service cycle | 21,754.63 | 0.407729 | 0.398625–0.431000 |
| short CPU | Caller-driven service | 9,797.26 | 0.663417 | 0.417792–2.317833 |
| short CPU | Tokio worker service | 20,390.45 | 0.426542 | 0.393709–0.655958 |
| bulk CPU | Owner native service cycle | 10,563.29 | 0.467125 | 0.412834–0.823000 |
| bulk CPU | Caller-driven service | 8,040.99 | 0.579896 | 0.507750–2.011334 |
| bulk CPU | Tokio worker service | 7,656.59 | 0.570605 | 0.432667–1.293833 |
| bulk mixed | Owner native service cycle | 561.24 | 8.081729 | 3.661750–13.178750 |
| bulk mixed | Caller-driven service | 597.47 | 6.611167 | 2.908167–9.310709 |
| bulk mixed | Tokio worker service | 592.47 | 6.540729 | 3.191833–10.020500 |

These are observations of existing paths, not gains from a production
optimization. The caller-driven short path contains slow windows: individual
sample rates range from 4,574.55 to 24,139.70 messages/s. Every such sample has
37 collections, while GC wall time ranges from approximately 14.5 to 25.7 ms
for the entire sample. Entry and recovery tails also grow in slow samples.
This does not identify a particular OS wait or prove a per-request GC cause;
it prevents blaming the aggregate slowdown on collection count alone. No
outlier was excluded and no repeat run was used to replace this cohort.

For the Tokio-worker short path, the medians of sample phase p50 values are
1.125 microseconds for native construction, 11.3545 for start-to-constructor,
9.146 for constructed-to-projection, 0.333 for projection, and 8.021 for
projected-to-consumed. These separate phase statistics are not an additive
latency decomposition. They support studying owner/client handoff before
prioritizing further micro-optimization of the approximately one-microsecond
constructor. Bulk CPU retains a measurable service-protocol cost; it is not
evidence that Scheme computation should migrate to Rust.

The mixed profile intentionally suspends Scheme green threads and has broad
tail variation in every arm. Its roughly 1.4 ms median request latency and
one-outstanding-request limit cannot establish asynchronous saturation capacity,
cross-owner fairness or a production short-request SLO. No single-VM maximum
performance claim or default-policy change follows from this run.

Strict Clippy for the new bench and the existing native service lifecycle test
passed. That test covers same-Actor multi-shot continuation reentry, GC,
initialization/handler failures and close/drain. The first sandboxed smoke
attempt failed while opening the existing `/dev/fd` wake pipe and returned a
clean startup-failure receipt; it never entered measurement. The subsequent
unsandboxed smoke and full cohort passed, with the same five-second native
limits. This environmental failure is retained separately from the measured
performance data. Remote CI and a pure in-Scheme upper-bound measurement are
not claimed by this component cohort.

## Verified progress observer and rejected engine candidates

The original component bench sent a channel message for every verified result,
adding another cross-thread notification path while measuring owner/client
handoff. This qualification now records a monotonic verified-result epoch
atomically. The observer sleeps until the current deadline and rechecks the
epoch; progress only extends that deadline, so it needs no per-request wake.
Only final observer shutdown signals its condition variable.

Recording follows real result/oracle validation. Queued or late events cannot
revive an expired deadline, and older timestamps cannot regress progress. The
five-second request and no-progress limits remain unchanged. Deterministic Rust
tests cover extension, expiry, stale records and joined shutdown; they run
through the existing service test target. A no-progress failure measures
verified client progress and alone does not prove a Scheme deadlock.

Output identifies the observer as
`atomic_verified_epoch_no_per_request_notification`. Workload, input types,
validation, request counts and latency boundaries are unchanged; progress
recording stays inside throughput timing. This is a measurement correction, not
a library throughput optimization. The separate frozen `6f32a10` capacity and
open-arrival references are preserved.

Two engine candidates were implemented and exercised through real native APIs,
then withdrawn:

- A bound-owner client cached capabilities/capacity generation and reduced
  global admission-lock acquisitions from four to one. It retained the final
  enqueue/close fence. Native continuation/GC, errors, stale generation, partial
  reservation cancellation, timeout and close tests passed. Its performance did
  not justify adoption.
- An idle bulk plan submitted one input before an otherwise empty advance,
  requiring no tracked native job or owned completion token. Short inputs used
  their original early path. Existing joint-model state supplied small/wide
  eligibility and token-custody witnesses; native tests covered continuation
  reentry across budget changes, blocked bulk/short progress, tokens, errors,
  cancellation and drain. Safety did not imply a performance improvement.

The cache candidate used the channel observer. The final idle candidate below
used the corrected observer. These are separate matched cohorts, not a
cross-cohort comparison. Each row has 12,288 messages per arm, in six samples.
Throughput uses all completed messages/summed wall time; p99/p99.9 are medians of
the individual sample percentiles, not pooled percentiles.

| Candidate / profile | Control messages/s | Candidate messages/s | Control p99 (ms) | Candidate p99 (ms) | Control p99.9 (ms) | Candidate p99.9 (ms) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Cached client / short CPU | 23,494.04 | 20,813.54 | 0.406584 | 0.405396 | 0.459834 | 0.476271 |
| Cached client / bulk CPU | 9,625.17 | 9,001.23 | 0.472125 | 0.458521 | 0.877105 | 0.623688 |
| Cached client / bulk mixed | 600.62 | 650.52 | 3.874542 | 4.141604 | 11.313417 | 9.465104 |
| Idle bulk / short CPU | 26,169.12 | 26,954.78 | 0.394979 | 0.387625 | 0.454355 | 0.443187 |
| Idle bulk / bulk CPU | 11,005.41 | 10,913.96 | 0.437480 | 0.445459 | 0.668000 | 0.913855 |
| Idle bulk / bulk mixed | 687.36 | 664.38 | 2.902542 | 3.620792 | 6.060812 | 8.413792 |

Short input code paths were identical in the final idle-plan pair; differences
there are a negative control, not short-algorithm gains. The idle plan moved
bulk entry p50 from approximately 14.54 to 11.63 microseconds but moved
constructed-to-projection p50 from 29.38 to 31.67 microseconds. Those marginal
statistics are not an additive decomposition. The absence of net bulk gain and
mixed tail regressions reject advancing input alone as a completed solution.
Both candidates, their APIs and model changes were removed from the active
source. Runtime library and Scheme sources remain at the accepted baseline.

Earlier channel-observer idle-plan attempts are also retained externally: the
initial placement observed bulk CPU +5.64% but short throughput -5.06%; bypassing
short eligibility observed further regressions. No outlier or failed cohort was
discarded, and no later pass retroactively qualifies those attempts. Source
archives, raw captures and whole-file hashes stay outside Git.

One combined-process native test attempt hit the existing five-second watchdog
during initialization, before the new policy was enabled, with cleanup
unverified. Later separate-process passes did not establish its root cause.
That failure remains open; neither model passes nor this observer correction
close it. No engine optimization or production-default change was published.

The component bench is registered in the existing native CI smoke/lint job and
available through `just bridge-single-vm-service-benchmark`. Smoke validates
execution and cleanup, not production performance or exact-head remote CI.

## Corrected-observer component reference

With both engine candidates withdrawn, the runtime, Scheme and model sources
were checked identical to `594c4ce`. The corrected-observer capture retained the
same native image SHA-256 throughout the run. All 54 measured samples completed
(110,592 messages); every profile/arm has six samples and 12,288 messages.
No measured request was rejected or failed validation. Final roots were zero, the owner
joined and its image file was removed. Raw executable/image/source receipts and
captures remain external.

This table is the restored engine with the corrected observer, not an engine
speedup. p99/p99.9 columns are medians of six sample percentiles, and the p99
range explicitly preserves slow samples. Throughput includes every sample.
These component statistics are not pooled production SLO percentiles.

| Profile | Arm | Messages/s | Orders/s | Sample p99 median (ms) | Sample p99 range (ms) | Sample p99.9 median (ms) |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| short CPU | Native service reference | 14,804.31 | 14,804.31 | 0.373729 | 0.343333–1.891875 | 0.426604 |
| short CPU | Caller-driven service | 27,446.62 | 27,446.62 | 0.376188 | 0.364542–0.409459 | 0.428187 |
| short CPU | Tokio worker service | 30,311.11 | 30,311.11 | 0.366979 | 0.352417–0.397250 | 0.406479 |
| bulk CPU | Native service reference | 12,450.01 | 99,600.10 | 0.405938 | 0.373666–0.425083 | 0.433271 |
| bulk CPU | Caller-driven service | 8,614.84 | 68,918.73 | 0.410230 | 0.395584–1.183875 | 0.466708 |
| bulk CPU | Tokio worker service | 12,227.39 | 97,819.14 | 0.411063 | 0.388375–0.428583 | 0.455667 |
| bulk mixed | Native service reference | 708.61 | 5,668.91 | 2.392708 | 1.803084–3.744875 | 4.620688 |
| bulk mixed | Caller-driven service | 704.68 | 5,637.41 | 2.701375 | 2.115750–3.590625 | 4.710292 |
| bulk mixed | Tokio worker service | 691.74 | 5,533.90 | 2.818105 | 2.148083–4.104084 | 5.563584 |

The native reference still contains a Rust queue and a blocking host ticket; it
is not a pure in-Scheme ceiling. Its slow short-request window is retained,
including its lower aggregate throughput. These results cannot establish an
efficiency ratio or claim that one VM is already maximally utilized.

Every mixed bulk message contains eight orders and at least one requested 1 ms
Scheme suspension. With one outstanding message, these rates describe that
latency-limited component contract, not asynchronous saturation capacity. They
must not be compared directly with CPU-only throughput or extrapolated to twelve
VMs. The historical capacity and open-arrival cohorts remain separate.

Final local gates: seven service/observer tests passed, strict selected-target
Clippy and changed-bench Rust formatting passed, and the existing Justfile/CI entrypoints were
wired. Remote CI and the unresolved initialization watchdog are not qualified by
this capture. No rejected runtime candidate was committed or sent to PR #18.

## Fixed million-message baseline

The explicit `--million-short-cpu` mode completes exactly 1,000,000 verified
short messages through the public Tokio-worker API on one VM. It preserves
the original 2,048-message corpus, fresh native construction, five-second
request/progress guards and whole-request timing. Its single measured cohort
reports whole-million quantiles, not a median of small-sample p99 values.
The old default diagnostic mode and production engine are unchanged.

Run `just bridge-single-vm-million-baseline`. The model and architecture
contract is in `specs/scheduling/README.md` at the repository root.
A closed-loop single-VM zero-rejection result does not replace the twelve-VM
fixed-arrival acceptance gate.

## Same-workload owner scaling

`--fixed-owner-scaling=1` and `--fixed-owner-scaling=12` each measure exactly
1,000,000 short-CPU messages in total, with one live request per fixed owner
and a common start barrier. They retain the original oracle, five-second guards
and cleanup controls. Quantiles pool actual requests, while throughput ends at
the last client completion. This closed-loop comparison is separate from the
12-VM mixed open-arrival acceptance contract. See
[the scaling contract](../../../../specs/scheduling/fixed_owner_scaling.md)
for measured results, ownership boundaries and frozen artifact hashes.
