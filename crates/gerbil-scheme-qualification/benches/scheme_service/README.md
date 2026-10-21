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
