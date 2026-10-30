# Historical two-second burst at 500,000 planned arrivals per second

This retained result is a short burst. The current duration-driven protocol is
documented in [bounded short concurrency](short_concurrency.md); it does not
stop at one million messages.

The clarified request is an **offered rate of 500,000 messages/s**, not a
50,616/s completed-throughput requirement. The earlier completion-threshold
interpretation is withdrawn. This run measures the current local owner-publication
research candidate, not a newly admitted engine revision or a baseline speedup.
The published research branch remains at `974a941`; these changes are local.

## Fixed contract

`just bridge-single-vm-open-500k-burst` uses the existing Rust `scheme_service` capacity
bench with an explicit `--capacity-short-only` selection. Default mixed-class
behavior is unchanged. The workload uses the real native-type Scheme service:
one order with four records per message, full independent Rust result checking,
no JSON transport and no Scheme capability reduction.

| Setting | Value |
| --- | ---: |
| Scheme VMs / fixed native owners | 1 / 1 |
| Tokio workers / client tasks | 1 / 50 |
| Planned arrival rate | 500,000 messages/s |
| Planned measurement messages | 1,000,000 |
| Planned arrival interval / generation period | 2 microseconds / 2 seconds |
| Warmup, excluded | 1 second, closed-loop |
| Bounded ingress queue | 128 messages |
| Native request slots / work credits | 4 / 32 |
| Router short / bulk slots per owner | 1 / 3 |
| Deadline from original planned arrival | 5 seconds |
| Native-progress watchdog / external process limit | 5 / 300 seconds |

Short-only selection does not borrow the three unused bulk class slots or change
any library defaults. This measures the configured routed API, not an intrinsic
Gambit single-core compute ceiling. Admission pressure, client queueing and
execution are distinct resources.

The existing arrival clock keeps original planned timestamps. Already-due
arrivals catch up in at most queue-sized turns, without retiming or silently
reducing the offered rate. A full ingress queue rejects before constructing
Scheme input. Every planned arrival has a recorded terminal outcome. Generator
lag includes all arrivals, including rejected messages. Successful latency
quantiles include preparation, admission, native service and client observation;
rejected and failed requests remain separately counted.

This is a finite two-second offered burst followed by drain, not evidence of
long-run sustained stability. Histogram/result storage in the benchmark is
separate from bounded runtime queues and native roots.

## Observed result

| Metric | Observed |
| --- | ---: |
| Planned / ingress-attempted messages | 1,000,000 / 1,000,000 |
| Actual ingress-attempt rate | 499,999.792 messages/s |
| Generation time | 2.000000833 s |
| Library-started / successful messages | 50,064 / 50,064 |
| Ingress rejections before library invocation | 949,936 |
| Ingress rejection rate | 94.9936% |
| Library-call failures, including timeouts | 0 |
| Elapsed measurement including drain | 2.006757583 s |
| Successful completion throughput | **24,947.707 messages/s** |
| Successful planned-to-consumed p50 | 6.958791 ms |
| Successful planned-to-consumed p95 | 8.757916 ms |
| Successful planned-to-consumed p99 | **11.505958 ms** |
| Successful planned-to-consumed p99.9 | 12.207666 ms |
| Generator lag p99 / max, all arrivals | 24.375 / 223.333 microseconds |
| Tracked roots after / owners joined / runtime images removed | 0 / 1 / yes |

**50,064 is the total success count, not a per-second rate.** Successful
throughput is 50,064 divided by 2.006757583 seconds. The generator achieved the
requested offered rate; these observations do not support a generator-rate
shortfall explanation. The current configuration **cannot admit and complete
500,000 messages/s**. Clean exit and cleanup establish outcome collection and
drain, not offered-load performance acceptance.

The existing Rust oracle verified every successful message. All one million
planned outcomes are accounted for: success + ingress rejection + failure.
A 1,000-arrival functional profile check preceded the measurement; its timing
is not reported as capacity. Strict benchmark Clippy passed.

| Successful-request marginal p99 interval | Observed |
| --- | ---: |
| Ingress queue | 8.313334 ms |
| Preparation / routing admission / owner wait | 3.446750 ms |
| Native service interval | 0.389208 ms |
| Result ready to client observation | 0.312792 ms |

These marginal quantiles may describe different requests and cannot be added.
The native interval is not pure Scheme CPU time. The retained one-slot short
class also serializes admission across 50 clients; this result must not be
renamed to a Gambit compute ceiling or used to justify weakening Scheme.
Increasing a queue would not by itself establish greater successful throughput.

## Reproduction and identity

The recipe invokes Cargo directly through the existing project build environment;
no Python or Bash driver is added. The `arrival-clock` feature is required by the
existing high-resolution arrival plan. Native compilation remains at optimization
level 2. The executable default image was checked against Cargo output and then
compared byte-for-byte with the frozen image before measurement.

| Artifact | SHA-256 |
| --- | --- |
| Measured executable | `a6bccfb2ff2ef18978ee8170c1adbb0f5bcc9b618688b96f26cec2a15838c028` |
| Loaded native image | `271d52efd66b04999fbbbcca0c9d49cd5cc2d55e0d5cbd23b763dbaa40cb24ec` |

Host: Apple M2 Pro, 12 physical/logical cores, 32 GiB RAM, macOS 26.6.1 (25G76).
Raw logs, environment, source hashes, executables and images remain in ignored
local capture storage. No other task was paused and no automation was created.
This one-run observation is not an optimization A/B or a production admission.
No engine/benchmark candidate is committed or pushed on the strength of it.
