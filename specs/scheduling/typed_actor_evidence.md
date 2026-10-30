# Typed Actor construction: evidence and acceptance boundary

## Source identity and comparison contract

The local research tree is based on commit
`974a9417b32882d89bb48c3c79d86360017aa68b`, with uncommitted research changes.
That commit alone does not reproduce the measured candidate. The frozen binary,
image, modified-source snapshot, source digest manifest and raw outcome records
are retained together in ignored local build artifacts. The binary and image
identities and complete four-row results are recorded in
[native_construction.md](native_construction.md).

Both arms keep one live Scheme VM, four resident Scheme service workers, one
Tokio worker, three native active requests, window128, native slots4/work32,
Snapshot completion, feed1 and five-second request/watchdog gates. The same
four-record native input and result oracle are used. Atomic construction is an
explicit opt-in; it changes neither the scheduler policy nor default queues.
Each arm offers ten seconds of arrivals at500,000/s or1,000,000/s, independently
of completion. Rejected arrivals are not counted as successful throughput.

## Evidence chain

| Claim | Source or check | Limit |
| --- | --- | --- |
| Same Scheme graph, fewer external roots | `SchemeServiceInput::tagged_numeric`, native constructor and `native_construction.qnt` | Five registrations become one; peak roots three become one. This does not remove payload allocation. |
| Original input quota retained | Native183/184-byte boundary tests and quota model | Native frame GC reachability is checked natively, not proved by QNT. |
| Normal Gerbil execution retained | Actual Actor result oracle, GC and Scheme exception tests | Rust constructs/transports types; it does not execute Scheme handlers. |
| Root and capacity retirement retained | Existing owner lifecycle, constructor abort tests, native shutdown receipts | Four arms have zero admitted failures; each pair ends with zero roots and joined owner. |
| Focused local benefit | Calibrated paired result table | One observation per arm, control first; no repeatability or production-capacity claim. |
| Failed rate target | Complete planned = completed + rejected + failed accounting | Best500k arm rejects86.65924%; neither offered rate is admitted as production capacity. |

The argument-selector cache is a benchmark calibration repair applied to both
arms. Its benefit must not be attributed to the library. The rejected previous
feed3/NativeCycle experiment is separately recorded in
[actor_supply.md](actor_supply.md), not stacked onto this constructor comparison.
Historical mixed12-VM10.141/13.039ms results describe another workload and remain
separate references.

## Source-grounded next boundary

Resident service workers already take another queued job inside Scheme via
`service-handoff!` and `service-next!`; they do not need a new Tokio task for each
Actor turn. The host source nevertheless constructs and submits one request,
then the ordinary owner loop visits `collect` before the next producer input.
With feed1, active3 alone does not guarantee three runnable Actor inputs.
Readiness, result take, projection, root release and permit return occur before
the next supply opportunity. A native permit is intentionally retained through
projection. Its release boundary cannot be moved earlier without preserving the
result graph's independent ownership.

This identifies a protocol coupling, not its exclusive measured CPU cost.
The earlier feed-only experiment failed; merely changing a batch threshold is
not an accepted remedy. A future protocol must distinguish runnable Actor input,
completed native result custody, and client-owned output. It must permit supply
and bounded completion progress without requiring all client consumers to run,
while retaining exact per-result roots, permits, cancellation and close drain.
Reuse the continuous-source, Actor-supply and completion models for that change.
Do not infer a500k/s handler capacity from the present successful-only p99.
