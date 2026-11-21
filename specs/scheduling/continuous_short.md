# Bounded continuous short service

## Protocol and implementation

This is an opt-in extension of the existing owner-resident service producer, not
another Scheme executor. The owner still enters the original resident Gerbil
service through the native typed request interface. Scheme retains its Actor
workers, green threads, continuations, exception handling and GC. No Scheme
source, native image, default, native queue capacity or watchdog changes.

`open_scheme_service_continuous` registers once before arrivals. The source stays
open when empty. `try_submit_with` obtains a bounded host window credit before
preparing Rust input, validates a stateless one-unit short request, and publishes
to the existing source. Publication precedes its existing empty-to-nonempty wake.
The owner producer supplies requests whenever its native active/work/slot credits
permit. Results are independent tickets, available before source seal.

Three ownership intervals are distinct:

- The live ticket window spans prepared Rust input, native work and unread owned
  output. Waiting for or dropping a ticket returns that window credit.
- The source also enforces its input count under its publication mutex. Abandoning
  a ticket cannot bypass this bound by publishing unlimited cancelled inputs.
- Native credits and roots survive caller abandonment and return at native
  retirement. They do not depend on client consumption. Abandoned work already
  beyond the entry check still drains on its original owner.

The live ticket limit is 128 in the directed bench, replacing the old bench
queue of 128 rather than adding another queue. Abandoned requests have separate
bounded source/native ownership; they are not claimed to remain charged to a
live ticket. The active native limit is 1 or 3, preserving the four-slot native
budget. A ticket can expire before entry without constructing a native payload.
Its absolute deadline is the original planned arrival plus five seconds. The
finite producer lifetime covers the ten-second arrival window plus that gate;
it does not extend any individual request deadline.

Seal stops new submissions and permits draining. Cancel/drop rejects staged
inputs and wakes the existing producer. Native accepted work always drains.
Rust preparation callbacks must be bounded and must not enter Scheme. The API
is intentionally restricted to independent short requests; stateful Actor lanes
continue to use their existing owner-affine APIs.

## Model refinement

`continuous_short.qnt` imports `bounded_short` from `short_concurrency.qnt`, which
already imports the owner lifecycle and Actor models. It adds a host outstanding
window and monotonic seal state. The two-slot window is a finite reduction, not a
new runtime default. The model distinguishes native retirement, result
publication, client consumption and abandonment.

The safety invariant requires all nonabandoned unobserved work/results to retain
window ownership. A mutation releasing the window at projection violates that
invariant. Directed traces cover supply while an earlier result is unread,
seal/drain, pre-entry cancellation, and accepted cancellation without early native
release. Random bounded safety exploration is not an unconditional liveness or
performance proof: owner and client scheduling still need progress opportunities.

## Directed Rust benchmark

Use the existing `scheme_service` bench with `--capacity-continuous`,
`--capacity-short-only`, `--capacity-short-slots-ab`, one owner and one Tokio
worker. Each arm lasts ten seconds, excluding warmup. The rates remain 500,000
and 1,000,000 planned arrivals per second: five and ten million total arrivals
per arm. The original clock, payload, per-message oracle and five-second gate
are retained. No batch merges multiple service messages into one count.

Every arrival invokes the public continuous library admission API directly;
there is no benchmark ingress queue or receiver-mutex/client forwarding stage.
Native result observation runs concurrently on Tokio. The result tasks are
reaped between bounded emission turns. Full host windows are explicit library
rejections; rejected arrivals are never reported as successful throughput.

The JSON separates `benchmark_ingress_rejected_messages` (zero on this path),
`library_rejected_before_admission`, accepted execution errors, successful
throughput and success-population latency. Generator lag is reported for all
attempts. These labels do not turn high rejection into acceptance. The previous
97--99% rejection captures remain negative controls; removing the benchmark
queue is a protocol change that must demonstrate real completed work.

Run the model with:

```text
quint test specs/scheduling/continuous_short.qnt --main=continuous_short
quint run specs/scheduling/continuous_short.qnt --main=continuous_short --invariant=safety --max-samples=1000 --max-steps=50 --verbosity=1
```

Native qualification lives in `tests/scheme_service/continuous.rs`, including
unread-result bounds, sealing, preparation failure, cancellation, expiry before
construction and recovery after construction failure. Results and any negative
performance outcomes must be appended before admission. No push is authorized
by model or lifecycle success alone.

## Captured observations (2026-10-27)

The directed continuous-supply captures used the frozen executable SHA-256
`52387707c306e69872b2a79cc95adfaff51b2be4786acd9c7a399bb26fcfca37` and
native image SHA-256
`271d52efd66b04999fbbbcca0c9d49cd5cc2d55e0d5cbd23b763dbaa40cb24ec`.
The image is byte-identical to the preceding negative-control capture. Source,
contract and environment snapshots are retained with the ignored local capture;
raw logs and binary products are not repository artifacts.

| Planned arrivals/s | Native active | Planned messages | Successful messages | Completed messages/s | Library rejection | Successful p99 | Successful p99.9 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 500,000 | 1 | 5,000,000 | 578,588 | 57,844 | 88.42824% | 7.290 ms | 15.048 ms |
| 500,000 | 3 | 5,000,000 | 563,931 | 56,381 | 88.72138% | 7.364 ms | 10.448 ms |
| 1,000,000 | 1 | 10,000,000 | 559,067 | 55,893 | 94.40933% | 9.200 ms | 20.967 ms |
| 1,000,000 | 3 | 10,000,000 | 540,571 | 54,045 | 94.59429% | 6.833 ms | 29.846 ms |

All four arms had zero benchmark-ingress drops and zero accepted-request errors.
Every planned message is accounted for as a completion or explicit library
window rejection. Each pair retained the same live VM identity. Both processes
reported one joined owner, zero remaining roots, service quiescence and removed
runtime files. The five-second watchdog was unchanged.

The protocol removes the external forwarding barrier and demonstrates more real
completed work than the preceding captures. It does **not** admit 500,000 or
1,000,000 messages/s. Rejection remains unacceptable for that goal. Three active
requests did not improve throughput over one; at one million planned arrivals/s,
p99 improved while p99.9 worsened. Do not increase concurrency or queue size by
default based on these observations.

There is no quiet-host claim: concurrent Clippy/native build activity was observed
and preserved in the environment receipt. All-attempt generator p99 lag ranged
from 2.068 to 4.851 ms. Each pair is one observation, not a stable production SLO
or a proof that OS interference caused the remaining limit. The old JSON `api`
and `router_short_slots` labels were inherited on the measured executable;
`continuous_supply=true` identifies the actual continuous path, and the slot
field means producer active concurrency. The source metadata label was corrected
after capture, alongside lint-only code organization. The frozen measured source
is preserved independently; no post-cleanup performance rerun is claimed.

The next optimization boundary is the accepted-message service path, not a larger
pending queue. The current roughly 54--58 thousand completed messages/s cannot
be presented as half a million. Construction, native submission, Gerbil service
execution and projection must remain distinct when locating its limiting cost;
these aggregate captures alone do not assign that cost to Gambit or prove a
native CPU ceiling. This candidate remains local and unaccepted; no defaults,
PR update, commit or push follow from these results.

Final local correctness checks: 13 directed QNT cases and 1,000 safety traces
(maximum 50 steps) passed, including the lost-wakeup and early-window-release
negative mutations. Three native continuous-service tests passed on the final
API; strict Clippy for the qualification bench and test target passed. No whole
runtime, production liveness or performance acceptance is inferred from these
focused checks. The native test process completed cleanly; no watchdog relaxation
or task interruption was used.
