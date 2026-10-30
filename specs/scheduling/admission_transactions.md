# Two admission transactions, one existing native lifetime

This candidate starts from research revision `974a941`. It preserves the frozen
short-CPU scaling driver and native image. It does not include the rejected
atomic-backend experiment, change defaults, or replace the mixed open-arrival
production gate.

## Protocol and ownership

The ordinary public pinned-owner service call currently visits the shared
registry four times: service validation, capacity capture, repeated service
validation, and final capacity attachment/publication. Those visits are not
four independent ownership transfers. The candidate combines them into two
transactions without caching a sender or adding a new lease:

1. Capture application declarations once in the existing `Checked` input.
   Under the registry lock, validate the live service target and clone its
   capacity Arc. This is a capacity observation, not permission to enter Scheme.
2. Release the lock and wait for work credits, then a request slot, using the
   existing finite timeout and semaphore ordering. Cancellation drops partial
   credits. This stage owns Rust input and capacity only, with no Scheme root.
3. Under the original final publication fence, revalidate the service target,
   require the reservation's capacity Arc to equal the current owner's Arc,
   and attempt the existing bounded channel send. Close removes the owner table
   under the same fence. Capacity reset replaces Arcs under this fence and
   closes old semaphores after unlocking. A stale waiter cannot publish into
   the new capacity, even if it acquired an old permit before old closure.
4. Unlock before destruction of a rejected request, credit return or foreign
   waker notification. A successful send retains the existing owner notification.
   First Scheme entry, roots and continuations remain pinned to that owner.
5. Keep native completion/projection, result publication, permit retirement and
   client consumption as their existing distinct events. A client timeout does
   not release accepted native work. No completion order or ready-first policy
   changes in this experiment.

The admission elapsed-time check includes the merged initial transaction. It
retains the supplied duration and returns an expired reservation before enqueue.
The change adds no request fields, heap allocation, per-message task, queue,
thread, periodic wake, or application callback under the registry lock.
`enqueue_checked` only receives a closure over copied short/lane metadata;
public user trait declarations remain outside that lock. Generic native/byte
requests retain their original enqueue behavior.

For this pinned-owner benchmark, the pool-level `CapacityEvents` OnceLock is
not initialized by a router or short pool. Its completion path therefore does
not perform the watch/Notify fanout used by those optional APIs. Editing that
fanout would not explain this benchmark. The backend readiness mutex, native
loader calls, Tokio handoff and owner service cycle remain possible costs;
this candidate makes no causal attribution of their measured shares.

## Existing model refinement

`admission_transactions.qnt` imports `owner_lifecycle`, retaining its slot/work,
close, accepted-work timeout, root and completion rules. Only captured capacity
identity is added. Its bounded epoch represents the existing Arc identity; no
new runtime generation counter is proposed. Old-generation permits are tracked
separately from replacement capacity. An unsafe stale-publication mutation is
rejected even when replacement credits happen to be available.

The directed suite has 82 passing cases: 76 inherited lifecycle cases and six
new traces. A 1,000-trace, 50-step exploration found no safety violation using
seed 1 and the TypeScript evaluator. The default Rust evaluator spent over a
minute processing the imported model without completing the first
reported case; that local tool invocation was stopped. It is not recorded as a
model pass. The alternate evaluator checked the same model and predicates.

Use:

```console
quint test specs/scheduling/admission_transactions.qnt --main admission_transactions --seed 1 --backend typescript
quint run specs/scheduling/admission_transactions.qnt --main admission_transactions --invariant safety --seed 1 --max-samples 1000 --max-steps 50 --backend typescript --verbosity 1
```

These are bounded abstract checks, not native safety or physical latency proofs.

## Native and source checks

The new public API test deterministically occupies an owner and verifies partial
credit return on cancellation and a 1 ms test admission timeout, rejection of
budget reset while reserved, and close rejection followed by accepted-work
drain. Its blocking operations retain the existing five-second guard. The zero
shutdown duration is an explicit nonblocking close observation, not an enlarged
limit. The existing full native GC/continuation/exception/close qualification
also passed. Strict benchmark and qualification-test Clippy passed.

The original three-arm smoke passed. The benchmark source and scaling module
remain byte-identical to the frozen control. The executable's embedded image
path and Cargo output were checked, then the candidate's image was compared
byte-for-byte with the original image before capture.

