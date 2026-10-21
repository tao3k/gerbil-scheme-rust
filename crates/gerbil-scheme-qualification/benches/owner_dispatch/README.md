# Rust dispatchers on fixed Scheme owners

```text
just bridge-owner-dispatch-benchmark --test
just bridge-owner-dispatch-benchmark
```

Use the repository's existing Gerbil SDK environment. Cargo builds the same native
Scheme order image through the established build support. No Python driver, Bash
wrapper, JSON payload transport or Scheme-to-Rust workload translation is involved.
JSON Lines are result output only; retain raw captures outside Git.

## Matrix and controls

- Native owners: 1 and 4. Each owner runs one fixed-thread Scheme VM and four Scheme
  Actor workers. The same VM state identities are asserted throughout the entire
  dispatcher matrix. Scheme owns green-thread waits, cooperative yields and GC.
- Direct has zero additional dispatcher workers; the caller submits all work before
  waiting. Tokio and Rayon use 1, 2 and 4 dispatcher workers independently of owner
  count. All modes also have one caller and one benchmark watchdog. Worker startup
  and exit callback identities are checked and emitted. These are observed Rust
  dispatcher workers, not a process-wide OS-thread count or an OS join receipt for
  Rayon. Native owner joins are separately required at final teardown.
- Tokio spawns asynchronous dispatch tasks and awaits native oneshot tickets. The
  new `submit_numeric_batch_async_on` public API reuses the synchronous batch's
  bounds validation, owner-local Scheme execution, projection and root retirement.
  No Tokio worker blocks waiting for native Scheme. Rayon uses indexed parallel
  submission and collects native tickets before waiting on the caller; the
  intentionally different `rayon-blocking` variant is outside this matrix.
- Batches: 1 and 32, cooperative quantum one. Each window releases 128 identical
  offered orders across fixed owner routes. CPU and mixed inputs use the shared
  `benches/support/orders.rs` corpus and independent invoice oracle. Mixed inputs
  request a 1 ms Scheme wait every seventh order; all orders have 48 invoice lines.
- A full sample has 1,024 orders; smoke has 128. Five blocks rotate case order and
  run forward/reverse sweeps, producing ten samples per configuration. Smoke uses
  one block and is not performance acceptance. Each case constructs only its own
  executor, warms it on 128 orders, measures, then checks all worker exit hooks
  before the next case. Other dispatchers are not kept resident. Setup/retirement
  are outside sample timing and reported separately; native VM state is retained.
- Complete typed outputs are independently validated. Every mode/thread count also
  propagates a real Scheme exception and recovers on every owner. Empty async
  batches are rejected before admission. At close, Actor workers stop, full GC
  observes zero external roots, native owners join and private image files disappear.

## Measurement boundaries

Throughput is complete orders/s from median sample duration. Corpus generation is
outside timing; Rust input cloning, task submission, native conversion, execution,
projection, retirement and Rust validation are inside. No phase diagnostics are
enabled. Native response p99 starts at the common window release and ends at the
native ticket completion timestamp. Window-delivery p99 additionally includes
executor/caller result collection. Per-owner native p99 and the minimum completed
orders among owners when the first owner finishes its full share are separate
fairness indicators. Each owner receives 128 / owners orders per window. The Actor
peer minimum measures within-batch progress before the first Actor worker finishes;
it is null for one-item batches. With batch 32, one owner can finish an entire
batch while another has not returned any batch yet: zero completed-order progress
is not evidence of zero internal Scheme progress or starvation. Batch members share completion clocks, so these
are burst statistics rather than independent arrivals, open-loop SLOs or proof of
starvation freedom. Single-owner progress is necessarily 128 and is not scaling.

All modes use the same five-second window deadline and native progress watchdog.
Accepted tasks are drained; timeout is not cancellation. Failures abort qualification
without automatic retries or deadline changes. The prior competing-probe QueueFull
negative remains separate: this matrix does not add competing probes or fix global
admission. Python/GIL, resident-input data placement, arbitrary stateful session
ordering, long-term memory convergence and generic extension unloading require their
own controls. Parallel stateless submissions need not preserve per-owner enqueue
order; stateful session ordering is not admitted by these measurements.

No production defaults change. Local results do not certify a quiet host, linear
VM scaling, a universal scheduler winner or unchanged performance on another SDK.
CI runs smoke and strict lint, not a performance promotion gate.

CI also runs the existing complete native capability/reuse smoke. Its Scheme
fault assertion requires the explicit typed diagnostic; unexpected assertions
retain their panic output. This is separate from the performance matrix and
does not establish long-term memory or arbitrary extension unloading.
