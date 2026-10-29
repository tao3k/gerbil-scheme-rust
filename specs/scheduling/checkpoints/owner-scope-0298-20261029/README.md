# Frozen 0.298 ms best-observed checkpoint

Checkpoint ID: `owner-scope-0298-20261029`.

The user subsequently requested a real Git source revision and reproduction
before further optimization. This source-checkpoint commit freezes the restored
runtime and retained model counterexamples for that purpose. Its exact revision
is obtained with `git rev-parse HEAD` in a clean checkout; the
[replay plan](replay_source.json) records the fixed command and original cohort.
This is a newly fixed reconstruction, not a retroactive claim that the old
0.298 ms binary was originally built from this new commit. Reproduction is
pending until an execution receipt is bound to this revision.

This preserves the current best observed single-owner latency, with its exact
executable, private VM image and original result. It is the comparison reference
for auditing the rejected 770.831 ms observer. The earlier 0.317 ms remains a
historical source checkpoint; it must not be used to disguise a regression
against this 0.298 ms observation. Existing automated gates are unchanged.

| Original observed metric | Frozen value |
| --- | ---: |
| Owners / Scheme workers | 1 / 4 |
| Planned rate / duration | 500,000 messages/s / 10 s |
| Successful messages | 5,000,000 |
| Successful throughput | 500,000.025 messages/s |
| p99 / p99.9 | 0.298 / 1.634 ms |
| Rejected / timed out | 0 / 0 |
| Observed outstanding / original guard | **1,415 / 128: failed** |
| Unpublished source debt peak | 1,413 |

The local artifact bundle lives under
`$XDG_STATE_HOME/gerbil-scheme-rust/checkpoints/owner-scope-0298-20261029`, using
`$HOME/.local/state` when `XDG_STATE_HOME` is unset. Its 49 payload files and
manifest were copied outside temporary storage, hash verified and marked read-only.
The benchmark retains execute permission. Binary artifacts,
raw diagnostics and source archives are local data, not additions to Git.

The [checkpoint record](checkpoint.json) contains the binary/image digests and
whole-bundle manifest digest. The original result hash is also bound by the
[Scheme receipt](../../owner_wait_scheme.json). This freeze operation did not
execute the binary, rebuild, run a test or benchmark, change limits, or push.

The executable still embeds its original build-time image location. Any future
replay must explicitly select this checkpoint's image with the existing
`GERBIL_SERVICE_BENCH_IMAGE` environment variable; do not fall back to a stale
build-cache image. This is not a hermetic OS/system-library snapshot.

Source identity is deliberately qualified:

- `source/base-6d4ec27.tar` is the original Git commit archive.
- `source/native-manifest-matched` contains qualification Scheme/C files whose
  hashes exactly match the manifest retained from the 0.298 ms build.
- `source/host-reconstruction` labels the restored Rust/build sources. Their
  complete historical byte identity was not independently retained and is not
  asserted. There is no fabricated all-source historical Git revision.

The original raw histogram was not retained, so this checkpoint does not claim
an independent retrospective rank certificate or reproducibility of precisely
0.298 ms on every run. It freezes the actual observed cohort and its negative
128-message result. It is not full performance acceptance. The initial artifact
freeze created no Git commit; that historical fact remains in `checkpoint.json`.
The later source freeze above is separate. The parent revision plus a dirty
source tree must not be called the exact historical binary revision.
