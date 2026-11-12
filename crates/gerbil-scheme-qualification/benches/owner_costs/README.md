# Native owner cost benchmark

Run from the repository root, using the existing Gerbil SDK environment:

```text
just bridge-owner-cost-benchmark --test
just bridge-owner-cost-benchmark
```

The first command is a correctness smoke run. The second runs five ABBA blocks,
with ten measured samples per arm after warmup. Cargo builds the existing native
Scheme image through the repository build support. No Python driver, JSON payload
serialization, Bash script, translated Rust workload or alternate build is used.
JSON Lines are measurement output only; redirect them outside the repository.

## Matched controls

Each pair uses the same live VM owners, Scheme image, native `s64vector` types,
order records, Actor service and complete Rust result oracle. The shared corpus
and oracle live in `benches/support/orders.rs`, also used by the existing
`instance_resources` qualification. Scheme computes 48 order lines into SKU and
invoice totals using four Scheme Actor workers per VM. Rust only submits work to
fixed owner threads. Mixed inputs request a 1 ms Scheme green-thread wait on every
seventh order. These waits are intentional application work, not a watchdog pulse.

The matrix covers 1/4 owners and batches of 1/32 with 128 orders released per
window. A full sample contains 1,024 orders. Inputs are generated before timing;
Rust input cloning, native conversion, dispatch, complete native projection,
root retirement and Rust validation remain in the sample duration. Batch quantum
is fixed at one. Production queue, owner and scheduler defaults do not change.

A is the normal public batch API. B is its opt-in profiled counterpart. This A/B
measures diagnostic overhead, not an optimization or resident-input speedup.
There are no competing short probes in this benchmark. The existing mixed-probe
QueueFull negative remains separate and is not repaired or superseded here.

## Output boundaries

- Throughput is complete orders/s from median sample duration, not Actor messages/s.
- Response p99 is measured from each 128-order window's release to native ticket
  completion. Batch members share a completion clock; they are not independently
  arriving requests. This closed-loop burst test is not an open-loop service SLO.
- Profiled phase percentiles are per native batch: admission plus owner queue,
  item construction, envelope construction, Scheme execution including GC,
  native projection, and root retirement/completion. Clock reads, diagnostics and
  OS preemption affect these spans. Phase quantiles must not be added together.
- Native GC real time and collection deltas are reported separately for item
  construction, envelope construction, execution and projection. They are SDK
  counters, not sampled wall time that can be subtracted to derive pure compute.
- `peer_min` measures progress by all four Scheme Actor workers before the first
  worker finishes its share in a batch. It is null for single-item batches and is
  not proof of starvation freedom or fairness between VM owners.
- Startup and shutdown are separate receipts. Teardown asserts clean joins,
  zero external roots and removed private image files. It does not assert zero
  RSS, complete dynamic-library unloading or long-term allocator convergence.
- Python/GIL time and direct/Tokio/Rayon scheduler comparisons require their
  separate client/dispatcher controls. This target establishes the direct native
  reference and must not be used to declare a scheduler winner.

The native progress watchdog and ticket deadlines remain five seconds. CI runs
only the smoke mode and strict Rust lint; smoke timings do not qualify performance.
