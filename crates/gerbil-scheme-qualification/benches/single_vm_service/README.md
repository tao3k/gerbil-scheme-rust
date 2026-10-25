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
