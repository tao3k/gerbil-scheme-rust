# Separate c60794c CI follow-up

The SCM receipt failure is no longer the Ubuntu Cargo failure. Its Actor module
compilation now fails at the unguarded CLOCK_UPTIME_RAW use and mach/mach.h
include. macOS bindings, Ubuntu Bazel, Actor concurrency qualification and both
model shards have remote success receipts. The cancelled fixed-owner lifecycle
job is not a pass; remaining jobs retain their current remote status.

The local repair selects POSIX CLOCK_MONOTONIC outside Darwin. The Mach observer
remains Darwin-only: unsupported start/stop return ENOTSUP and its 35 field reads
expand to UINT64_MAX. No borrowed progress address is retained there and no fake
zero-valued observation is admitted. Inactive completion/wait observation hooks
have no observation to update; real Scheme execution is not disabled.

The actual embedded C's unsupported path compiles and executes in a focused
Rust test, with strict Clippy and formatting passing. This host simulation is
not a Linux SDK build receipt. On Darwin, preprocessing both actual C blocks
and all 35 original/repaired reading expressions gives identical bytes, SHA-256
4d0ec878c110520183025b6f1222306b121d9b9134120510ecf377804c08d889.
This is narrower than whole-module native binary identity; the original frozen
binary/image are independently preserved without rebuilding or running them.

This local build repair is separate from the rejected scheduling prototype.
No compiler, SDK, workload, deadline or default is changed. The repair is submitted separately for remote Linux admission. It contains no
scheduling candidate or new performance result. Remote CI must finish before
this repair is described as fully admitted.
