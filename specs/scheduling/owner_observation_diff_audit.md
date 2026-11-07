# Static audit of the rejected peak observer

This audit executes no VM, compiler, test, model checker or workload. It reads
Git changes, the retained rejected source, the two frozen Mach-O images and
existing receipts. Disassembly and digest verification are static inspection.
No runtime, SDK, default, wait protocol or deadline change is proposed here.

The comparison reference is the [frozen 0.298 ms best observation](checkpoints/owner-scope-0298-20261029/README.md).
`6d4ec27` and its historical 0.317 ms remain preserved. The latter must not
replace 0.298 ms to make a slower observation appear successful. Neither
observation passes the separate 128-message outstanding guard.

## Scope and identity

The current `git diff` against `6d4ec27` includes earlier observer work and
formatting. It omits untracked implementation files, and the rejected runtime
delta has already been reverted. Consequently it is not the experiment delta.
This audit also compares the saved rejected source with the restored runtime
files, and inspects the actual images bound by the receipts:

| Image | SHA-256 |
| --- | --- |
| 0.298 ms observation | `92eba95f7972033f4c9980344f0d62070ed38ea8f5c8e9513a1f9692141900d1` |
| Rejected 770.831 ms observation | `9b3eb968dc44ca113a2313c4a87add3374da6abeded70079cc1b1918bb61d54d` |

The Scheme/C runtime files still match the original manifest retained from the
0.298 ms build. The rejected collector is now test-only in
`tests/owner_wait_recovery.h`. Its cold report module is not imported by the
running bench. Model counterexamples and negative receipts are retained.

## Findings

### 1. A per-drive observer replaced a cheap peak check with continuous work

`instance-mailbox-backpressure.ss` invokes `mailbox-scoped-source` at each
`drive` iteration, before supplying work. The idle path yields and returns to
`drive`, so callback frequency is NOT bounded by successful-message frequency.
There are at least five million such calls in the main arm; idle iterations
were not counted. The 1,000/s warmup uses the same loop.

The rejected `owner_recovery_source` validates the previous counters, selects
CPU cuts, updates `last` on every iteration, and also updates `stable` whenever
outstanding debt is below the guard. It does not charge only for a new peak or
an actual transition. It passes clock aggregates by value and constructs/copies
marker aggregates. These are owner-CPU work even though they allocate no Scheme
object, read no new clock, acquire no lock and never wake another thread.

The actual native code confirms the cost was not optimized away:

| `_owner_scope_scheme_source` | 0.298 ms image | Rejected image |
| --- | ---: | ---: |
| Symbol text, including its emitted padding | 192 bytes | 1,160 bytes |
| Static instruction words in that symbol | 48 | 290 |
| Entry stack reservation | 64 bytes (`0x40`) | 352 bytes (`0x160`) |

In the rejected image, instructions at `0x10e4` through `0x1130` load the two
cached clock aggregates and copy them to stack slots, before the guard check at
`0x1138`. The normal path retains writes to `last`; the below-guard path also
writes `stable`. Inlining removed the helper call, not its work. The collector's
embedded TLS state grows by 712 bytes, as corroborated by the original peak
field moving from offset `0x198` to `0x460`.

These sizes are NOT dynamic instruction counts, cycle measurements, or proof
of a sixfold runtime cost. The additional static TLS resolver call site lies
on a failure path; it must not be counted as an additional lookup on every
successful callback. No shared-lock or false-sharing cause is established.

### 2. A sample-only helper changed the every-completion entry frame

`owner_recovery_sample` runs inside the already sparse CPU-sample branch in
`owner_scope_scheme_publish`; it adds no clock-read cadence. Nevertheless the
native callback's entry stack reservation changes from 320 (`0x140`) to 368
(`0x170`) bytes. That entry is traversed on every verified completion, including
non-sampling completions. This is a concrete reason to inspect generated code
rather than call a branch-local C helper free. Frame growth alone does not
prove that every extra byte is written or that the stack adjustment is slower.

### 3. The rank certificate was outside elapsed time but inside VM reuse

The rejected Scheme helper walks all 5,000,000 u64 histogram slots, reading
40,000,000 bytes per arm even for a 1,000-message warmup. It adds several bucket
comparisons on each iteration. This runs after the original `elapsed` and p99
values have been computed, but BEFORE the outer worker stop/join/drain and
before the synchronous call returns.

The original bench reuses the same pool for warmup and the main arm. Therefore
the full scan, longer histogram/root lifetime, cache effects and subsequent
allocations can change the main arm's starting state. The watchdog also remains
active until receipt projection returns. Being outside the reported elapsed
interval does not make work harmless to the whole experiment.

