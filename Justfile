# Bridge-owned qualification; model checks never substitute for FFI or latency tests.
build_env := if os() == "macos" { "env -u SDKROOT CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=/usr/bin/cc" } else { "env" }
model-check:
    quint typecheck specs/bridge_handoff.qnt
    quint test specs/bridge_handoff.qnt --main bridge_handoff_small --seed 1
    quint test specs/bridge_handoff.qnt --main bridge_handoff_wide --seed 1
    quint run specs/bridge_handoff.qnt --main bridge_handoff_small --invariant safety --seed 1 --max-samples 1000 --max-steps 50 --verbosity 1
    quint run specs/bridge_handoff.qnt --main bridge_handoff_wide --invariant safety --seed 1 --max-samples 1000 --max-steps 50 --verbosity 1

# Bounded symbolic verification, not an unbounded theorem.
model-verify:
    quint verify specs/bridge_handoff.qnt --main bridge_handoff_small --invariant safety --max-steps 12

bridge-test:
    {{ build_env }} cargo test -p gerbil-scheme --features native --test runtime_lifecycle calls_scalar_export_in_process -- --nocapture

bridge-contracts:
    {{ build_env }} cargo test -p gerbil-scheme --features native --test unit_test
    {{ build_env }} cargo test -p gerbil-scheme --features native --test runtime_round_trip --test exact_integer --test rooted_value --test identity
    {{ build_env }} cargo test -p gerbil-scheme-aot-build --test unit_test

bridge-targets:
    {{ build_env }} cargo check -p gerbil-scheme --features native --all-targets --locked

bridge-path-check:
    ! rg --files --hidden -g '!.git/**' -g '!target/**' -g '!.gerbil/**' | rg -i '(^|/)[^/]*native[^/]*$'

bridge-lint:
    {{ build_env }} cargo clippy -p gerbil-scheme -p gerbil-scheme-aot-build --features gerbil-scheme/native --all-targets --locked -- -D warnings

bridge-bench-smoke:
    {{ build_env }} cargo bench -p gerbil-scheme --features native --bench ffi --profile dev -- --test

bridge-benchmark *args:
    {{ build_env }} cargo bench -p gerbil-scheme --features native --bench ffi -- {{ args }}

bridge-bench-controls:
    {{ build_env }} cargo test -p gerbil-scheme --features native --test ffi_benchmark ratios_reject_unresolved_clock_samples_and_overflow

bridge-regenerate:
    {{ build_env }} GERBIL_SCHEME_RUST_UPDATE_GENERATED_SCM=1 GERBIL_SCHEME_RUST_CHECK_GENERATED_SCM=1 cargo check -p gerbil-scheme-sys --locked

smp-check:
    gxi scheme/runtime-smp-test.ss
