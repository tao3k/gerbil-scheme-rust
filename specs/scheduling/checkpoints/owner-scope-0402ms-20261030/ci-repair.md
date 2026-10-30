# Generated artifact repair and recovery admission

## Frozen performance identity

The performance reference remains commit `80e68254092589bbb778c9761a058da16fb6f72e`:
short-request p99 0.350 ms, p99.9 0.484 ms, 5,000,000 completions,
500,000 arrivals/s, zero rejected/errors/timeouts, and peak outstanding 1,142.
The unchanged peak gate is 128 and therefore still fails. No new performance
run or engine optimization is claimed by this repair.

The frozen executable SHA-256 is
`68e6899cd18f3c1d93a54e0eb20859aa086d1e17c797d37f8a6a1feed670a551`;
the frozen image SHA-256 is
`fa405648274b0f8cc88bdf21dbbec56a730990b2b77b506c75665460f4067b1d`.
Both were rechecked unchanged after regeneration.

## CI cause and repair

The original Cargo binding jobs on Ubuntu and macOS and the Ubuntu Bazel
Cargo gates failed at generated SCM validation. Whole-file hashes matched
their stored files. Their stored input fingerprint was
`2b982a07a96a102b43ee6747be86ce0c4decf81828c6eafd5893f16dadb8c7cf`,
while the current input fingerprint was
`662bdabc775184587cbeb2cd297e1c805c0107541d2f759d5ac2aa3b9d0dd735`.
The mismatch already exists in research checkpoint `6b35864`; the merge
inherited it. The PR's earlier parent `1c54c43` has a matching receipt under
its own input list. This is not evidence of corrupted compiler output.

`just bridge-regenerate` regenerated both tracked files and their external
whole-file receipts using the installed Darwin SDK with the same Gerbil
revision and patch identity as the pinned Darwin CI release. In particular,
`runtime.scm` previously omitted the source's exchange exports. They now
appear in compiler output. UTF-8 changes are compiler timestamp and temporary
identifier changes. No generated file or digest was hand-patched. The SDK,
Gambit sources, generation rules and validation rules were not changed.

The build-input unit fixture also omitted `runtime-exchange.ss` even though
the fingerprint requires it. The fixture now creates it and verifies that
changing it invalidates the fingerprint. A separate SDK-independent regression
checks both actual tracked files against the current workspace inputs. It
would reject the original stale checkpoint without requiring an FFI build.

Strict check-only compilation ran with updating disabled. Local validation
and remote CI remain separate receipts; the previous remote run cannot
be declared green from a local check.

## Recovery contract and complete-cycle decision

The original frame source, worker execution, independent oracle and source
consumption are unchanged. The source alone returns capacity after verified
consumption. For the integrated peak:

- arrivals due: 2,447,139;
- published: 2,445,999;
- consumed: 2,445,997;
- source debt: 1,140;
- published but unconsumed: 2.

Returning both active frames still leaves more than 128 unserved arrivals.
At 500,000/s, the zero-entry-debt allowance for a no-consumption interval is
256 microseconds. This is a necessary service condition, not a newly chosen
sleep, retry, batch size, or queue limit. Nonzero entry debt reduces it.
The recorded 1,094,625 ns progress interval is not the peak's own interval;
it must not be used as a causal reconstruction of the peak. Its enclosing
CPU readings also do not distinguish OS runnable wait from blocking.

To reduce outstanding from 1,142 to 128 during continuing arrivals requires
1,014 net completions beyond new arrivals. Hypothetical sustained consumption
rates of 600,000/s and 1,000,000/s would take at least 10.140 ms and 2.028 ms
respectively, assuming no further service interruption. Neither is a measured
capacity claim. Later catch-up does not erase a failed historical maximum.

A proposed runtime change must account for the entire owner cycle:

| Work | Required accounting |
| --- | --- |
| Source eligibility and frame search | Include the existing source cut and up to three slot inspections |
| Input construction | Include factory, allocation and roots until handoff |
| Actor dispatch and execution | Include both mailbox handoffs, scheduling and business work |
| Independent oracle | Preserve full verification and failure propagation |
| Consumption | Include original clock/deadline check, progress publication and clearing roots before returning credit |
| Recovery | Count real consumption minus continuing arrivals, not ready results or final drain |
| Added coordination | Charge every new phase, scan, callback and lifetime guard; moving work is not removing it |

The deferred-construction proposal remains cancelled: it retains the original
factory/business/oracle work on the same OS owner, adds preparation/frontier
maintenance, and has no established full-cycle saving or availability benefit.
The CI repair introduces no runtime coordination cost and claims no recovery
benefit. No new runtime proposal has enough evidence to implement safely yet.
The next proposal must establish both a bounded interval between actual
consumption opportunities and positive recovery surplus. Increasing a queue,
returning credit at readiness, reducing offered load, or relaxing the gate
cannot establish either property.

Existing `owner_recovery.qnt` now retains directed checks for the integrated
peak, the no-consumption interval bound, and net recovery surplus, reusing
its original lifecycle and recovery equation. These bounded model checks do
not certify native timing or provide a new OS scheduling guarantee. Defaults,
waiting protocol, deadlines and the measured frozen pair remain unchanged.
