# Build-budget investigation for 902cbfe

The completed CI run for 902cbfee27424a6e0c8820aadb52501f14d44c9d has seven
successful jobs. Both bindings jobs, both Bazel jobs, both Quint shards and the
single-processor Actor/Rust qualification passed. Fixed-owner lifecycle and
Owner Rust lint were cancelled. Their check annotations explicitly report
exceeding the unchanged 30-minute job execution limit.

The cancelled logs end during compilation of gerbil-scheme-qualification;
cleanup terminates compiler/build-script children. They do not establish a
five-second runtime watchdog failure, OOM, or a particular native module as the
cause. Runner-capacity notices alone do not attribute compiler elapsed time.
No native lifecycle completion or final lint admission can be claimed.

The follow-up changes build diagnostics only. Cargo verbose output exposes the
existing typed archive observations while the build script is still running.
The same observer now brackets producer-module compilation and compiler-graph
staging, in addition to its existing per-object phases. A start without a
matching completion identifies an outstanding operation; parallel object
starts must be matched by module identity, not by the last printed line alone.

No compiler optimization, compiler verbosity environment variable, worker
count, feature, timeout, cache policy or runtime observation is changed. No
test is removed. Existing phase-observer contract coverage and focused strict
Clippy pass locally. These checks admit the diagnostic wiring, not the full
native build or a build-time fix. A subsequent exact-head CI receipt is still
required to identify and address the expensive phase.

The frozen 80e6825 executable/image and its historical 0.350 ms p99 remain the
performance reference. Peak 1,142 still fails 128. No scheduling proposal is
part of this diagnostic change; PR #18 remains unmerged.

Source: [CI run 41009298021](https://github.com/tao3k/gerbil-scheme-rust/actions/runs/41009298021).

## f1e3beb result and identified module

[Run 41015460124](https://github.com/tao3k/gerbil-scheme-rust/actions/runs/41015460124)
completed with five successes, four cancelled jobs and the unchanged SMP skip.
Both bindings and both Bazel jobs passed, as did Quint shard 0. Shard 1 exceeded
its original 15-minute budget; it is not a model counterexample receipt. Actor,
lifecycle and owner lint did not complete within their 30-minute budgets.

The lifecycle job now identifies the outstanding object unambiguously:

| Operation | Result |
| --- | --- |
| Producer modules | Completed in 36.819 s |
| Compiler-owned graph | Completed in 4.655 s |
| Generate 141 module C inputs | Completed in 4.847 s |
| Generate qualification/actors C | Completed in 2.720 s |
| Native object batch | Ten jobs, three workers |
| Nine other objects | All completed; slowest was async-service, 86.402 s |
| qualification/actors object | Started 06:44:24.355 UTC; no completion before cancellation around 07:10:53 UTC |

The identified budget consumer is C object compilation of
gerbil-scheme-rust/qualification/actors, not Scheme expansion or runtime work.
The log does not identify the expensive internal C optimizer pass. A local
control generated C has 8,736,081 bytes; its largest selected procedure is the
scoped backpressure variant, 1,147,315 bytes and 1,323 labels. These are static
local source counts, not proof of the remote compiler's exact input identity or
pass-level CPU time. The module combines lifecycle, services and all mailbox
qualification variants through includes. Any compilation decomposition must
preserve those variants and their shared state; removing test cases, lowering
optimization or lengthening the deadline is not an accepted repair.

Owner lint's first command completed at 07:07:32 UTC. The second reached
qualification compilation at 07:10:35 UTC and timed out without phase output.
The diagnostic syntax was wrong for this external Cargo subcommand: Cargo's
prefix `cargo -vv clippy` does not forward the required verbosity. A focused
cached no-feature check emitted one line with that spelling, versus 112 lines
and 104 Fresh/Running records with `cargo clippy -vv`. Both checks passed strict
Clippy. Correct only the two Clippy argument positions; the builtin bench
command already exposes its observations and does not need this repair.

The lint module therefore remains unconfirmed until its own records arrive.
Do not attribute it to the lifecycle module merely because both jobs compile
the qualification package. No build-time improvement is claimed by this
diagnostic correction. All existing features, commands and budgets remain.
