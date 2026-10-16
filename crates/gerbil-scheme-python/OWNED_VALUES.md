# Owned Transport Values

`OwnedValue` is a VM-independent transport type shared by Rust execution
libraries. It is not a Scheme heap pointer, evaluator, or execution authority.
The optional Scheme bridge projects its existing rooted scalar values into this
type; Rust-only providers can use it without linking or entering a Scheme VM.

It preserves arbitrary-precision integers, distinct booleans, binary64 values,
UTF-8 text, bytes, lists and string-keyed maps. Python tuples normalize to lists.
`ValueLimits` bounds decoded depth and node occurrences. Shared containers are
copied; cyclic graphs and arbitrary Python objects require another explicit
handle protocol and are not represented by this tree type. Provider-specific
opaque references retain their own owner and generation validation.

`positive_usize` validates optional caller ceilings without accepting booleans,
zero, negative integers or values exceeding the host word. These checks do not
introduce a finite default for a library's fact or request policy.

`ResourceTree::file` and `stage_source_inventory` publish complete per-path
source inventories through the existing checked package publication mechanism.
They do not compile or interpret sources. Missing, aliased, extra, changed and
unowned files remain errors. Installed verification does not consult original
build-machine paths.

Local qualification `ascent-build-048-bridge005` passed one ASP gate and 27
integration tests in 3.771 seconds under the unchanged 120-second process cap,
with no skipped tests, timeout or surviving process group. This is bridge-layer
qualification, not complete ASCENT DSL, consumer, multi-VM or performance
qualification. Historical failed overlap validation remains a separate receipt.
