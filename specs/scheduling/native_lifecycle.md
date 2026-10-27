# Native initialization and qualification resource ownership

The original full parallel `scheme_service` test process reproducibly exited70
with the unchanged five-second progress watchdog during initialization. Same-run
stacks showed startup callers waiting for owner receipts, owners waiting for the
process lifecycle mutex, and the admitted owner loading fresh Mach-O images.
This was cold initialization evidence, not a hot Actor execution diagnosis.

A rejected narrowing experiment moved dyld admission outside the SDK gate.
Startup-only phase receipts then showed one owner entering SDK setup near0.50s
and completing near2.62s while other owners repeatedly entered dyld admission.
The experiment still failed. SDK/TLS setup itself can consult loader state;
uncoordinated loader admission therefore did not close the wait. That candidate
was withdrawn rather than reported as a lifecycle improvement.

The retained library boundary coordinates image loading/symbol admission with
SDK setup/rollback and SDK cleanup. Application finalizers, root release, file
retirement and mapping retirement do not hold this process-wide gate. Foreign
application callbacks must not retain a gate needed by another owner's setup.
`GERBIL_INSTANCE_LIFECYCLE_TRACE` optionally records startup-only committed phases;
it is off by default and adds no request-path clocks or synthetic heartbeats.

## Qualification fixture contract

The previous test binary launched independent cold-start/failure-injection cases
and process-wide `/dev/fd` assertions concurrently without a resource fixture.
Their watchdogs started before they owned those shared process resources. This
made their independent baseline assumptions incompatible. The test fixture now
owns a `native_case` lease before starting each native case, including its
watchdog and FD snapshot. The ordinary Cargo test thread count is unchanged.
Non-native tests remain parallel. Within each native case, multiple fixed VM
owners, concurrent Rust clients, budget replacement and cancellation still run
concurrently with the original five-second gates.

This is test resource isolation, not a production promise that arbitrarily many
independent cold pools initialize simultaneously within five seconds. No VM
startup timeout or hot service default changed, and no test was dropped. The
original full16-case command passed in45.53s with the fixture. Old failures,
phase receipts and the rejected narrowing result remain local ignored evidence.

`native_lifecycle.qnt` reuses the owner lifecycle, adding preparation, native
setup/rollback, callback quiescence and cleanup ownership. Preparation represents
root-free file preparation; the native transaction includes dyld admission.
The callback-lock negative mutation violates the exclusion invariant. All79
inherited/directed tests and1,000 bounded traces passed after using the existing
three-job owner instance. The initial two-job instance was invalid for inherited
three-job tests; those failures were retained and corrected by instantiating the
existing model correctly, not deleting tests. Safety exploration is not a bound
on kernel loader latency.

The resource-fixture extension separately checks watchdog admission after the
case lease and clean case reuse: three focused cases and1,000 bounded traces
passed. After adding typed-construction qualification, the full default-thread
suite passed17 cases in63.34s. This result retains native concurrency within a
case; it does not mean independent native cases execute concurrently.
