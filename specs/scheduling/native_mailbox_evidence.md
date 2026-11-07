# Native Gerbil mailbox admission experiments

## Scope and fixed contract

The current task is application use of Gerbil mailboxes and green threads at
1,000,000 planned arrivals/second. Further Rust integration is deferred. No
upstream scheduler, GC, SMP implementation or public default is changed.
The Rust executable starts one native call per arm; there are no per-message
Rust entries. These are native component experiments, not production Rust API
acceptance or a measurement of mailbox-only capacity.

Each measured arm lasts ten scheduled seconds: ten million planned messages at
1M/s, or five million at 500k/s. One VM, four Scheme workers, three active native
requests, window 128, original absolute arrival timestamps, five-second request
and progress bounds, C `-O2` and the release Rust build remain fixed. Every
accepted message constructs a fresh four-record signed64 input and tag, runs
the complete order aggregation, and passes the unchanged independent 21-field
oracle. Successful latency includes planned arrival through verified consumption.
Rejections are counted separately; successful p99 alone cannot establish capacity.

The preserved historical 500k/s reference is 476,775.66 completions/s,
4.64288% rejection and 0.418 ms p99. Its 1M/s record is 505,582.60 completions/s,
49.44061% rejection and 0.644 ms p99. Neither is replaced by a new slower control.

## Results retained on 2026-10-27

| Run | Planned rate/s | Completed | Completed/s | Rejection | Successful p99 ms | Successful p99.9 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Original mailbox, pipeline pair | 1,000,000 | 4,988,275 | 498,815.40 | 50.11725% | 0.662 | 0.849 |
| Ready-drain / fixed-slot candidate | 1,000,000 | 5,117,097 | 511,697.96 | 48.82903% | 0.652 | 0.852 |
| Ready-drain 500k guard | 500,000 | 4,454,282 | 445,421.36 | 10.91436% | 0.727 | 17.667 |
| No cross-Actor execution diagnostic | 1,000,000 | 5,413,538 | 541,341.74 | 45.86462% | 0.645 | 0.999 |
| Original mailbox, data-access pair | 1,000,000 | 4,895,919 | 489,579.69 | 51.04081% | 0.681 | 0.836 |
| Guarded native data-access candidate | 1,000,000 | 5,736,383 | 572,758.19 | 42.63617% | 0.651 | 20.284 |
| Guarded data-access 500k guard | 500,000 | 4,586,112 | 458,611.20 | 8.27776% | 0.635 | 3.732 |

All listed runs completed with zero accepted errors/timeouts, zero final staged
or native requests and clean owner shutdown. All planned messages are accounted
for by completions plus rejections. No candidate meets the historical 500k guard
or admits the requested 1M/s; none is accepted for submission. No negative row
is removed. The no-cross-Actor diagnostic still uses a self-reply mailbox and
the full source/oracle; it is not a business-only or hardware upper bound.

The data-access pair improves observed completion throughput by about 17%, but
also has a much worse p99.9. Its maximum generator lag is 103.080 ms, versus
2.301 ms in its control. The candidate-only 500k guard has 84.685 ms maximum
generator lag. These observations do not attribute the lag to the host, GC,
scheduler or mailbox; that causal split has not been established.

## Changes and correctness evidence

The ready-drain candidate removes unconditional handoff when responses are
already available, independently retires bounded replies and validates active
slot generations. Its existing modular QNT refinement passes seven tests and
1,000 bounded traces (50 steps, seed 1), including forced-handoff and premature
slot-return negative cases. Native contracts cover snapshot GC, duplicate take
and close with unread/failed replies. This establishes bounded safety evidence,
not numeric performance acceptance.

Generated C shows ordinary signed64 vector access remains a procedure call in
the original order handler. The data-access candidate uses local unchecked
vector access only after existing type, length and index guards. A single
`defrules` algorithm generates the original and specialized kernels. Arithmetic,
allocation, checkpoints, suspension, GC, input construction and independent
verification stay intact. Non-fixnum output values retain the original checked
signed64 conversion; no global unsafe declaration or fixed-width arithmetic is
introduced. Generated C confirms direct `___S64VECTORREF` / `___S64VECTORSET`
operations in the specialized kernel.

Before the clock starts, both kernels are compared on ordinary and bulk input,
invalid type/shape/record values, signed64 extrema, bignum outputs, overflow,
invalid rank and a suspending handler. The contract compares returned values,
failure status and callback count; it does not prove identical exception text
or every continuation scenario. The strict release bench Clippy check passes.
The access experiment does not alter the existing lifecycle protocol/model.

## Frozen identities and reproduction

Artifacts are local under `target/replay-native-mailbox-20261027/`; raw logs are
not committed. Each directory includes a source snapshot, source manifest,
binary, image, identity and raw stdout/stderr. The base revision is
`974a9417b32882d89bb48c3c79d86360017aa68b`; it alone does not identify these
uncommitted experiments.

| Directory | Source manifest SHA-256 | Binary SHA-256 | Image SHA-256 |
| --- | --- | --- | --- |
| `final` | `abc0d1ba5472c8c2ef9dc643138c8bff4f4865def5009642b0d0e87a61c12a55` | `9ebe9532cac80fd95f1c2fa58cea8857fdd46999ce6e94e8e98c4c1aac0317ff` | `0c52a71ba093a04b75b30668c092030347d082831e03ee77478bacc0c443c7ad` |
| `direct-diagnostic` | `267df29ab143a864ad85e6a9ee75e187c55ec5edd2679123a516851eb1669304` | `ec9cc7e2e9fa07e5f384ba74d36ec44f8704649c40745e2e32027ebfe3b1c3c0` | `eb233bfc970bf57c0c6e7599a04e73d16d1dba863532f9c441683506019308e1` |
| `native-access` | `6fe5bdd3c6b8b1438726eb4c8be9750e5e289124a3c4ce33dd38227b59626873` | `e646b6ef05e6e35301b2d86ae55bbcccbb4a672cccbb6264676f0a3b11c1c84c` | `19ca0b62b958246ad74e69c133db0b6904a9e0253080ed3756122a629bcac1b9` |

From a frozen artifact directory, use the existing native launcher:

```text
env GERBIL_SERVICE_BENCH_IMAGE=./owner_instance.dylib ./gerbil_actor_arrival --rate=1000000 --seconds=10 --access-compare
env GERBIL_SERVICE_BENCH_IMAGE=./owner_instance.dylib ./gerbil_actor_arrival --rate=500000 --seconds=10 --native-access
```

Use `--pipeline-compare` for the pipeline pair or `--direct-diagnostic` for the
diagnostic, with the matching frozen image. Never mix images between directories.

## Next architectural question

Do not infer a Gerbil mailbox ceiling from this end-to-end driver. The native
caller serially creates inputs, supplies workers, receives results, runs the
full oracle and records progress/latency. Removing cross-Actor execution alone
did not remove that shared work; reducing handler access cost improved one rate
but did not preserve the guard. The next question is how a bounded native supply,
execution and result-ownership cycle separates necessary application work from
driver work without moving the original clock, dropping verification, enlarging
queues or hiding overload. Resolve that ownership/cost contract before another
scheduler change. Keep the original full-workload benchmark as acceptance.
