# Owner-local publication authority

This experiment starts from `974a941`, retaining the original engine and frozen
million-message driver as control. It replaces per-message shared-registry
coordination for pinned service requests with owner-local metadata gates. It
does not change Scheme execution, Actor turns, continuations, GC, exception
handling, native root custody, native-credit retirement or client consumption.

## What the preceding result establishes

In the preceding two-transaction candidate, single/twelve-owner p99 was
0.556750/4.939750 ms (8.872x), while throughput was 14,525.43/24,742.03
messages/s (1.703x). Against that same round's original twelve-owner control,
the candidate p99 rose from 4.045250 to 4.939750 ms (22.11%). These are different
comparisons: the entire one-to-twelve scaling penalty cannot be assigned to
the candidate change.

The preceding twelve-owner candidate's marginal start-to-constructor p99 was
4,289.958 microseconds, versus 27.875 for construction and 826.083 for the
constructed-to-projection interval. These marginals need not describe the same
request and cannot be added or used as causal lock-wait percentages. They make
pre-construction publication/owner handoff a concrete investigation boundary,
not proof that the registry mutex is the sole cause.

The closed-loop case has one outstanding request per owner, four request slots
and 32 work credits. It is not a saturated client-class admission scenario.
Changing short/bulk quota thresholds would change a different mechanism.

## Authority and lock order

Successful startup seals immutable owner indexes, each pointing at a metadata
gate. Before startup completes, existing internal dispatchers use the original
registry path. No public API for changing owner indexes is introduced.

A live gate holds a weak sender, wake metadata and an Arc to the current owner
capacity. The registry and gate refer to the **same capacity object**, not two
ledgers. Regular service validation, capacity observation and final publication
use only the target owner's gate. The final send checks the live state, service
capability and capacity Arc identity. A stale reservation is rejected before
Scheme entry. Generic byte/control enqueue paths retain their existing registry
fence. Backend mode readiness also retains its original mutex.

Only control-plane close/reset acquires the registry and then all owner gates
in ascending index order. A publisher takes one gate and never acquires the
registry while holding it. No native Scheme call, blocking channel send or
semaphore wait runs under a gate.

For whole-pool reset, acquire all gates before testing quiescence. If active
slots, work or producer reservations exist, fail without replacing capacity.
Otherwise replace all registry capacity Arcs and refresh all gate references
before releasing any gate. Old semaphores close after unlocking. Old snapshots
may still acquire during this retirement window; final identity validation
prevents them from entering replacement capacity.

For close, acquire all gates, invalidate registered acceptance, empty every
gate and remove registry senders at one control transition. Release locks before
closing capacity semaphores, notifying services or destroying retired metadata.
A cached dispatcher retains only empty gates after close, not native resources.
The transient upgraded sender is released before unlocking its gate. The live
registry retains the strong anchor until control can lock that gate, so this
release cannot disconnect a queue or invoke its final destructor under the lock.
No temporary sender can survive an unlocked, suspended publishing thread and
extend shutdown. Request destruction and notifications still occur outside.
A publisher on an owner not yet locked may finish during control lock collection;
it linearizes before close and must drain normally.

Per-owner publication tracks successful sends locally. Rare budget replacement
folds this fact into the original registry admission history, avoiding a new
process-wide per-message atomic write. First capacity activation after unbounded
admission remains rejected.

## Model and native evidence

`owner_publication.qnt` imports `admission_transactions.qnt`, which projects the
existing `owner_lifecycle` state. No Actor, root, credit or completion state
machine is replaced. Only owner permissions, control lock collection and gate
capacity views are added. Seven new directed traces plus six directly imported
capacity traces pass. One thousand bounded traces of at most 50 steps preserve
the combined safety predicate. Partial-reset and partial-close mutations fail
it. This is pairwise abstract safety, not a twelve-owner exhaustive proof, OS
fairness proof, or a microsecond latency guarantee.

```console
quint test specs/scheduling/owner_publication.qnt --main owner_publication --seed 1 --backend typescript
quint run specs/scheduling/owner_publication.qnt --main owner_publication --invariant safety --seed 1 --max-samples 1000 --max-steps 50 --backend typescript --verbosity 1
```