There is an important exclusion: the rejected warmup already reports p99
17.791 ms, versus 0.010 ms previously, BEFORE its first certificate scan. That
scan cannot directly cause its own already-recorded warmup tail, nor can the
main arm's post-run scan directly cause that arm's already-recorded p99. No
exclusive attribution of the regression to histogram scanning is justified.

### 4. Cold report construction changed heap state and the compiled module

The rejected recovery report adds four tables, 63 named FFI queries and 63
constructed query strings, followed by Rust collection/projection/JSON work.
This is post-measurement work, not per-message Rust scheduling. It can affect
same-VM reuse after warmup and the time to original cleanup. It is not a direct
charge to the completed arm's histogram entries.

The extra Scheme functions were included in the common `actors.ss` graph,
including images where the new path was not selected. The existing compiler
uses `___SINGLE_HOST`; the SDK defines module-level generated C host code in
that mode and suppresses separate procedure host bodies. Consequently adding
cold functions can change the compiled module's layout and optimization. A
false Scheme macro removes calls from the control path, but does not establish
whole-image identity or unchanged instruction/cache placement. This is a
confound, not proof that layout caused the tail.

### 5. Several visible changes cannot be charged as new per-message work

- The generation allocator performs a checked reservation at cold scope open;
  it is not a new shared atomic update on every message.
- The select wrapper's clock/cancellation association was already present in
  BOTH observed images. It belongs to the earlier observer delta against Git
  HEAD, not the new peak-observer delta between 0.298 and 770.831 ms.
- Big-integer report projection and Rust argument scanning occur outside the
  native per-message service loop. No new Tokio/Rayon scheduler or Actor queue
  was introduced by this delta.
- The percentile formula, bucket width, five-second gates, original offered
  schedule and frame-return order are unchanged. The rank check certifies the
  rejected run only; it does not reconstruct the previous raw histogram.

## Why a modest service deficit can produce a huge tail ratio

The old observed arm completed at 500,000.025/s; the rejected arm completed at
464,313.165/s. Those are historical observations, NOT a causal paired A/B.
Under a fixed 500,000/s arrival schedule, sustained under-service accumulates
source debt; the latency effect need not scale linearly with callback cost.
The rejected peak's 372,198 outstanding messages represent 744.396 ms of
scheduled arrivals, the same scale as its 770.831 ms p99. They are distinct
sampled quantities, not an equality or proof of a particular cost source.

The existing episode data also has a 2.648288 s non-CPU lower bound. Added C
instructions run on CPU and cannot simply be assigned that entire absence.
No paired observer-cost control or matching owner scheduling/blocking trace
exists. The defensible conclusion is that substantial hot work and experiment
confounds were introduced without cost qualification, not that Git diff alone
explains every millisecond or that the machine is responsible.

## Lower-interference direction, not an implementation authorization

1. Keep the 0.298 ms binary/image pair immutable as the best observed reference.
   Keep the 6d4ec27 historical source baseline and all negative guard results.
2. Reuse the existing native peak-update event and already sparse CPU cuts.
   Additional capture belongs only inside those event branches. No continuous
   `last`/`stable` state machine, by-value clock aggregates or new per-drive FFI
   crossing should be added to obtain a recovery timeline.
3. Read existing cached cuts by reference and copy only a selected event's
   scalar evidence. Scope/generation validity and exceptional closure still
   apply. If matching cuts are missing, report unknown; do not borrow the
   longest gap's CPU/GC or pretend verification has already returned a frame.
4. Limit the first proposed capture to an actual peak bracket. This does not
   supply an exact continuous consumption/recovery curve at zero cost. Exact
   recovery history would need a separately priced sampling contract; it must
   not be smuggled into every source iteration.
5. Keep rank-algorithm correctness in the retained test fixtures. Do not place
   a full histogram scan and new report envelope between performance warmup
   and main work. Fixture correctness cannot independently certify a historic
   histogram that was never retained.
6. Avoid adding cold observer functions to the common Scheme module solely for
   reporting. Before any future execution is authorized, inspect common-path
   assembly, stack/aggregate copies and the Scheme-object identity produced by
   the existing build pipeline. A missing default-path source diff is not
   enough. Keep default values, wait semantics and gates fixed.
7. Model safety/ownership and observation cost are different obligations. The
   retained QNT cases cover the former and finite-source admission mistakes;
   passing them does not qualify the hot cost of a collector. No new model,
   observer, build, test or performance run is added by this audit.

The audit changes only documentation/checkpoint metadata. It neither restores
the rejected collector nor promotes any rejected result. No commit or push.
