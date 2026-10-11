# Scheme/Rust conversion audit

## Frozen bridge baseline (2026-10-11)

The accepted baseline is the filled-capacity, bounded bulk encoder after the
conservative rollback described below. `just bridge-benchmark
'buffers::utf8_fresh'` completed twenty samples per changing-text load:
1,000 transfers at 16.24 ms median, 10,000 at 167.4 ms, and 100,000 at 1.782 s.
Every transfer includes fresh Scheme encoding, independent rooting, owned bulk
copy, complete UTF-8 validation and exact content checks. These are recorded
local bridge medians, not portable timing guarantees or new acceptance limits.
The Org parsing sub-100-ms gate remains unchanged and is not qualified here.
Rejected optimization measurements remain in this report. Further changes
must use same-condition controls and reject slower candidates, not redefine
the benchmark or substitute cached snapshots. Delivery uses the existing
gerbil-scheme-rust PR #18, without a new PR or release tag.

This inventory distinguishes working conversions from opaque handle shapes.
A nonzero handle is not evidence of runtime classification, rooting, traversal,
or a bulk conversion. Scalar element access is legitimate; whole-value APIs
must not implement traversal by repeated Rust-to-Scheme calls.

| Scheme family | Rust representation | Current implementation | Boundary / gap |
| --- | --- | --- | --- |
| boolean, fixnum, character, flonum | `bool`, `isize`, `char`, `f64` | Checked scalar projection | Constant-size values; not bulk traversal |
| exact integer | rooted integer; checked `i64`/`u64` | Range-checked projection and byte encoding | Arbitrary-precision Rust result is not implemented |
| u8vector | borrowed input `&[u8]`, owned/reused `Vec<u8>`, reused output `&mut [u8]` | Binary construction and bulk copying | Copies ownership; no exported Scheme heap slice |
| string | borrowed Rust `&str`, owned `String`, explicit rooted UTF-8 snapshot | Scheme UTF-8 encoding and bulk bytevector copy | Snapshot owns an independent root; not zero-copy or an implicit mutable-string cache |
| pair | `SchemePair`, checked car/cdr projections | Runtime-classified borrowed access | Recursive owned Rust conversion is not implemented |
| proper list | `SchemeList` | Classified borrowed identity | No bulk list-to-`Vec<T>` conversion |
| vector | `SchemeBorrowedVector` | Rust handle slice with pointer/length ABI | No Scheme-vector classification, rooting, or bulk output conversion |
| symbol, keyword | `SchemeSymbol`, `SchemeKeyword` | Opaque handle shapes | No runtime-backed name/interning conversion |
| nil, void | typed sentinel views | Checked predicates/projections | Not collections |
| s8/u16/s16/u32/s32/u64/s64/f32/f64 vectors | no typed owned bridge representation | Not implemented | Need element-width, range, alignment, endian, GC and copy contracts |
| rational, complex numbers | no owned bridge representation | Not implemented | Need lossless numerical representation and checked error policy |
| hash table, struct/object, procedure, port | no owned bridge conversion | Not implemented | Need explicit identity/ownership or serialization contracts; not raw pointer casts |

Missing families must not be advertised as supported by reusing an unclassified
word or passing an unrooted `Vec<GerbilValueHandle>` across a GC boundary. Numeric
vectors need typed ABI contracts; generic collections need explicit element-root
ownership. These are distinct from copying a homogeneous bytevector.

## Regression gates

`just bridge-contracts` checks Rust syntax trees for scalar loops/calls in
whole-bytevector and whole-string conversion methods, including deliberate
mutation controls. Shared buffer-copy tests cover 0 through 1,000,000 bytes and
at most one bulk callback invocation (none for a validated empty value).
Runtime tests cover binary data, Unicode, embedded
NUL, non-aligned lengths, wrong types, invalid UTF-8 and untouched error outputs.
These are targeted regression checks, not a proof of arbitrary helper code.

The installed official `gerbil/runtime/util.ss` routes UTF-8 `string->bytes` and
`bytes->string` through `string->utf8` and `utf8->string`. The original bridge
used those primitives. The single-pass Scheme encoding refinement below is
bridge-owned, based on official encoding rules; decoding remains unchanged.
No Unicode encoder was introduced in Rust or C.

## Workload-specific reuse

`SchemeBytevector::copy_to_vec` and `RootedSchemeBytevector::copy_to_vec` replace
a caller-owned `Vec<u8>` without allocating when its capacity is sufficient.
The output length may grow, shrink or become zero. They initialize spare
capacity directly through the existing checked bulk ABI, without a zero-fill
pass. Length-query failure preserves the previous output; copy failure leaves
an empty vector and never exposes partially initialized bytes. They retain
owner affinity and do not return pointers into the Scheme heap.

