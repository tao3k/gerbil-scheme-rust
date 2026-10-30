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