Actual public-API regressions cover cancellation and timeout credit return,
close/drain, cached-dispatcher descriptor release, and concurrent owner publication
with whole-pool budget replacement. Snapshot limits must stay uniform across
owners. A stale captured capacity can return `Closed` before acceptance; accepted
tickets must return the exact Scheme result. Full native GC/multishot-continuation,
exception and shutdown qualification and strict Clippy are separate obligations.
No model pass substitutes for native qualification or performance acceptance.

## Paired million-message experiment

Each case completes 1,000,000 short-CPU messages, one native order with four
records, using the original corpus and oracle. One live request per owner,
matching Tokio worker count, 32 warmup messages per owner, five-second guards
and 300-second external capture are unchanged. Order is original-1, candidate-1,
candidate-12, original-12. No profiler, background-task suspension, automation,
load reduction, default change or full benchmark matrix is introduced.

The frozen historical control remains 35,083.21 messages/s, 0.373500 ms p99 for
one owner and 70,829.74 messages/s, 0.810125 ms for twelve. Slower current controls
cannot establish retention of those historical observations. Admission requires
correctness, no single-owner regression, and improved twelve-owner throughput
without worse p99 or completion spread; a single paired observation still does
not establish a statistically significant causal effect. The mixed open-arrival
production contract remains separate.

The benchmark and native image must be byte-identical across control/candidate
where applicable. Raw logs, source snapshots, executables, environment captures
and native images stay outside Git. Private absolute paths are not required by
this document or the existing Rust benchmark entrypoint.

## Clarified request: 500,000 planned arrivals per second

The user clarified the request as **one VM under 500,000 planned arrivals per
second**, testing admission and successful processing capacity. The assistant's
previous interpretation as a 50,616/s completed-throughput requirement is
withdrawn. No historical source lookup or such completion-throughput gate is
required. Offered load, ingress acceptance and successful completion are
separate quantities; none is renamed to stand for another.

The directed run retains 1,000,000 total planned messages, so the planned
arrival interval is two seconds, followed by drain. It uses native-type short
requests and existing bounded quotas/deadlines. This finite offered burst does
not establish long-run sustained capacity. See [the open-arrival experiment](single_vm_open_500k.md).

## Single-owner checkpoint after reference clarification

| One-million short-CPU case | Messages/s | Whole-cohort p99, ms | p99.9, ms |
| --- | ---: | ---: | ---: |
| Original frozen executable, current observation | 26,498.82 | 0.390917 | 0.672583 |
| Final owner-local candidate | 28,907.05 | 0.382709 | 0.453667 |

Both cases completed exactly 1,000,000 messages, with zero rejection, zero native
errors, zero tracked roots, one clean owner join and removed runtime images.
The earlier **57.11%** comparison was against the assistant's mistaken
50,616/s requirement and is withdrawn as an acceptance argument. These remain
closed-loop observations, not measurements of the clarified open-arrival load.
The candidate remains unaccepted and the slower control is not a new baseline.
No causal/statistical optimization benefit is claimed from these two runs.

The candidate twelve-owner comparison was not run after the assistant
misinterpreted the requested load as a single-owner completed-throughput gate. No new twelve-owner throughput, p99 or fairness
claim is available. Historical twelve-owner records remain unchanged.
The clarified requirement is 500,000 planned arrivals/s on one VM. No
historical-source search is pending; the prior 50,616/s interpretation is obsolete.

The final candidate releases its transient strong sender before gate unlock.
Three public close/reset/resource tests and strict Clippy passed after this
refinement. Full GC/continuation/exception qualification passed before that
final sender-release ordering refinement; it is not labelled an exact-final
source full-suite run. The final frozen candidate's original three-arm smoke
passed. Further admission would require completing final native qualification
and matched performance gates, not relying on earlier passes.

| Frozen artifact | SHA-256 |
| --- | --- |
| Final candidate executable | `84efa13c7b3733c42495fca7edf140d8eaa89915cb498e9e159778df35cd0ae4` |
| Original control executable | `d85f9dd3fd1835cebef610d5e7e7f269310155194fa89fcf3030a165cb226396` |
| Shared native image | `271d52efd66b04999fbbbcca0c9d49cd5cc2d55e0d5cbd23b763dbaa40cb24ec` |

The protocol/API candidate remains isolated and **uncommitted** in the research
checkout for review. No branch or PR is pushed, no default is changed, and the
published baseline remains `974a941`. Frozen captures and source snapshots stay
in ignored storage. The earlier sender-release prototype was never used in a
million-message candidate run and is not reported as a performance result.
