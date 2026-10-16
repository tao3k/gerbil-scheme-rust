# Installed Wheel Qualification: Source Snapshot 020

This is a local partial qualification, not a release, complete regression,
parallel Scheme VM acceptance or exact-head CI result. No old wheel is counted
as a new candidate. The working checkout remains uncommitted.

## Fixed Inputs

The build source is `/private/tmp/gerbil-scheme-python-source-020`, copied from
base commit `2c82b845c141ce68147cf74cb5a4ec680c6ffb07` plus the current changes.
The source tar SHA256 is
`988a1a2829114fff6666816be0a4805b8c21e75396fad7e84c24e46f89e6bfba`.
All 1381 source files were independently rehashed after the first builds with
no content mismatch. AppleDouble metadata is hashed separately, not counted
as code or runtime resources. The original archive remains intact.

All three Cargo lockfiles pin ASP to
`1ca41124b846dec947f3d05119cafa07eaa57924`. ASCENT remains the actual Git
dependency at `e6884b83c7b141ec66e2095252e4fb9e17da9869`, not a dirty path
dependency. Local Git-object transport preserved that URL and commit; it does
not close the earlier remote-fetch failure.

## Actual Builds and Installed Tests

| Wheel | Features | Build seconds / cap | Installed tests | Whole-process seconds / cap |
| --- | --- | --- | --- | --- |
| Ledger 020 | Rust only | 36.061 / 120 | 6 passed | 0.532 / 15 |
| ASCENT 021 | Rust only | 32.617 / 120 | 3 passed | 0.685 / 15 |
| Ledger 023 | Scheme only, official wheel repair | 1.315 / 120, reuses build 022 | 13 selected passed | 3.277 / 15 |
| Ledger 024 | Scheme and Rust, official wheel repair | 47.352 / 120 | Rust: 6 passed; Scheme: 13 selected passed in another process | 2.935 / 15; 0.295 / 15 |

The Scheme-only compilation in 022 took 50.113 / 120 seconds. Its external
library packaging warning was retained; 023 uses maturin's official
`--auditwheel repair`. It is not a second cold compilation. Build caches were
reused throughout; no cold-build or performance speedup claim is made.

Installed processes used isolated Python (`-I`), a minimal environment, and a
qualification OS sandbox denying SDK/source reads and compiler execution.
Scheme 023 and both modes of 024 additionally denied all Homebrew Cellar and
opt paths. Required external libraries were in the repaired wheel. These
tests never supplied SDK variables or compilation commands to the consumer.
Process receipts include actual exit, timeout and process-group quiescence.

Rust ledger tests execute the Scheme-authored, build-time typed fold as Rust,
including big integers, independent mutable states, result invalidation and
explicit rejection of unsupported dynamic DSL. Rust-only wheels have no
Scheme runtime crate in their normal dependency graph. In combined wheel 024,
the actual Scheme initialization-attempt flag remained false before/after
every Rust test. This is stronger than the uncompiled-provider `None` case.

ASCENT tests execute real recursive relational rules, source withdrawal,
retained snapshots, concurrent independent Rust sessions and unsupported-input
rejection. This adapter supports its existing typed relational subset, not
general Scheme macros or the complete Evidence DSL. Rust sessions are not
Scheme VMs.

Scheme tests exercise exact values, complete resource revalidation, GC/shared
and cyclic roots, closures, collections, error recovery, released references,
thread affinity, fork refusal and final teardown. The fork negative control's
Python deprecation warning is retained. Second-VM refusal is a negative
boundary test, not independent-VM acceptance.

## Wheel SHA256

- Ledger 020: `845bbbea17da91891fee9e1dd4368aadc006d1d19451f5828a904c86c872fe2f`
- ASCENT 021: `99b9260bfe2006794b2c28b01ef119c909d78070ede472c719cdea48b4545933`
- Ledger 023: `9264ab76e4e7eaf9b1ea1cd5b93cc1d459ffb1fc5c29134fbef8663fbcaef7fd`
- Ledger 024: `cb4458a04ab0a3ddcd5643ab0b18c91f5571c720464a7f04ae491446c8febf15`

## Remaining Acceptance Failures

1. Independent Scheme VM creation, module initialization, roots, parallel entry
   and independent teardown are not implemented. The current Rust/C bridge
   still owns one process-global runtime. The linked SDK's
   `___SINGLE_THREADED_VMS` branch selects VM0 and makes the real-pstate setter
   a no-op. Removing the singleton, creating more owner threads, or recompiling
   only a bridge header would not fix that coherent ABI requirement.
2. The prior 15-second runtime-loaded hygienic DSL timeout has no causal fix.
   It was not rerun. The 13 Scheme tests explicitly exclude that requirement;
   they are not a full-suite result or an implicit waiver.
3. The ASCENT qualification example has no Scheme provider. Its Rust acceptance
   does not qualify ASCENT's full Scheme DSL or Python product API.

No process pool, serialized VM0 binding or Rust-session count is substituted
for parallel Scheme VMs. No timeout was raised and no failed receipt changed.

## Local Evidence

Raw observations are in the consumer checkout's ignored
`.data/upstream-python-015` directory: `ledger-build-020.json`,
`ascent-build-021.json`, `ledger-scheme-build-022.json`,
`ledger-scheme-build-023.json`, `ledger-dual-build-024.json`,
`ledger-installed-020.json`, `ascent-installed-021.json`,
`ledger-scheme-installed-023.json`, `ledger-dual-rust-installed-024.json`,
`ledger-dual-scheme-installed-024.json`, and `source-inventory-020.json`.
These local files are not claimed as a durable Git evidence archive.
The failed ASCENT 020 PATH attempt and all older failures remain retained.
