# Scheme-owned asynchronous service benchmark

Run from the repository root in the admitted native SDK/Cargo environment:

```text
just bridge-scheme-service-benchmark --test
just bridge-scheme-service-benchmark
```

This is one directed synchronous/service pair, not a scheduler matrix. Both arms
use the same four Scheme VMs and image, two Tokio workers, four request slots and
32 work units per owner, batch eight, quantum one, the same numeric constructor,
Scheme order handler and bounded projector. Both enable the service/wake
infrastructure. Defaults are unchanged.

The full run alternates AB/BA for ten samples per arm/load: 8,192 48-line orders
and 64 four-line probes on a nominal 500 microsecond schedule. Probes are serial;
a delayed probe delays subsequent actual arrivals. The mixed workload retains
real Scheme waits. It measures completion of accepted work, not offered QPS.

JSON events report configuration/startup, individual samples, aggregate results
and native cleanup. Throughput uses median native bulk completion duration; p99
pools the corresponding observations. Probe actual latency starts before
admission; scheduled latency retains arrival delay. Pending-before-projection
includes Scheme execution, waits and interleaving, and is not isolated CPU time.
Per-owner completion maxima, rather than submission order, define finish spread
and progress at first owner finish. Actor peer minima are observed progress,
not a proof of starvation freedom.

The five-second progress gate is unchanged. Correct results, stable VM identities,
returned credits, successful Scheme quiescence, four joins, zero external roots
and removed copied image files are required. Keep raw logs, frozen binaries and
source archives outside Git. See the repository status report
`docs/30-39-status/30.33-scheme-owned-service-and-native-wake.org` for matched
measurements and negative results. This is not an old-default Python comparison
or a claim that every CPU workload benefits.

## Explicit short layout and completion-ready comparison

```text
just bridge-scheme-service-benchmark --test --split-layout-ab
just bridge-scheme-service-benchmark --split-layout-ab
```

Both arms are asynchronous Scheme services on the same VMs. The control keeps
single-element batch probes and FIFO client completion observation. The candidate
uses a tagged native order/result layout for probes and the actual opt-in
`owner_flow_ready_backpressured` API for bulk completion observation. This is a
combined pipeline comparison, not independent attribution of each component.
Both arms impose the same explicit client bulk limit of three callbacks per
owner through projection, retaining the existing four native slots/work 32.
This leaves capacity for the one probe producer in this workload. It is not a
pool-wide reserved class quota against arbitrary independent producers.

The application's tag/version `[77,1]` is disjoint from every valid one-element
batch quantum header. It accepts at most four order lines and returns one native
numeric vector. Larger work is rejected at that entry; the full generic service,
batch Actor, evaluator and continuation capabilities remain available. Scheme
selects and executes the service worker; Rust does not execute the order logic.
Native short construction, pending and projection spans are retained separately.
The pool still shares Scheme workers and GC: this layout is not preemption or
complete latency isolation from CPU-bound non-yielding handlers.

The combined pipeline is not automatically admitted for performance. To isolate
short layout while retaining FIFO client observation and the same three-callback
bulk cap in both arms:

```text
just bridge-scheme-service-benchmark --test --short-layout-only
just bridge-scheme-service-benchmark --short-layout-only
```

This targeted follow-up addresses the mixed-throughput regression observed in
the combined candidate; it adds no VM/thread matrix. Keep both comparisons and
all negative captures under their own source identities.

## Classified route algorithms

```text
just bridge-scheme-service-benchmark --test --class-route-ab
just bridge-scheme-service-benchmark --class-route-ab
```

This pair keeps native short layout and FIFO completion identical. The fixed
control uses the same three-batch/one-short bound as the candidate's actual
`SchemeServiceRouter`; only the candidate selects destinations through the
explicit short queue-pressure and batch declared-work algorithms. Owner clocks
come from `NativeSession::owner_index`, with normalized progress and actual
assigned totals retained. Configuration is opt-in and shared by router clones.
Independent routers/unclassified native callers do not share its class quotas.

The current route heuristic regresses mixed throughput and short p99 despite
better normalized owner progress. It is experimental, not a default promotion.
See `docs/30-39-status/30.35-class-specific-native-service-routing.org` for the
actual routing API, native quota/cancellation/close checks and complete negatives.
Cross-instance routing and Scheme-owned short-queue scheduling are separate
problems; this comparison does not implement or qualify the latter.

## Scheme short scheduling only

```text
just bridge-scheme-service-benchmark --test --short-service-ab
just bridge-scheme-service-benchmark --short-service-ab
```

Both arms use the same four VMs, two Tokio workers, class router (one short / three
batch callbacks), native short layout, batch eight, work budget 32 and FIFO client
collection. Only the candidate opts into `SchemeServiceRequest::short_service`.
The service rejects this declaration unless it has one work unit, no Actor lane,
and an endpoint negotiating ABI v2. ABI v1 remains usable for general requests.

Scheme owns separate short work and completion queues, alternating eligible
short/general work on each persistent worker and returning at most one completion
of each class per owner turn while short work remains outstanding. This is a
cooperative work budget, not a wall-clock guarantee for arbitrary Scheme code.
An executing handler is not interrupted or moved to another thread.

`probe_post_admission_wait_ns` measures the interval from the admission future's
return to native construction entry, saturated at zero when execution starts
before the future returns. It excludes the measured admission interval, but still
combines host queueing, wake delivery and OS scheduling. Do not subtract aggregate
p99 values or label this interval isolated Scheme execution or wake latency.
See status report 30.36 for native acceptance and retained negative A/B evidence.