## Measurement contract

Both executables run one and twelve fixed owners, with exactly 1,000,000 total
messages per case, one outstanding request per owner and matching Tokio worker
counts. Warmup is 32 per owner. The native short request has one order and four
records. Oracle, timing phases, 5-second guards and 300-second outer limit are
unchanged. Run order is control-1, candidate-1, candidate-12, control-12. There
is no full matrix, profiler, background-task suspension or arrival-rate change.

Acceptance was recorded before measurement: zero errors/rejections and clean
retirement; single-owner throughput no lower and p99 no higher than its paired
control; twelve-owner throughput higher, p99 no higher and completion spread
no wider than its paired control. A single observation cannot establish a
statistically significant causal gain. Historical observations stay visible
and are not overwritten by the new paired control.

| Artifact | SHA-256 |
| --- | --- |
| Original control executable | `d85f9dd3fd1835cebef610d5e7e7f269310155194fa89fcf3030a165cb226396` |
| Candidate executable | `ab10ea518852fca627bde08e55e4ced7ba7ddd54261c9cffa86d0e04da8b4e06` |
| Shared native image | `271d52efd66b04999fbbbcca0c9d49cd5cc2d55e0d5cbd23b763dbaa40cb24ec` |
| Shared parent benchmark | `80d14ba407b2ea62386ee15f3fe651c4307c1b4c5bdf1ea732f68d09caf28360` |
| Shared scaling module | `f35362c81ee48faeeb86311855f1aa3d48c4abd6212bc6a39cf9d269ed9bd7fa` |

Raw logs, executables, source snapshots and environment captures remain outside
Git in ignored local storage. No absolute private path is required here.

## Outcome: not accepted, production source restored

| Case | Completed | Messages/s | p99, ms | p99.9, ms | Owner completion spread, ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Original, 1 owner | 1000000 | 11615.75 | 0.848459 | 6.859958 | 0.000000 |
| Candidate, 1 owner | 1000000 | 14525.43 | 0.556750 | 4.533167 | 0.000000 |
| Original, 12 owners | 1000000 | 26464.45 | 4.045250 | 13.093042 | 816.680750 |
| Candidate, 12 owners | 1000000 | 24742.03 | 4.939750 | 20.437208 | 407.282000 |

All four cases completed their million messages with zero rejection, zero
native errors, zero tracked roots after cleanup, the expected joins and removed
runtime images. Zero rejection is a closed-loop observation, not a fixed-arrival
production admission guarantee.

The one-owner candidate improved relative to this round's paired control, but
both observations are below the published unchanged-engine reference of
35,083.21 messages/s and 0.373500 ms p99. That history is retained and is not
replaced by the slower new control. No single-VM improvement is admitted.

The twelve-owner candidate fails the pre-recorded paired throughput and p99
gates: 24,742.03 versus 26,464.45 messages/s, and 4.939750 versus 4.045250 ms.
Its completion spread narrows, but that cannot compensate for the failed
throughput/tail gates. The historical twelve-owner control remains 70,829.74
messages/s and 0.810125 ms p99. This experiment does not explain the control's
cross-run variation or attribute it directly to machine competition.

| Sum of owner-thread CPU, s | Original | Candidate |
| --- | ---: | ---: |
| 1 owner | 46.119944 | 41.544341 |
| 12 owners | 96.175889 | 94.647667 |

Twelve-owner start-to-constructor p99 was 3,624.625 microseconds for the control
and 4,289.958 for the candidate; constructed-to-projection p99 was
708.875/826.083 microseconds. Marginal phase quantiles are different requests
and cannot be added, or used to attribute a registry lock's wall/CPU cost.

The source-level reduction from four registry acquisitions to two is verified;
a performance benefit is not. There is no accepted optimization or claim that
shared registry access is the sole scaling bottleneck. The production source
has been restored byte-for-byte to `974a941`. The candidate source snapshots
and executable remain only in the ignored experiment capture. The new model,
public regression test and this report remain local and uncommitted. No PR or
branch is pushed, no defaults change, and no additional matrix is run to seek a
favorable observation.

A subsequent design must address the scope of coordination, rather than count
locks alone: owner-local publication/close authority would need to retain
current capacity identity, invalidate cached authority at close, avoid retaining
senders past close, and make multi-owner budget replacement consistent. Those
are future refinement obligations, not behavior installed by this experiment.
