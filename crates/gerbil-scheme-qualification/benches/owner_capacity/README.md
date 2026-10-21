# Shared capacity and explicit Scheme batches

Run from the repository root using the existing SDK and Cargo build:

```text
just bridge-owner-capacity-benchmark --test
just bridge-owner-capacity-benchmark
```

This separate focused bench retains four identical native Scheme VM states and
two Tokio workers. It has two explicit AB/BA pairs: shared owner capacity 128
versus 4 at batch size 1, then batch size 1 versus 8 at capacity 4. CPU and mixed
cooperative Scheme waits use the same 48-line invoice corpus. This adds no thread
count/VM matrix and changes no production defaults. The first control also enables
shared capacity; it is not relabeled as a historical unbounded binary baseline.

Each full sample offers 8,192 independent bulk orders and 64 four-line short
probes scheduled every 500 microseconds. Smoke uses 256 and 8. Probes are one
closed-loop producer: a delayed response postpones its next submission, retaining
the original schedule for latency. Wake lateness is reported separately. This is
not an independent open-loop arrivals test. The bulk producer uses a 128-request
invocation credit bound and immutable owner assignment. It can have only one
pending capacity reservation per owner; existing accepted work retains its shared
credit through real native completion. Two Rust tasks represent the producers,
never one task per native request; only the fixed native owners execute Scheme.

Bulk inputs are generated outside timing. Probe fixture construction and cloning
are included in the probe entry/admission span. Bulk payload cloning, admission,
native type construction, Scheme execution, projection and independent Rust
validation remain inside elapsed timing. Explicit Scheme Actor batches preserve their documented
whole-batch exception/drain and cooperative quantum semantics. The numeric batch
is an existing specialized native API; the generic backpressured native operation
retains arbitrary Scheme types and procedures. No arbitrary calls are implicitly
fused. Each owner verifies a real eight-item batch error and subsequent recovery.

Warm each pair arm on the same 256-order native workload before measurement. Five
forward/reverse blocks provide ten samples per arm, one block in smoke. The fixed
five-second startup/progress/observation gates remain. Reconfigure only between
samples once every shared credit has returned; pool state is never reinitialized.

`capacity_sample` retains complete per-owner bulk native offsets and individual
probe native/delivery/admission/wake spans. `capacity_result` gives complete and
bulk-only rates, probe p99, bulk owner progress at first owner completion, probe
completions during bulk execution and bulk completions by the final probe. Faster
bulk completion can reduce overlap; inspect that count before claiming a
steady-state p99 improvement. Bulk batch members share completion timestamps.
Actor peer progress applies to batch 8 and is null for batch 1. Finite progress,
semaphore FIFO and fixed probe totals do not prove starvation freedom or arbitrary
cross-caller fairness. Probe admission spans include host validation as well as
capacity wait; they are not pure mutex or GC estimates.

Retain every failure and negative pair. Store raw captures outside Git in a
caller-selected directory using relative evidence names. Follow the existing
[portable capture instructions](../owner_dispatch/README.md#portable-evidence-capture)
for Git/source, executable/native-image SHA-256, command, exit and host-load
identities. JSON Lines are result records, not the Scheme payload protocol.


The shared bound counts native requests, not items or CPU time. At capacity 4,
batch 8 can retain up to 32 bulk items per owner versus four at batch 1. The second
pair therefore evaluates an explicit batch policy under the same request-slot
limit, not equal queued-work budgets or a pure Scheme scheduler speedup. Whole
native turns remain non-preemptive to the Rust owner queue; internal Scheme Actor
yields do not let another Rust queue entry enter mid-call. A throughput gain with
a short-probe tail regression must remain unpromoted. Work-weighted/service-turn
budgets require a separate protocol refinement; arbitrary Scheme work has no
automatically inferable execution cost.
