# Owner-local native type exchange (experimental)

The rejected resident completion-Actor candidate is isolated from this experiment.
Its source and failed capacity receipts are retained separately. This candidate
starts from the pre-collector service, retaining single-transaction construction,
joint staged-input/client lease ownership, cancellation withdrawal and the
original per-request publication deadline. It adds no Actor, worker or scheduler.

## Protocol

An explicit idle-owner API, `configure_scheme_service_exchange_on(owner, true)`,
requires Snapshot completion, ABI34 and all three native exports. Command49 is
reserved for this capability; command48 remains the native scheduling query.
The capability has one internal root and is retired with the owning image.
Changing completion policy requires disabling exchange first. Defaults do not change.

1. `SchemeServiceInput::enter_tagged_numeric` charges the same logical input bytes
   as the existing constructor before attempting entry. Within one synchronous
   native call, it copies the borrowed numeric spans into fresh Scheme values and
   submits them to the existing service. The return value is a positive scalar
   job identity, never a root masquerading as an identity. Rust pointers are not
   retained after return. Scheme exceptions use the existing diagnostic channel.
2. One harvest copies bounded ready identities, progress and statuses to Rust and
   retains only the result batch. It drains currently visible short-ready results
   and preserves the existing general-class selection rule. It does not wait
   for future completions or change handler execution budgets.
3. `SchemeServiceOutput` is owner-local and single-use. Generic projection takes
   an independent temporary root, preserving every existing Scheme type. Explicit
   bounded numeric projection validates the family/size, copies into initialized
   Rust storage and clears its native slot in one call. Validation failure leaves
   custody intact until rooted take/discard. A failed take retains native capacity
   for the existing drain path. Every successfully consumed slot retires separately.

The original fixed owner executes Gerbil handlers, green threads, continuations,
exceptions and GC. Rust retains existing dispatch, producer, capacity and owned
result responsibilities. Input preparation remains bounded and root-free.

## Formal and native obligations

`native_exchange.qnt` refines `actor_completion`, which already refines Actor
supply and the joint continuous lease. It checks copy-before-acknowledgement,
slot custody on failed projection, independent retirement and abandoned-client
close drain. An early-acknowledgement mutation is rejected by the safety property.
Model results are bounded evidence, not a native or performance admission.

Native tests must exercise byte quota boundaries, forced GC, caught exceptions,
wrong-family/undersized outputs, repeated projection, generic multishot
continuations and clean shutdown. Existing staged cancellation and publication
expiry tests remain mandatory. Generated Scheme sources use whole-file receipts
including the new runtime source in the build-input fingerprint.

## Capacity admission

Use the existing Rust benchmark entrypoint, `bridge-single-vm-native-exchange`,
with planned rates of 500,000 and 1,000,000 messages/second. Both arms use the same
VM, payload, single-transaction constructor, bounded producer, capacity/window,
expiry and five-second watchdog. The candidate differs only in typed exchange.
Each ten-second arrival interval plans five or ten million messages, respectively.
Report API admission and useful completions during the arrival interval separately
from drained end-to-end throughput, backlog, pre/post-admission expiry and p99.
Zero QueueFull is not zero failure. Native mailbox component capacity is a
reference, not evidence of typed API capacity. Failed controls or watchdogs do not
constitute a completed A/B pair. No performance acceptance has yet been established.

## Local correctness receipt (2026-10-27)

The isolated working tree is based on `974a9417b32882d89bb48c3c79d86360017aa68b`
plus explicitly snapshotted research changes; that commit alone is not the
candidate identity. The native service suite passed 23/23 tests, including the
three direct-exchange tests. Strict Clippy passed for the qualification service
test and benchmark. Three model scenarios and 1,000 bounded random traces passed
at seed1/max50 steps. None of these checks establishes the offered-rate target.

An earlier native validation run failed after a maximum-output-length overflow:
the unconsumed slot forced service close. The corrected path takes/discards the
slot before reporting the expected bounds/allocation error. The regression test
now covers an undersized output, wrong numeric family and integer-size overflow,
then verifies another successful request. The failed receipt is retained locally.
Sandbox-denied access to the test's own `/dev/fd` descriptors is recorded
separately from native protocol failures. Original limits were preserved.
