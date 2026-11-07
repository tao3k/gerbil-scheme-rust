# Native typed construction at the Actor boundary

The service's short native payload is an ordinary Scheme vector containing an
s64 metadata vector and one numeric vector. Previously, constructing that payload
and its service envelope registered five external roots across several native
calls. Lexical scopes limit simultaneous external roots to three. Only the final
parent envelope needs external ownership before normal Actor submission.

`SchemeServiceInput::tagged_numeric` constructs the exact same graph in one
Scheme construction frame. Its metadata is application data, not a built-in
handler tag. All existing numeric kinds use the sealed numeric copy ABI. Child
values are owned by the Scheme frame and its collector; only the finished parent
gets one external root. Rust does not execute the handler, schedule a green
thread, reuse a payload buffer, or serialize the input.

The method is explicit and requires its optional native-image capability. A
missing symbol is an error, never a silent fallback. Both the reusable program
image and qualification image export allow-lists publish the new symbol. The
normal service endpoint, Actor queues, budget policy, exception handling,
continuations, GC, completion policy and independent retirement remain unchanged.

The cumulative input charge is identical to composing the original values:
metadata bytes + data bytes + header bytes + four root-reference widths for the
two pair vectors. For the existing64-bit four-record workload, the charge is184
bytes:183 must fail and184 must pass in both implementations. Allocation failure
unwinds the constructor frame; no child root or raw Rust pointer is published.

`native_construction.qnt` reuses the existing owner/root lifecycle. The composed
trace records five registrations, a peak of three external roots, and184 bytes.
The atomic trace records one registration, one returned parent, and the same
charge. Abort and quota rejection retain normal owner-credit retirement. A
premature external-root escape during construction is a negative mutation.
Scheme GC reachability within a native frame remains a native qualification
obligation, not a property proved by this abstract model.

The native test compares the exact Actor result,183/184 byte boundaries,
collection while the completed parent is rooted, Scheme handler exceptions and
zero roots on shutdown. The benchmark uses the same Snapshot/feed1 path and
active3 in both arms; it does not stack the rejected feed3/NativeCycle policy.
One VM, one Tokio worker, window128, native slots4/work32, five-second deadlines,
ten-second arrivals at500,000 or1,000,000/s, the four-record payload and every
successful result oracle are retained. Only the explicit constructor changes.
Success throughput, all rejection and successful p99 must be reported together.
The atomic construction timestamp includes its service envelope; the old payload
construction timestamp alone must not be subtracted as an exclusive CPU saving.

## Focused qualification, 2026-10-27

The final driver caches two immutable command-line selectors before measurement.
Previously `retire_publication_ab` scanned and allocated the complete argument
list for every constructed message. This was benchmark overhead. Both arms below
use the corrected driver; its removal is not counted as a library speedup.
The earlier uncalibrated 500k pair remains a negative local record: composed
34,986/s and29.781ms p99; atomic44,198/s and16.022ms p99. It was not accepted.

Each final arm has ten seconds of open arrivals, after excluded warmup. The
500k/s arms each offer5,000,000 messages; the1M/s arms each offer10,000,000.
Every arrival attempts library admission. Benchmark ingress drops are zero.
These are short-only native four-record messages, not the historical mixed
12-VM workload. Successful throughput includes result checking and drain time;
p99 and p99.9 cover successful messages from planned arrival to consumption.
Rejections remain in the complete arrival accounting.

| Offered messages/s | Constructor | Completed | Completed messages/s | Rejected | Rejection rate | p99 ms | p99.9 ms |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 500,000 | Composed | 553,808 | 55,367.81 | 4,446,192 | 88.92384% | 4.389 | 6.167 |
| 500,000 | Atomic | 667,038 | 66,696.13 | 4,332,962 | 86.65924% | 3.617 | 6.075 |
| 1,000,000 | Composed | 483,215 | 48,309.09 | 9,516,785 | 95.16785% | 11.222 | 40.083 |
| 1,000,000 | Atomic | 707,414 | 70,728.00 | 9,292,586 | 92.92586% | 2.888 | 5.187 |

All four arms have zero admitted request failures. Each pair uses the same live
VM and native image. Both final cleanups report zero roots, a joined owner,
quiescent service and removed image files. Each row is one observation, with the
composed arm first; it is not a stability or randomized-order study. All-arrival
generator p99 was0.786/1.010ms for the500k pair and10.957/0.674ms for the1M pair.
The large1M timing difference includes generator progress: it must not be
attributed entirely to native construction. Stage percentiles are not additive.

The frozen driver SHA-256 is
`ca78d1f9000f35d02ceceaeb498a1ba5d09ef0f16c80edd98c027734a4fd2851`.
The native image SHA-256 is
`dbc7f75804d9ba5935c5706f53cb0f02582bf0c9968b965947e743d7936d70bb`;
it is byte-identical before and after argument-selector calibration. Binary,
image, source receipts and full outcome accounting remain local ignored
artifacts. No raw logs or build products are published.

Native qualification covers all nine existing numeric element families, exact
quota accounting, GC, Actor results, Scheme exceptions and root retirement.
The full native suite passed17 cases; the final constructor changes also passed
the focused native case and strict Clippy. The construction model passed five
focused cases and1,000 bounded safety traces, in addition to the reused lifecycle
model checks. These checks do not prove native GC or timing properties.

The paired500k observation improves completed throughput by20.46%, but still
rejects86.66% of arrivals. Neither offered-rate capacity target is met. No
performance acceptance, commit or push follows from these results. Existing
constructor defaults, queues, deadlines, and the historical mixed-workload
10.141/13.039ms references remain unchanged.