`RootedSchemeString::to_utf8_bytes` creates an independent rooted UTF-8
snapshot through the bridge's Scheme encoder. The source root may be dropped;
the snapshot stays valid until its own release or runtime destruction. Explicit
snapshot reuse is suitable when the same text is transferred repeatedly. For
changing text, use fresh encoding instead. No implicit memoization assumes
that Scheme strings are immutable, and no automatic policy hardcodes a job
count or machine size.

`just bridge-benchmark buffers` compares owned allocation and capacity reuse
with identical varying bytevector lengths (0, 1, half and full payload), at
1,000 / 10,000 / 100,000 transfers and 64 / 8,192-byte maximum payloads. Separate
Unicode/NUL controls compare fresh encoding with an explicitly pre-encoded
snapshot; both produce validated Rust strings. Each case uses 20 samples.
Initialization and snapshot construction are excluded from repeated-transfer
timing, not claimed as free. These are actual single-owner FFI conversions,
not parser throughput, independent-VM parallelism or queue-admission timing.
The source regression gate covers both reusable output methods and the
snapshot encoder, with deliberate scalar-FFI mutations rejected. Helper tests
also verify pointer/capacity retention, growth and a partial-write failure.

### Local diagnostic receipt, 2026-10-10

All twelve variable-length bytevector cases completed 20 samples. Reusable
Unicode snapshots completed 20 samples at each of the three request totals.
These measurements ran on a busy host, partly alongside compilation and other
qualification; they are not matched quiet-host speedup admission.

Bytevector transfer medians (milliseconds, maximum payload per varying cycle):

| Transfers | 64 bytes, allocated | 64 bytes, reused | 8,192 bytes, allocated | 8,192 bytes, reused |
|---:|---:|---:|---:|---:|
| 1,000 | 0.3091 | 0.1707 | 0.2234 | 0.1755 |
| 10,000 | 6.797 | 2.920 | 18.50 | 4.650 |
| 100,000 | 224.9 | 53.17 | 226.3 | 181.7 |

For example, the allocated 64-byte 100,000-transfer range was 39.29 to 492.3 ms;
the reused 8,192-byte range was 67.26 to 378.2 ms. Capacity retention is checked
directly; wall-clock ratios are not promoted to stable performance claims.

The fresh-encoding 20-sample run completed 1,000 and 10,000 transfers, then was
interrupted while entering the 100,000-transfer case (exit 130). This is an
incomplete receipt, not an admitted long-run result. Genuine 1,024-transfer
progress reporting was added, and a separate **one-sample diagnostic** completed
all three fresh-encoding loads without changing encoding or payload semantics:

| Transfers | Fresh encoding, one sample (ms) | Explicit snapshot, 20-sample median (ms) |
|---:|---:|---:|
| 1,000 | 202.5 | 23.80 |
| 10,000 | 1,222 | 462.7 |
| 100,000 | 15,370 | 3,980 |

These columns have different sampling and construction costs, and are not a
paired ratio. Each result is a validated Rust string containing 8,196 UTF-8
bytes, including NUL and multi-byte characters. The 100,000-transfer snapshot
range was 1.329 to 6.197 seconds. The repeated fresh-encoding hotspot remains;
snapshot reuse avoids re-encoding only when the caller explicitly wants stable
text. It does not optimize a workload of new or mutated strings.

The matching official Gambit `gambit/string/string.scm` encoder computes UTF-8
length before filling its bytevector. The official `c_intf.c` C string converter
rejects embedded NUL; substituting it would violate this bridge's contract.
Neither SDK source nor the kernel was changed, and no bespoke Unicode codec
was added. The remaining numeric-vector and general-collection gaps in the
inventory above are unchanged, not advertised as completed conversions.

`bridge-contracts` and workspace Clippy passed after these API changes. The
contracts included five buffer/root helper tests, 37 safe API/source checks,
the live binary/Unicode lifetime controls and a clean 444,031-request handoff
drain. Existing ignored build diagnostics were unchanged. These conversion
receipts do not admit independent VMs, Scheme SMP or the Org parser's 100 ms
threshold.

After the final source fixes, `just bridge-benchmark buffers --test` completed
all 18 bytevector/Unicode scenarios with exit zero, including 100,000 fresh
encodings. Smoke completion is correctness evidence, not a replacement for the
incomplete 20-sample fresh-encoding timing run. Formatting, diff and filename
checks also passed.

## Fresh changing-text encoding refinement

`scheme/utf8.ss` is a standard module imported by the production FFI module. It follows Gambit's
`gambit/string/string.scm` codepoint emission rules and Gerbil's
`std/encoding/utf16.ss` maximum-capacity / shrink pattern. A checked capacity
of four bytes per character permits one character traversal instead of a
length traversal followed by an encoding traversal. Fresh calls still create
and release independent encoding roots; this is not snapshot memoization.
At that stage, UTF-8 decoding and Rust's checked `String::from_utf8` validation
were unchanged. The subsequent SIMD validation refinement below preserves the
same acceptance and error contract while replacing the validation algorithm.

