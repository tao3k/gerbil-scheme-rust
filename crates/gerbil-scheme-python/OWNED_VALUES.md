# Owned Transport Values

`OwnedValue` is an owned host transport type. It is not a Scheme heap pointer,
evaluator or execution authority. The native Scheme bridge projects rooted
values into this type when an independent host copy is required. Copying host
data does not execute Scheme; Scheme application calls retain their owner VM.

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

Current runtime qualification uses the complete native Scheme ledger path; see
[wheel qualification](WHEEL_QUALIFICATION.md). Historical subset transport
experiments do not qualify complete Scheme execution. Raw diagnostic artifacts
are kept outside the repository.
