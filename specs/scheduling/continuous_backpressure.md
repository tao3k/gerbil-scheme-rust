# Continuous supply with bounded backpressure

`SchemeServiceContinuous::submit_with(deadline, prepare)` waits for an existing
outstanding-window permit instead of returning `QueueFull` when that window is
full. The existing immediate `try_submit_with` remains available, unchanged, for
explicit shedding/capacity diagnostics. Neither API changes production defaults.

The waiting future owns no input payload, native permit or Scheme root. Only
after the existing permit is acquired does the shared submission helper recheck
cancellation, closure and the original absolute deadline, then prepare and stage
one typed request. Dropping the waiting future cancels its semaphore acquisition.
Preparation errors drop the permit. Seal/cancel closes the semaphore and wakes
waiters. Native acceptance still retains roots and native credits through normal
retirement, even if the client drops its ticket.

The same outstanding credit covers staged input, native execution and an unread
owned result. It returns only after both the staged job and ticket have released it. A bounded producer
must therefore run independently of result consumption; publishing all inputs
before observing any outputs can exhaust the window by design. Returning this
credit early would merely create an unbounded unread-output queue, which the
existing model correctly rejects. Gerbil still schedules Actor/green-thread work;
Tokio only waits for host admission and observes owned output.

`continuous_backpressure.qnt` reuses the continuous-source and Actor/owner models.
Waiting is separate from the existing window and owns no native resource. Directed
cases check consume-to-admit progress, cancellation/expiry without publication,
and closing waiters. These are abstract safety checks, not a throughput proof.

Backpressure does not make an over-capacity offered rate disappear. A correct
open-arrival qualification must retain original planned timestamps and deadlines,
and report actual admission rate, outstanding upstream obligations, timeout/error
counts, completed throughput and successful latency together. Zero `QueueFull`
with growing upstream delay or deadline failures does not qualify500k/s. Do not
retime requests after admission or substitute a closed-loop result for this gate.

## Current correctness receipt

The native regression suite passed18 cases in40.49s with unchanged five-second
gates. The new focused case verifies pending admission without preparation,
absolute expiry, consumption-driven permit return, dropping a pending waiter,
and waking an already-polled waiter on seal. Strict Clippy passed for the
native test target and new Actor bench. The backpressure model passed ten
inherited/focused cases and1,000 bounded safety traces. These are correctness
receipts; sustained Rust-path capacity under backpressure has not been measured
or admitted by this change. No commit, push or default change follows.

## Joint-lease and withdrawal correction, 2026-10-27

The first open-arrival backpressure qualification exposed a real inconsistency:
a ticket timeout/drop returned window credit while its input could remain in the
source queue. A later publisher could obtain that credit but still hit the fixed
queue bound. This was not fixed by increasing capacity or ignoring `QueueFull`.

The existing per-request cancellation allocation now also owns the window permit.
The typed job and ticket share it. Client cancellation alone cannot return a
credit while a staged/claimed job still owns it. No extra lease allocation is
introduced. If the request has not started, ticket drop attempts actual queue
withdrawal using the existing `Request.started` allocation identity. Withdrawal
and `Source::take` serialize on the same queue mutex. Only one wins; a claimed
job remains owner-owned and drains normally. Removed request destructors and
wake notifications run outside the queue lock. IDs do not depend on application
message numbers and the key cannot be reused while its ticket still owns it.

The deterministic owner-barrier scenario failed with the pre-fix implementation
and passed after correction. The final strengthened scenario checks both dropped
and timed-out queued tickets: a replacement can be staged while the owner remains
blocked, but another request still sees the unchanged full window. Thus canceled
input is actually removed before replacement admission, not merely forgotten by
the window counter. The full final native suite passed23 cases, strict Clippy
passed, and the shared-lease model retained the premature-return negative control.

`continuous_lease.qnt` refines the earlier ticket-window model; it is now reused
by Actor supply/completion and waiting-admission models. Native credit/root
retirement remains separate from shared Rust record custody. The old frozen
records remain evidence for their original implementation, not current behavior.

The final publication fence also uses `min(request_deadline, source_deadline)`
at queue publication. A valid check before preparation is insufficient: the
preparation closure can finish after the request expires. The native deadline
case covers immediate and awaiting admission, and `publication_deadline.qnt`
retains the longer-source-lifetime mutation. Expiry after a valid publication
cannot silently free joint custody or native resources.

Current performance qualification remains failed; see
[mailbox_service.md](mailbox_service.md). Correctness of backpressure does not
turn expiry into capacity, and zero `QueueFull` is not zero failed requests.