Allocation and GC publication stay outside the bounded encoding leaf. The
leaf encodes at most 256 characters / 1,024 output bytes per call with a local
`not interrupts-enabled` declaration, following official
`gerbil/runtime/interface.ss` primitive-region practice. The outer loop retains
normal scheduling polls. Valid-input leaf operations allocate nothing; invalid
scalars raise the existing contained conversion error. No global interrupt,
VM, SDK or kernel setting was changed. The producer must not concurrently
mutate a transferred string during foreign conversion; this change does not
admit shared mutable roots or an unqualified SMP executor.

The capacity tradeoff is explicit: temporary allocation is at most four times
the character count (four times exact bytes for ASCII, approximately 1.67 times
for this mixed benchmark). In-place shrinking returns the exact byte length;
no claim of zero-copy or globally optimal memory use follows.

`scheme/runtime-test.ss` compares all 1,112,064 Unicode scalars against the
installed official encoder in bounded blocks, plus empty/ASCII/NUL strings and
surrogate rejection fixtures. The public character constructor already rejects
surrogates; only negative test fixtures use the raw constructor. The
encoder module is tracked by both generated-SCM provenance and Cargo rerun
inputs, with a changed-encoder fingerprint regression. Both runtime and encoder
SCM modules are generated, linked and checked. Generated SCM was
regenerated and checked without changing the installed SDK.

The fresh benchmark now alternates four distinct 8,196-byte texts and verifies
every resulting Rust string, not just output lengths. Encoding, root handoff,
Rust allocation/copy/validation, exact comparison and real progress reporting
all remain inside timed batches. Snapshot construction is not substituted.

Before the bounded-leaf refinement, two complete 20-sample changing-text runs
finished with these medians (milliseconds):

| Transfers | First single-pass run | Repeated single-pass run |
|---:|---:|---:|
| 1,000 | 22.60 | 54.71 |
| 10,000 | 907.2 | 967.1 |
| 100,000 | 8,693 | 8,878 |

The first 100,000-transfer range was 4.258 to 10.99 seconds; the repeat range
was 6.253 to 16.14 seconds. These noisy negative observations are retained,
not promoted to a stable speedup against earlier differently sampled controls.
Generated C showed inlined fixnum/bytevector operations but a poll on each
character, motivating the subsequent bounded-leaf measurement below.

The first attempted bounded-leaf run was rejected and interrupted (exit 130):
the Gerbil incremental build did not invalidate an included source file, so its
generated C still contained the earlier per-character polls. Source fingerprints
alone did not establish actual compiled implementation identity. The encoder
was moved from an include to a standard module in the canonical build graph;
the standalone Gambit archive now compiles and links both modules. The foreign
body uses the fully qualified encoder binding, not an unresolved unqualified
name. No timestamp touch or installation/kernel workaround was used.

The correctly linked bounded-leaf implementation completed all three loads,
with 20 samples and one full batch per sample (2,220,000 fresh transfers total):

| Transfers per sample | Median | Observed sample range |
|---:|---:|---:|
| 1,000 | 123.5 ms | 13.47–379.8 ms |
| 10,000 | 820.7 ms | 280.1–1,409 ms |
| 100,000 | 6.892 s | 5.244–10.82 s |

The process exited successfully after the twentieth 100,000-transfer sample.
The generated C contains scheduling polls in the outer encoder, but none in
the bounded chunk function. This verifies the actual compiled algorithm, not
just source provenance. All samples use fresh encoding of alternating texts;
none use the stable-text snapshot path.

The large timing variance and mixed results across loads do not establish a
stable end-to-end speedup or close the performance gate. These are synchronous
value-conversion measurements on one runtime owner, not parallel parsing or
an Org document benchmark. The parser's sub-100-ms acceptance criterion is
unchanged. No kernel modifications, reduced sample counts or relaxed limits
were used to admit this result.

Final local qualification completed: the Scheme Unicode/root-lifecycle
conformance suite, Rust bridge contracts, generated-module provenance checks,
workspace Clippy with warnings denied, formatting, diff whitespace and filename
checks passed. This is local evidence only; no new remote CI or downstream
parser performance qualification is claimed.

### Negative-result follow-up (2026-10-11)

The earlier fresh-conversion negative results remain unresolved performance
evidence, not passing admission. They included a stderr write every 1,024
transfers inside the timed loop (97 writes per 100,000-transfer sample).
That is a measurement confound, not evidence that logging caused the entire
regression. The loop logging was removed; one actual sample-completion event
remains. An AST regression gate rejects logging inside for/while/loop bodies,
including nested conditional logging, while allowing sample completion logs.

