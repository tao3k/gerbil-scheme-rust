# Bounded short-request concurrency in one Gerbil VM

## Meaning of the original one-slot limit

`measurement::router` explicitly constructs a routing configuration with one
short slot and three bulk slots. This is a benchmark configuration, not an
intrinsic Gerbil restriction or the number of native processors. The public
`SchemeRoutingConfig` API already accepts compatible class allocations.

`routing::RoutedRequest` owns its class semaphore permit and load charge while
waiting for native admission, executing Scheme and projecting the result.
`project(self, ...)` consumes the wrapper and drops those guards, including on
failure/unwind. Native work/slot retirement is a separate owner action. The
oneshot result can remain unconsumed after class retirement. The ingress client
charge, when enabled, is another resource and is not that class permit.

One short slot therefore serializes admitted, unprojected short requests; it
does not hold the class slot until the client polls the result. A class ledger
snapshot in pinned mode can also include admission waiters, so a load count is
not an exact count of acquired class permits.

## Gerbil mechanism and actual API

The official [Actor manual](https://gerbil.scheme.org/reference/std/actor.html)
distinguishes asynchronous `->` from reply-waiting `->>`, and describes raw
mailbox messages separately from envelopes. The existing local service uses
persistent Gerbil `spawn` workers and `thread-send`/`thread-receive` mailboxes.
No remote ensemble or per-request Actor creation is required for this workload.

`scheme/async-service.ss` already has four persistent service workers with
stateless lane selection. Short-only requests use those workers; they do not
exercise the nested bulk batch in `scheme/actor-batch.ss`. Both paths remain
intact. Multiple admitted jobs allow Scheme to keep mailbox work available,
including across cooperative waits. They do not create additional native CPU
executors or make CPU-bound computation execute on three cores.

The explicit candidate reuses the public API:

```rust
SchemeServiceRouter::new(dispatch, SchemeRoutingConfig {
    owners: vec![0],
    short_slots: 3,
    batch_slots: 1,
    short_max_work: 1,
    batch_max_work: 8,
})?
```

The control uses short/bulk 1/3. Both retain four native slots, 32 work credits,
queue capacity 128, four Gerbil workers, one native executor, 50 client tasks
and one Tokio worker. Defaults stay unchanged. One bulk class entitlement
remains; immediate physical slot availability can still wait for an old native
permit to retire. This is not an unconditional mixed-load fairness guarantee.
No generic scheduling wrapper, new runtime, native-root transfer or Scheme
capability reduction is introduced.

## Existing model refinement and native checks

`short_concurrency.qnt` imports `owner_lifecycle` and `actor_batch`. Only class
ownership, projection and client observation sets are added. It distinguishes:

- class reservation and native work/slot reservation;
- Scheme Actor execution/yield on one executor;
- projection/failure dropping class guards;
- native retirement and output-root release;
- later client observation or abandonment.

The base lifecycle conservatively retains an output root after completion;
its existing release action cleans up that abstraction. The adapter separately
records owned Rust result publication and permits client observation before or
after native-credit retirement, matching both existing publication policies.
This is not a claim that Rust output holds Scheme objects. Accepted timeout/close does not release class/native custody early.
Early class release and native retirement before projection are negative
controls. Pairwise safety is not a physical latency or starvation-freedom proof.

Run `just bridge-short-concurrency-models` for both class domains and bounded
safety exploration. Actual native API tests prove that a second request can
complete before the first result is consumed, failures return guards, three
short requests can be admitted, and a fourth waiting at the class bound holds
no native work credit. Full GC, multishot continuations, exceptions and clean
shutdown qualification are separate from model checks.

## Duration-driven offered-load A/B

The user's clarified load is **500,000 to 1,000,000 planned arrivals per second**,
not a fixed one-million-message trial. Each arm runs ten seconds:

| Planned rate | Planned messages per arm |
| --- | ---: |
| 500,000/s | 5,000,000 |
| 1,000,000/s | 10,000,000 |

`--capacity-count=0` removes the minimum-count floor only for positive timed
open arrivals. Closed-loop count-zero and empty arrival plans remain invalid.
The original per-case ten-million-sample bound, five-second deadlines/watchdog
and 300-second outer capture remain unchanged.

```console
just bridge-single-vm-short-concurrency 500000
just bridge-single-vm-short-concurrency 1000000
```

Within each pair, the same live VM and executable run control 1/3 followed by
candidate 3/1, with a one-second excluded warmup per arm. The VM identity check
must pass. All short messages retain the same native-type four-record payload
and independent Rust oracle. Generated rate, all-arrival generator lag,
successful throughput, rejection/failure counts and successful p99/p99.9 are
separate results. No offered arrival is silently removed or retimed.

Ten-second stress does not establish long-run production stability. Successful
latency excludes rejected/failed arrivals; those remain in full outcome counts.
A clean exit does not demonstrate full-load performance acceptance.

## Retained initial result and measurement interference

The first ten-second 500,000/s pair used growing outcome vectors:

| Short slots | Success messages/s | Ingress rejection | Successful p99 |
| --- | ---: | ---: | ---: |
| 1 | 5,123.75 | 4,948,583 / 5,000,000 | 203.022209 ms |
| 3 | 6,489.58 | 4,934,981 / 5,000,000 | 173.565708 ms |

Both had zero library failures and clean drain. Actual attempt rates were
499,994.20/499,997.55 per second. All-arrival generator p99 lag was
12.524209/8.110875 ms. These observations are retained, not adopted as a slower
baseline to prove a speedup.

The source allocated rejection-record storage with `Vec::new()` in the
arrival-emission future. `Runtime::block_on` drives that future on its calling
thread; spawned client tasks use runtime workers. The earlier same-thread
claim was incorrect. Millions of rejected arrivals force
geometric reallocations/copies inside the timed path. This is a concrete
avoidable observation cost; it does not prove that it caused every long tail.

The corrected driver allocates exactly the bounded plan's statistics capacity
before the arrival epoch, writes nonzero sentinels and uses `black_box` before
clearing the logical length. This commits/touches statistics pages without
creating fake outcomes. Native input construction and per-request payload
preparation remain timed. Runtime queues, quotas, Actor behavior, original
planned timestamps, rate and duration are unchanged. Both arms use the same
corrected executable; no cross-driver difference is called an engine speedup.
Statistics buffers are benchmark storage, not Scheme roots or an enlarged queue.

## Final corrected-driver A/B observations

Each configuration has seven directed tests and 1,000 bounded safety traces of
at most 50 steps. The model permits result observation both before and after
native-credit retirement, preserving the current publish-before-retire option.
The two native routing tests, full native GC/multishot-continuation/exception/
shutdown qualification and strict Clippy passed. Configuration is through one
shared router cloned across clients; creating independent routers does not
create one shared class quota.

All following arms use the same corrected executable and native image. Same-live-VM
identity assertions pass within each pair. Each arm retains its excluded
one-second warmup. No cross-driver difference is claimed as an engine speedup.

| Planned arrivals/s | Short slots | Planned messages | Successful messages/s | Ingress rejection | Successful p99, ms | Successful p99.9, ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 500000 | 1 | 5000000 | 3605.77 | 99.27584% | 366.134375 | 562.460958 |
| 500000 | 3 | 5000000 | 5683.06 | 98.86108% | 304.247459 | 643.815417 |
| 1000000 | 1 | 10000000 | 15084.96 | 98.48993% | 84.823834 | 146.747542 |
| 1000000 | 3 | 10000000 | 29434.75 | 97.05391% | 32.197000 | 104.633083 |

The 500,000/s arms completed 36,208/56,946 messages and rejected
4,963,792/4,943,054. The 1,000,000/s arms completed 151,007/294,609 and rejected
9,848,993/9,705,391. All four arms had zero library failures, accounted for every
planned message, reached zero tracked roots, joined the owner and removed runtime
image files. Performance acceptance remains **false**: neither configuration
admits and completes the offered load.

| Planned rate / short slots | Actual average attempts/s | Generator p99 lag, all arrivals |
| --- | ---: | ---: |
| 500,000 / 1 | 499,845.24 | 34.204666 ms |
| 500,000 / 3 | 499,978.45 | 11.206209 ms |
| 1,000,000 / 1 | 999,996.65 | 4.505417 ms |
| 1,000,000 / 3 | 999,967.44 | 3.674083 ms |

Average offered rates are close to the plan, but the generator is visibly
bursty. These aggregates do not establish a precise per-second uniform arrival
count. The one-million/s pair performing better than the half-million/s pair
must not be interpreted as capacity increasing with offered pressure. These
are separate paired observations, not a statistically stable monotonic capacity
curve or proof of a causal percentage gain. No new slower baseline replaces the
published measurements.

Negative results remain explicit: the half-million/s candidate's p99.9 rises
from 562.460958 to 643.815417 ms, despite its better p99. Prefaulting outcome
storage did not eliminate the tails; the initial observations remain above.
It removes a source-level allocation/copy cost, not an established complete
explanation. No machine-competition cause is asserted.

| Marginal p99 interval, ms | 500k / 1 | 500k / 3 | 1M / 1 | 1M / 3 |
| --- | ---: | ---: | ---: | ---: |
| Ingress queue | 311.982916 | 231.728250 | 61.773125 | 22.914209 |
| Preparation/admission/owner wait | 148.534416 | 106.032042 | 19.576333 | 7.824791 |
| Native service interval | 1.279541 | 1.209083 | 0.413958 | 0.404542 |
| Result-to-client observation | 0.846459 | 2.969125 | 0.233459 | 0.325417 |

Marginals cannot be added or assigned as causal percentages of end-to-end p99.
The shorter native interval does not isolate actual CPU from runtime waiting/GC.
The remaining ingress/admission tails and generator jitter are open obligations;
this experiment does not establish the Gambit compute ceiling. The explicit
three-slot API configuration is safe in tested scenarios, but is not presented
as an industrial-performance completion.

| Final artifact | SHA-256 |
| --- | --- |
| Corrected measured executable | `58eee9d29e736f3f1be439e1870e838dec4754ee0a6dfb53e70ea0d7146077a1` |
| Native image, both drivers/all arms | `271d52efd66b04999fbbbcca0c9d49cd5cc2d55e0d5cbd23b763dbaa40cb24ec` |
| Initial growing-vector executable | `378ac195dda74ae4e026c63ac23bf7fb7d98451ab0f09aa7a40fb314572e48cc` |

Raw captures and source snapshots stay in ignored local storage. Research code,
models and reports remain uncommitted; no PR update, merge or push is made on
these performance results. Defaults, queue limits, deadlines, optimization level
and Scheme capabilities remain unchanged. CPU/RSS convergence and long-run
stability are not qualified by these finite stress runs.
