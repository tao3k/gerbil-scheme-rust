# Owned C ABI

## Scope

This is a transport and lifecycle contract for **existing, independently compiled
algorithm exports**. It is not an interpreter, algorithm registry or new backend
for arbitrary Scheme. Rust does not implement the producer's closure body or
component transition. The producer supplies actual C-compatible function pointers.

Use `gerbil-scheme-python` with `default-features = false, features = ["c-abi"]`.
Add `rust-backend` only when Python conversion is required. `owned-values` contains
the shared data types without Python. Neither combination enables `gerbil-scheme`
or `gerbil-scheme-sys`. The feature name does not prove an external producer is
VM-independent; exact producer source and artifact admission remain necessary.

## Ownership

The public header is `include/gerbil_scheme_owned.h`:

- `GerbilOwnedView` borrows finite acyclic data for one call: null, boolean,
  exact arbitrary integer, binary64, UTF-8 text (including NUL), bytes, list or
  text-keyed map. Sharing is allowed; projection is a tree copy, not graph identity.
- `GerbilOwnedValue` transfers one immutable result allocation and release entry.
  Its spans remain valid independently of the callable or component lifetime.
- `GerbilOwnedCallable` transfers captures and a compiled call entry. The producer,
  not Rust, executes the closure. Factory errors may transfer partial ownership,
  which is released even when ABI admission fails.
- `GerbilOwnedState` transfers snapshot and transactional replacement entries.
  Replacement returns the old independent result on success and preserves state
  on failure, including failure after allocating partial output.

Rust's `ForeignValue`, `ForeignCallable` and `ForeignState` consume these owners.
Closing one callable/state alias invalidates all aliases; independent owners are
isolated. Owners cannot be sent/shared across Rust threads. Independent owners may
be created and used on independent host threads if the producer supports that.
This is not a Scheme multi-VM claim.

The unsafe admission boundary requires valid foreign memory and function pointers,
no unwinding/longjmp across C, no VM entry, and executable code retained until the
last result/context release. A dynamic-library handle must outlive *all* callbacks
and values. Release remains ABI-compatible on rejected versions. An untrusted
pointer or lying `NO_VM` declaration is not made safe by bounds validation.

Unsupported result projection reports an error while retaining the actual foreign
result until its owner drops. It does not rerun execution, fall back to another
backend, or claim support for arbitrary opaque/cyclic graph arguments. Structural
projection bounds are independent of application fact/request capacity policies.

## Official FFI Boundary

[Gerbil's FFI guide](https://gerbil.scheme.org/guide/ffi.html) supports embedding C
and typed foreign calls. [Gambit's C interface](https://gambitscheme.org/4.2.6/manual/)
distinguishes C/Scheme values and generated conversions. A Scheme procedure
exported through `c-define` is not thereby independent of Scheme objects, module
initialization, roots or garbage collection. Source-embedded pure C kernels can
qualify separately, but the exact artifact must actually expose that independent
entry. Do not bypass initialization of a rooted export or rewrite its algorithm
in Rust merely to call the resulting host VM-independent.

## Local Qualification 053

The test producer `tests/fixtures/owned_exports.c` is compiled as C11 with
`-Wall -Wextra -Werror` and loaded through the platform's standard shared-library
loader. It implements capture/state controls only; it is **not ASCENT's DSL**.
Controls cover ABI layouts, integers/bytes/nested data, captured values, independent
state, transactional failure, partial-result cleanup, rejected ABI/VM declarations,
alias closure, projection failure with retained ownership, independent parallel
host owners and complete producer allocation teardown.

The Python/Rust + C ABI integration target passed 32 tests, zero ignored, in
13.377 seconds including compilation, within the original 120-second cap.
Raw receipt: `/private/tmp/ascent-c-abi-053-python001.json`.
Earlier compile and lint failures remain in the other `ascent-c-abi-053-*` receipts.
These are local observations, not installed-wheel, ASCENT consumer, sealed review,
current-graph, performance or published-Git qualification.

After fixing the real ASP findings (public documentation, co-located ABI
identity/header constants and nested conversion traversal), the entire bridge
crate passed: ASP gate plus 32 integration tests, zero ignored, 24.596 seconds
within the same 120-second cap. Final Clippy with warnings denied passed in
0.479 seconds. Receipts are `ascent-c-abi-053-crate002.json` and
`ascent-c-abi-053-clippy005.json`; the first failed whole-crate receipt is retained.
The consumer archive is `evidence/library/c-abi-053/`, with an explicit partial
source inventory and no wheel/consumer/publication admission.

An existing ASCENT rich-DSL entry still uses `___setup`. This ABI does not remove
that requirement. Actual VM-independent algorithm exports and installed consumer
acceptance remain open; neither a C fixture nor typed Rust IR closes that gap.