Four diagnostic phase controls completed all three loads with 20 samples each.
Every fixture alternates the same four 8,196-byte texts. Pre-encoded bytevectors
are used only in the copy control, never in a fresh-conversion path.

| Transfers | Encode/root/release | Owned copy | UTF-8 validation/comparison | Fresh encoding with reused Rust output |
|---:|---:|---:|---:|---:|
| 1,000 | 40.72 ms | 0.6013 ms | 11.09 ms | 35.90 ms |
| 10,000 | 167.9 ms | 16.73 ms | 108.6 ms | 373.5 ms |
| 100,000 | 2.253 s | 419.5 ms | 977.6 ms | 2.756 s |

These medians are not additive: each phase ran separately under varying host
load. The encode control includes root publication, length query and release;
it does not claim to isolate pure encoding. A three-second stack sample during
the 100,000-transfer encode control found 1,867 top-of-stack observations in
the generated UTF-8 module out of 2,573 main-thread observations. This supports
targeting the encoding loop rather than attributing the hotspot to allocation
alone. That sampled run is diagnostic, not performance admission. The local
stack receipt is `/private/tmp/bridge-utf8-phase-sample-20261011.txt`.

The next loop refinement replaces two surrogate-range comparisons with one
aligned-block test (`codepoint >> 11 == 0x1b`). Surrogates are rejected within
the BMP branch, so the following four-byte branch no longer needs a redundant
lower-bound comparison. All 1,112,064 Unicode scalars again match the installed
official codec; surrogate rejection and rooted-value GC/release tests pass.
No validation, polling boundary, ABI or kernel contract is relaxed.

The first reduced-range variant completed 20 samples at each load: medians
28.88 ms / 266.0 ms / 2.863 s, with ranges 18.34–54.82 ms /
172.8–340.0 ms / 2.083–3.874 s. Its generated C exposed another code-quality
issue: a non-tail `when` error check retained a return continuation and stack
adjustment on the valid BMP path. The rejection was rewritten as the tail
branch of an `if`; validation semantics are unchanged. This intermediate
result is retained, not used as a claim of stable algorithmic speedup against
the earlier noisier runs.

Conformance now also rejects all 2,048 surrogate values and mutates a single
Scheme string in place across UTF-8 width boundaries. Each new encoding is
compared to the official codec, and the original encoded bytevector must stay
unchanged. This specifically guards against stale snapshots masking changing
input, rather than testing only a rotation of immutable roots.

The final tail-rejection version completed the full fresh-conversion matrix,
without a profiler attached and without batch-internal progress writes:

| Transfers per sample | Median | Observed sample range | Samples |
|---:|---:|---:|---:|
| 1,000 | 16.92 ms | 12.72–51.88 ms | 20 |
| 10,000 | 243.5 ms | 148.1–300.8 ms | 20 |
| 100,000 | 2.515 s | 1.855–3.317 s | 20 |

All 2,220,000 fresh transfers completed with exact content checks and exit zero.
Generated release C confirms the surrogate-block comparison and tail jumps to
the error procedure; the chunk has no return continuation for the rejection.
Compiler temporary stack operations still exist, so this is not a claim that
the loop has no stack traffic. The outer scheduling polls remain intact.

The new observations are lower than the earlier negative record, but changes
in logging and host load prevent assigning the entire difference to the
algorithm refinement. No stable speedup, parallel parser execution or parser
sub-100-ms qualification is claimed. Each transfer in this conversion fixture
is 8,196 bytes; the 100,000-transfer sample moves 819.6 MB of output, not
100,000 independently parsed Org documents. The old negative result is recalled
as an acceptance baseline, not erased as an observation or relabeled a pass.

One expanded conformance attempt failed because an unnecessary SRFI import
was not available in the selected SDK module surface. The fixture now exhausts
the surrogate range with an ordinary named loop and no added module dependency;
the complete conformance rerun passes, including in-place mutation and GC.

Final local checks pass: 39 safe-surface gates (including the new loop-I/O
negative controls), real FFI Unicode/root conversions, 444,031 handoff releases
with clean shutdown, 21 AOT unit checks and 20 AOT contract checks, generated
SCM regeneration/provenance, workspace Clippy with warnings denied, formatting,
diff whitespace and filename checks. Existing separately supervised/GCC-only
ignored tests remain outside this focused qualification (one runtime latency
control and five AOT tests); no new skip was introduced. No commit, push,
exact-head remote CI or downstream dependency admission was performed here.

### Bounded bulk encoding and full SIMD validation

