# Gerbil-native Actor arrival reference

## What this reference measures

The existing Rust arrival driver cannot alone establish Gerbil's native capacity.
This qualification bench makes one foreign call per arm. Inside that call Gerbil
generates scheduled arrivals, constructs the same fresh four-record native order
payload, executes the unchanged `orders-service-handler`, independently verifies
the21-field result and consumes it. There is no Rust task, result projection,
input conversion or foreign entry per message. This is a native comparison layer,
not production Rust API acceptance or a claim about millions of live Actors.

The library-service reference drives the existing endpoint with its four resident
workers and Snapshot ready/take protocol. The raw-mailbox control uses four
resident `spawn` workers, `thread-send` and `thread-receive`, retaining the same
business handler, input, oracle and arrival/admission boundaries. It isolates the
library's service protocol from raw mailbox execution, rather than comparing the
real workload with empty-message throughput. Gambit's timeout/default mailbox
contract is documented in the official
[thread manual](https://www.gambitscheme.org/4.9.4/gambit.pdf).

Both controls use one VM processor, active3 and total outstanding128, with no
unbounded mailbox. There is at most one message per admission, not a merged
business batch. The native generator stores root-free request IDs until an
existing active opportunity is available. Raw workers are stopped and joined on
normal and exceptional exit. The library's ordinary endpoint workers remain
present; only the four selected workers execute the measured application work.

## Timing and accounting

At500,000 and1,000,000 planned arrivals/s each measured arm lasts ten seconds,
respectively accounting for5,000,000 and10,000,000 arrivals. Excluded warmup is
one second at1,000/s. Overdue arrivals retain their original planned timestamps.
An arrival rejected at the fixed window is counted; it is never reported as a
success. The generator checks all original planned arrivals, yielding between
bounded generator opportunities. Catch-up cannot silently reduce the offered
count. Report actual offered duration and maximum generator lag as well.

Latency spans planned arrival through native result verification/consumption.
Only successful messages enter the latency histogram. Its1us bins report upper
bounds; p99 is not nanosecond-exact. Completed throughput includes final drain,
but excludes histogram summarization and warmup. Result errors abort the arm;
zero failures is reported only after all accepted results pass the oracle.

The five-second watchdog reads an aligned atomic native clock word written only
after a result passes validation. Supply, timer ticks and green-thread yields do
not refresh it. The progress pointer is qualification-only and borrowed for the
single synchronous call: an owning Rust `Arc<AtomicU64>` remains in that owner
closure until native return/unwind. It is not retained globally or exposed as a
production type interface. Startup and shutdown retain their five-second gates.
The total call wait accommodates a ten-second experiment; that is separate from
the unchanged five-second no-completion limit.

The native oracle and input generator are Scheme implementations, whereas the
Rust path uses its independent Rust generator/oracle and native-to-Rust copies.
In particular, the existing Rust generator creates48 records then truncates to4;
the native control creates4 records directly. Therefore a cross-layer throughput
ratio is not an exclusive measurement of FFI or Tokio overhead. It is evidence
for the next decomposition, not a causal allocation of all lost CPU time.

## Model and admission

`native_actor_reference.qnt` imports `continuous_short`, which already refines
owner lifecycle and single-processor Actor execution. It distinguishes supplied
work, native readiness, projection, verification, retirement and consumption.
A negative supply-heartbeat mutation violates the verified-progress invariant.
Ten inherited/focused scenarios and1,000 bounded safety traces passed. These
checks do not prove timing or GC behavior.

This reference introduces qualification code and a Rust bench only. The drafted
resident protocol was archived locally and removed from the active tree before
this native calibration. The earlier typed single-transaction construction and
all production defaults remain unchanged. A native result is not a production
Rust-path performance acceptance or permission to push PR18.

## Frozen paired observations, 2026-10-27
| Planned messages/s | Native path | Completed | Completed/s | Rejected | Rejection | Successful p99 ms | p99.9 ms |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 500000 | Existing service | 2426045 | 242592.76 | 2573955 | 51.47910% | 0.931 | 1.043 |
| 500000 | Raw mailbox | 4767856 | 476775.66 | 232144 | 4.64288% | 0.418 | 2.056 |
| 1000000 | Existing service | 2416660 | 241654.07 | 7583340 | 75.83340% | 0.925 | 1.021 |
| 1000000 | Raw mailbox | 5055939 | 505582.60 | 4944061 | 49.44061% | 0.644 | 1.564 |

All four measured arms account for their five/ten million planned arrivals.
Admitted result failures are zero and both pair shutdowns report clean. Each
rate pair shares one live VM and image, service first then mailbox, with separate
warmups. This is one observation per arm, not repeatability acceptance.
The frozen binary SHA-256 is `77cdd0b2e3dbb21de3cfa7fc94873fb5e8bbd2350004c4777d9461ae058841ad`.
The image SHA-256 is `0f9a0ab577e973e4218c8ec0da40ecf4a22399d485f004f01c71f41dd8d13020`.
Benchmark sources, source digests, contract, original outcomes and earlier
service-only observations remain together in ignored local artifacts.
The mailbox reference reaches about505,583 completed/s under1M/s offered
load; this does not establish500k/s with zero rejection, nor1M/s capacity.
Its500k/s arm still rejects4.64288%. The existing service reaches about242k/s
in both offered-rate arms. This identifies service-protocol headroom without
asserting that all cross-layer loss belongs to Rust or that Gerbil has a fixed
upper limit. Retain the high-rejection records as failed capacity admission.

The outstanding-credit backpressure gap is now addressed separately in the actual
Rust API; see [continuous_backpressure.md](continuous_backpressure.md). It does
not silently change the immediate-admission overload diagnostics above. The final
native regression suite passed18 cases and the related test/bench strict Clippy
passed. Moving raw-mailbox throughput into the supported typed Rust service path
remains performance work; neither correctness nor this component reference is a
production throughput acceptance.
