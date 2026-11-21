# Runtime VM Qualification

The two domains are intentionally separate:

- `runtime-smp-test.ss` checks several active processors inside one VM.
- `runtime-vm-test.ss` creates two live VMs inside one runtime, each on a
  fixed OS owner with its own processor 0. It requires simultaneous Scheme
  entry, separate module globals and strong roots through GC, and a surviving
  VM call after the other VM has been destroyed.

The second test uses the existing linked runtime's VM constructor, processor
selector, kernel program entry and cleanup APIs. It does not replace Gambit
macros, construct private runtime images, serialize Scheme execution, or
implement a second loader. Its C support header is test-only. Its thread-local
role distinguishes test initialization from the main test invocation; it is
not a substitute VM selector.

Build the bridge modules through the existing Library build script, then
compile the standalone test with the support header's directory on the C
include path. These are maintainer test builds, not a downstream SDK workflow:

```sh
gxi build.ss
gxc -exe -o /tmp/runtime-vm-test \
  -cc-options "-I$PWD/scheme -O0" scheme/runtime-vm-test.ss
```

Use the matching selected SDK and its existing library dependencies. A missing
dependency or ABI mismatch is a failure, not permission to mix runtimes.
The qualification process has a whole 15-second execution deadline, including
owner joins and teardown. The test's three-second rendezvous checks do not
extend that deadline. Never infer progress from a sleeping owner.

Both owners finish VM construction before either can destroy its VM. A
selector readback mismatch prevents entry into Scheme and remains a failed
qualification; it is not an accepted negative control for multi-VM support.
Partial constructor failure terminates the child without inventing rollback
ownership. Future failure-injection and rollback coverage remains separate.

## Observed Run 040

Against installed SDK `1cfb032`, the standalone test was actually compiled and
run. Both owners reported `created=1`, different VM addresses and
`processors=1`, but `selected=0`, `outcome=13`. The test exited 70 in
0.706068542 seconds, without timeout; the process group was absent afterwards.
Parallel Scheme execution, root isolation and surviving-VM calls were not
reached and are not qualified. Python shared-runtime multi-VM and ASCENT
Scheme acceptance remain open.

The preceding link failure caused by an obsolete configured OpenSSL path is
retained. The successful test build used the existing installed OpenSSL
library search path; it did not rebuild or edit Gambit. Library build and both
test compilation attempts consumed 12.432574959 seconds in total, within the
original 120-second build ceiling. Raw process observations are in the
consumer's local `.data/upstream-python-015/vm-test-*-039.json` and
`vm-test-*-040.json`; they are not an archived release or complete wheel receipt.