The production fresh-string path now addresses both measured hot phases.
`scheme/utf8.ss` owns allocation, overflow admission, bounded scheduling polls,
shrinking and errors. Its private C leaf processes at most 256 characters per
call using the official Gambit `___STRINGSIZE`, `___U8VECTORSIZE` and typed
`___BODY_AS` interfaces. No allocation, callback, poll or retained heap pointer
occurs inside the leaf. Rust does not implement the Scheme encoder, and no
kernel source is changed. Output remains an independently owned byte copy;
this is not a zero-copy claim across a possible Scheme GC boundary.

The algorithm follows the installed official `gambit/string/string.scm`
Unicode rules and `lib/c_intf.c` bitwise encoding, while rejecting surrogates
and code points beyond U+10FFFF. The older internal C codec is not called:
its extended five/six-byte representation and noncharacter policy are not the
required Rust UTF-8 contract. Before writing, the leaf rejects wrong types,
invalid ranges, chunks over 256 characters and insufficient destination
capacity. Scheme retains the single allocation and final shrink; there is no
preliminary UTF-8 length pass or stable-text cache.

The owned Rust conversion validates the entire fresh byte buffer through
`simdutf8::basic::from_utf8` before transferring that same allocation into
`String`. Only after successful validation is `String::from_utf8_unchecked`
used, avoiding a second validation scan, not avoiding validation. Invalid
input retains the existing operation name and `InvalidValue` status. The
library supplies architecture dispatch and a standard-library fallback;
the installed AArch64 target enables NEON without custom target flags.
See the [official simdutf8 implementation](https://github.com/rusticstuff/simdutf8)
and [Rust's ownership-conversion safety contract](https://doc.rust-lang.org/std/string/struct.String.html#method.from_utf8_unchecked).

Conformance covers all 1,112,064 Unicode scalar values against the installed
official codec, rejection of all 2,048 surrogates, mutation of one Scheme
string across encoding widths, and independence of previous outputs. Invalid
C leaf bounds/types are rejected before touching a sentinel destination.
Rust compares all 256 one-byte and 65,536 two-byte inputs, malformed sequences
across SIMD block boundaries, and a complete scalar corpus with the standard
validator. An allocation-identity assertion prevents a hidden extra copy.
An AST negative control rejects bypassing validation before the unchecked
ownership transfer; existing gates reject per-element foreign calls.

The first C compilation exposed the official subtype predicates' requirement
for a `___temp` scratch word. Adding that local declaration fixed the leaf;
the complete Scheme conformance rerun passed. A preliminary raw Cargo check
also failed to locate Darwin `iconv` because it bypassed the canonical Just
compiler selection. Subsequent builds use the existing Just recipes and
system compiler; no SDK reinstall or linker/kernel workaround was introduced.

The canonical Just recipes completed the following matrices with exit zero,
20 samples and one batch per sample at every load. Fresh conversion rotates
four different 8,196-byte texts, reencodes on every transfer and checks exact
content. All 2,220,000 fresh transfers completed; no snapshot is substituted.

| Transfers per sample | Fresh production median | Observed range | Samples |
|---:|---:|---:|---:|
| 1,000 | 16.42 ms | 7.858–73.57 ms | 20 |
| 10,000 | 190.1 ms | 124.9–274.9 ms | 20 |
| 100,000 | 1.892 s | 1.428–2.843 s | 20 |

Separate phase controls use identical changing text, retain correctness
assertions and complete all 20 samples per case. Encoding includes rooting
and release; validation includes an exact comparison, not an unvalidated
byte-count surrogate.

| Transfers per sample | Encoding/root median (range) | Standard validation median (range) | SIMD validation median (range) |
|---:|---:|---:|---:|
| 1,000 | 13.68 ms (5.645–55.97 ms) | 10.68 ms (8.163–46.95 ms) | 2.045 ms (0.9602–12.25 ms) |
| 10,000 | 211.7 ms (95.89–607.9 ms) | 92.3 ms (71.02–199.8 ms) | 35.60 ms (23.31–117.5 ms) |
| 100,000 | 2.113 s (1.113–3.358 s) | 1.717 s (1.016–2.774 s) | 324.1 ms (163.5–775.0 ms) |

The release benchmark executable contains the actual AArch64 NEON validator
symbol, and its generated `utf8.c` contains the bounded C encoding loop. These
are production-path changes, not only benchmark changes. The ordered phase
runs are input-matched diagnostic controls, not an interleaved quiet-host
A/B qualification. Host-load variance is visible: encoding alone has a higher
observed median than fresh conversion at the larger loads. Do not subtract
these medians, add phases, or claim a stable acceleration factor from them.
Encoding remains the next substantial profiling target, including generated
chunk-call overhead, allocation/shrink and GC, rather than skipping validation
or substituting cached text. The earlier negative observations remain recorded.

Local acceptance passes: eight runtime unit checks including three validator
conformance tests, 40 safe-surface/regression gates, real rooted conversions,
444,031 handoff releases with clean shutdown, 21 AOT unit checks, 20 AOT
contract checks, generated-SCM regeneration/provenance, workspace Clippy with
warnings denied, formatting, whitespace and filename checks. No new skip,
kernel modification, relaxed limit or forced SIMD target flag was introduced.
The existing one runtime latency control and five separately supervised AOT
tests remain outside this focused run. No commit, push, exact-head remote CI,
independent-VM admission or downstream parser sub-100-ms qualification is
claimed by this conversion receipt.

### Encoding allocation, GC and chunk-call audit

The next production refinement follows the installed official sources rather
than modifying the kernel. `std/ffi.ss`'s `def-C` expansion uses `##c-code`;
the checked private encoding leaf now uses that mechanism instead of a
`c-lambda` argument-conversion/foreign-return frame at each chunk. Its helper
receives Scheme words directly, checks string/bytevector types, fixnum indices,
nonnegative indices and all previous bounds, then returns an immediate fixnum.
The 256-character limit and outer Scheme scheduling polls are unchanged.
There is no per-character FFI call or pointer retained beyond the leaf.

The installed Gambit `_kernel.scm` definition of `##make-u8vector` omits
initialization when the fill argument is absent; the corresponding header
uses `___MAKEU8VECTORSMALL1` / `___MAKEVECTSMALLNOFILL`. The encoder now uses
this official primitive after its capacity admission. Every returned byte is
written, unused capacity is shrunk before return, and an error cannot publish
the partial vector. This removes redundant pre-zeroing, not initialization of
published output. The release-generated C confirms the no-fill allocation,
inline C expression and retained polls; generic `SCMOBJ_TO_U64` chunk
conversions are absent. The C compiler may still emit a local helper call:
this is not a claim that all function calls have disappeared.

The fixture has 3,415 characters, an 8,196-byte UTF-8 result and a 13,660-byte
maximum-capacity allocation per conversion: 14 bounded chunks. Thus 100,000
fresh encodings request 1.366 GB of bytevector payload before shrinking,
excluding headers, root registry and Rust allocations. The complete owned
conversion additionally enters the ABI for encoding/root publication, length,
bulk copy and release. The old generic chunk conversions were internal to
encoding, not 14 Rust requests or 14 parsed documents.

`t/utf8-cost.ss` provides a compiled, owner-local diagnostic matrix invoked
through `just scheme-utf8-cost` and native `gxi`. Each of the three loads runs
20 samples of control, initialized allocation/shrink, unfilled allocation/
shrink, reused-buffer leaf encoding, and fresh encoding. It uses the installed
official `##exec-stats` / `##process-statistics` indices for CPU time and GC.
All measurements and reporting are outside kernel code; no GC is disabled,
heap limit enlarged, watchdog relaxed or cached text admitted as fresh input.
These controls do not include Rust copying, validation or root publication.

For 100,000 operations per sample, the following are arithmetic means over
the complete 20-sample diagnostic run, not Divan medians:

| Diagnostic phase | CPU mean (user + system) | GC CPU mean | GC collections across 20 samples |
|---|---:|---:|---:|
| Control | 0.788 ms | 0 ms | 0 |
| Initialized allocation/shrink | 303.6 ms | 243.5 ms | 718 |
| Unfilled allocation/shrink | 245.7 ms | 240.1 ms | 717 |
| Reused-buffer leaf | 628.4 ms | 0 ms | 0 |
| Fresh encoding | 887.8 ms | 258.4 ms | 718 |

GC is approximately 29% of CPU in this fresh-encoding control, while the
reused-buffer leaf still has substantial encoding/chunk-dispatch work. Both
control and leaf report only 64 bytes of driver/statistics allocation per
sample and no GC. This rules out interpreter allocation as their large cost,
but does not isolate C arithmetic from Scheme chunk dispatch. The allocation
controls retain the same worst-case capacity: skipping initialization does
not remove the capacity/collector pressure. Do not subtract ordered means to
claim an exact causal speedup.

An additional `just scheme-utf8-cost dispatch` completes 20 samples at each
load with the same 14-chunk iteration shape and checked C entry but an empty
character range. It writes and counts zero output bytes, not fake successful
transfers. Its mean CPU times are 0.209 / 1.509 / 14.260 ms for 1,000 /
10,000 / 100,000 jobs; all samples have zero GC and 64 bytes of statistics/
driver allocation. The empty-entry control is much smaller than the full
628.4-ms leaf control at 100,000 jobs. Thus after the call-path cleanup,
character encoding and capacity/GC pressure deserve priority over further
eliminating generic call framing. This is a diagnostic magnitude comparison,
not an exact subtraction or isolated per-call latency guarantee.

The official allocation-byte counter produced negative deltas across GC in
these runs (18/20 fresh-encoding samples at 100,000 operations). It is therefore
not admitted as a gross-allocation metric. The installed `mem.c` counter uses
occupied heap/stack and still-object accounting across collections; attribution
of its negative delta remains unresolved. No kernel bug or negative allocation
claim is inferred. Requested capacities, GC counts and GC CPU are reported
separately. The first interpreted diagnostic was also rejected as cost evidence:
its driver allocated hundreds of kilobytes to megabytes per sample. The
compiled fixture removes that confound. An initial diagnostic build had an
incorrect relative target and a subsequent edit had an extra closing parenthesis;
both were repaired, and the final entire matrix exited zero. The entrypoint
defines `main` for gxi's automatic invocation without an additional explicit
top-level call, preventing duplicate runs. The diagnostic module exports the
named `run-cost-matrix`, not `main`.

The actual release fresh-conversion matrix also completed all 20 samples per
load with exact changing-text comparisons and exit zero:

| Transfers per sample | Median | Observed range | Samples |
|---:|---:|---:|---:|
| 1,000 | 16.15 ms | 7.667–32.58 ms | 20 |
| 10,000 | 223.1 ms | 98.85–730.3 ms | 20 |
| 100,000 | 2.060 s | 1.362–3.305 s | 20 |

The larger-load medians are worse than the preceding observation. This is a
negative end-to-end performance observation, not an acceleration pass; host
variance prevents causal attribution without an interleaved matched run.
All 2,220,000 fresh transfers still include encoding, owned bulk copy, full
SIMD validation and exact content assertions, with no snapshot substitution.
These are bridge transfers, not Org parsing latency or document concurrency.

Complete scalar parity, surrogate rejection, in-place mutation, empty/ASCII
input and root/GC isolation pass on the updated encoder. New negative cases
cover negative indices, flonums, a false value and a bignum. The safe-surface
gate also rejects reintroducing generic `c-lambda` chunks or pre-filled capacity.
Eight runtime unit tests, 40 safe-surface gates, live FFI conversions, 444,031
handoff releases, 21 AOT unit tests, 20 AOT contracts and warnings-denied
workspace Clippy pass. Existing ignored controls remain unchanged; no new skip
is introduced. No commit, push, remote CI or Org parser admission is claimed.

## Matched changing-text audit and conservative rollback

The subsequent audit uses `just bridge-utf8-matched`: one existing full-program
AOT qualification image, one owner, identical roots, owned bulk copy, full
SIMD validation, exact content assertions and root release for every variant.
Four different 8,196-byte texts rotate every request; every transfer re-encodes
the selected current Scheme string. There is no snapshot or memoization.
The fixture is not concurrent Org parsing, nor a test of scheduler parallelism.
Encoding selection is test-only, not an added production runtime choice.

For each load, ten ABBA groups give **20 complete samples per variant**.
The paired ratio is the median of the twenty adjacent B/A ratios, balancing
chronology in both directions. CPU, GC CPU and collections come from the
official `##process-statistics` snapshots outside the timed batch. No forced
GC, heap override, kernel patch, skipped validation or widened acceptance is
used. Build/test work does not overlap the measurement process. Diagnostics
report negative decisions rather than turning them into an acceleration pass.

Modes are: 0, frozen filled-capacity/generic numeric `c-lambda` historical
control; 1, the then-production unfilled-capacity/inline `##c-code` strategy;
2, an experimental bounded counting pass followed by encoding. All preserve
strict Unicode scalar rejection and at-most-256-character C leaves.

| Transfers | Historical wall / CPU median | Inline wall / CPU median | Paired inline/historical |
|---:|---:|---:|---:|
| 1,000 | 21.483 / 9.990 ms | 19.919 / 9.376 ms | 0.903752 |
| 10,000 | 144.139 / 99.656 ms | 135.891 / 94.964 ms | 0.981726 |
| 100,000 | 1,724.281 / 961.426 ms | 1,780.612 / 915.479 ms | **1.032497: reject** |

The initial all-load run completed every batch and exited zero. Its 1,000-load
console capture was truncated; an additional complete 20-sample-per-variant
1,000-load run supplies the first row, not a timing-based best-run selection.
The 10,000 and 100,000 rows are from the initial all-load run. The results are
not a confidence-bound claim of a stable regression or improvement on a busy
host. The strict admission decision nevertheless rejects the inline/unfilled
combination because the large-load paired wall ratio exceeds one. Lower CPU
does not override that negative end-to-end observation.

The counting candidate follows the installed official Gambit
`gambit/string/string.scm` `##string->utf8-length` / `##string->utf8` algorithm:
compute current character widths before allocation. Its private C counting
leaf reduces `1 + (c > 127) + (c > 2047) + (c > 65535)` and rejects invalid
scalars. This is not a claim that the C compiler vectorized encoding. A
1,024-byte spare admits the unchanged encoding leaf's worst-case chunk check;
requested capacity for this corpus falls from 13,660 to 9,220 bytes (32.5%).
It still scans every input again and is **not admitted into production**.

| Transfers | Inline wall / CPU median | Counted wall / CPU median | Paired counted/inline |
|---:|---:|---:|---:|
| 1,000 | 16.386 / 8.762 ms | 18.751 / 10.054 ms | **1.041617: reject** |
| 10,000 | 186.634 / 95.310 ms | 224.965 / 107.138 ms | **1.143925: reject** |
| 100,000 | 2,180.616 / 907.047 ms | 2,419.340 / 1,022.758 ms | **1.154983: reject** |

At 100,000 transfers, counting reduces total collections across twenty samples
from 18,519 to 12,657 and GC CPU median from 137.269 to 104.802 ms, but increases
total CPU median from 907.047 to 1,022.758 ms. Reduced allocation pressure does
not compensate for its additional scan/call/control cost in this workload.
GC counters are observations; requested capacities are not measured gross
allocations. The previously invalid allocation-byte delta is not reused.

The larger historical load is close to linear in CPU: 99.656 ms at 10,000
versus 961.426 ms at 100,000, with 1,853 versus 18,518 total collections across
the twenty samples. Wall time grows more than CPU. This establishes that the
extra observed elapsed time is not all charged to this process's CPU; it does
not uniquely identify descheduling, competing work or memory latency. It is
not evidence of a superlinear encoder algorithm or a diagnosed kernel bug.
The full AOT control image and the production archive differ, so their
absolute times and GC counts must not be compared as matched controls.

Production consequently returns to **filled capacity and generic bulk call
framing**, while retaining every newer object/fixnum/range check, strict scalar
rule, no-allocation leaf and poll boundary. The generic call now accepts
checked Scheme-object indices to preserve malformed-index rejection; this
is deliberately not described as a byte-for-byte restoration of mode 0.
Independent ownership, one owned bulk copy and complete SIMD validation remain
unchanged. The rejected inline implementation is frozen in a qualification-only
module so later production changes cannot silently redefine its controls.
Source guards are aligned with the rollback, not removed or weakened: the
same exhaustive malformed-input tests still pass. The negative candidate is
retained only as an explicit test fixture, not a selectable runtime strategy.

After rollback and generated-source refresh, the **actual public** fresh
conversion API completed all three release loads with twenty samples each,
exact changing-text checks and exit zero:

| Transfers per sample | Median | Observed range | Samples |
|---:|---:|---:|---:|
| 1,000 | 16.24 ms | 8.791–39.29 ms | 20 |
| 10,000 | 167.4 ms | 98.53–229.2 ms | 20 |
| 100,000 | 1.782 s | 1.361–2.527 s | 20 |

This is another complete 2,220,000 fresh transfers through the production
archive, not the comparison export. These absolute medians are **not** a new
matched acceleration claim against an earlier independent production run.
In particular, neither these numbers nor the test-only AOT comparison qualify
Org document parsing, Rust parse parallelism or the historical sub-100-ms
Org performance gate. No end-to-end Org acceptance is asserted.

Final qualification after rollback passes the full production Unicode/mutation/
malformed-index/root-GC contracts and the three frozen controls' complete
scalar/surrogate/mutation conformance. The actual production conversion suite,
eight runtime unit tests, forty safe-surface gates, twenty-one AOT unit tests
and twenty AOT contracts pass. Existing one runtime and five AOT ignored
controls are unchanged, with no new skip. Workspace and actor-AOT all-target
Clippy pass with warnings denied, as do formatting, diff whitespace and the
filename gate. The initial actor-AOT Clippy found a similar local variable name
and an overflow-prone midpoint expression in the new diagnostic; both were
fixed without lint allowances. Unresolved clocks or non-finite/negative cost
counters now fail the diagnostic explicitly, and a median requires all twenty
samples.

An additional 1,000-transfer AOT execution verifies the frozen-control linkage
after rollback, with twenty samples per variant and exit zero. Its paired
ratios are 0.899742 (inline/historical) and 1.001965 (counted/inline); it neither
replaces the earlier matrices nor readmits either rejected strategy. Early
fixture-build failures (compiler option calling convention, missing static
dependency staging and omitted linked-program initialization) were repaired;
there is no remaining failed correctness/build/lint gate. Performance rejection
records remain explicit. The rollback does not establish that character
encoding/GC optimization is exhausted or that the whole parsing architecture
is qualified. No commit, push, remote CI or Org acceptance is claimed here.
