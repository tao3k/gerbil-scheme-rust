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
    quint verify specs/bridge_handoff.qnt --main bridge_handoff_small --invariant safety --max-steps 12 --apalache-config specs/apalache.json

actor-model-check:
    quint typecheck specs/actor_batch.qnt
    quint test specs/actor_batch.qnt --main actor_batch_small --seed 1
    quint test specs/actor_batch.qnt --main actor_batch_wide --seed 1
    quint test specs/actor_batch.qnt --main processor_admission --seed 1
    quint run specs/actor_batch.qnt --main processor_admission --invariant safety --seed 1 --max-samples 1000 --max-steps 50 --verbosity 1
    quint run specs/actor_batch.qnt --main actor_batch_small --invariant safety --seed 1 --max-samples 1000 --max-steps 50 --verbosity 1
    quint run specs/actor_batch.qnt --main actor_batch_wide --invariant safety --seed 1 --max-samples 1000 --max-steps 50 --verbosity 1

actor-model-verify:
    quint verify specs/actor_batch.qnt --main actor_batch_small --invariant safety --max-steps 12

processor-model-verify:
    quint verify specs/actor_batch.qnt --main processor_admission --invariant safety --max-steps 12

vm-model-check:
    quint typecheck specs/vm_execution.qnt
    quint test specs/vm_execution.qnt --main vm_execution --seed 1
    quint run specs/vm_execution.qnt --main vm_execution --invariant safety --seed 1 --max-samples 1000 --max-steps 50 --verbosity 1

bridge-instance-inputs:
    {{ build_env }} cargo test -p gerbil-scheme-qualification --test build_inputs --locked

bridge-test:
    {{ build_env }} cargo test -p gerbil-scheme --test runtime_lifecycle calls_scalar_export_in_process -- --nocapture

# Focused registry regression: each target retains its own process-global owner.
bridge-root-contracts:
    {{ build_env }} cargo test -p gerbil-scheme --test rooted_value --test runtime_lifecycle --test exact_integer --locked -- --nocapture

bridge-contracts:
    {{ build_env }} cargo test -p gerbil-scheme --locked -- --nocapture
    {{ build_env }} cargo test -p gerbil-scheme-aot-build --lib --test unit_test --locked

bridge-workspace:
    {{ build_env }} cargo test --workspace --locked -- --nocapture

bridge-targets:
    {{ build_env }} cargo check -p gerbil-scheme --all-targets --locked

bridge-path-check:
    ! rg --files --hidden -g '!.git/**' -g '!target/**' -g '!.gerbil/**' | rg -i '(^|/)[^/]*native[^/]*$'

bridge-harness-test *args:
    {{ build_env }} cargo test -p gerbil-scheme-rust-project-harness --test unit_test --locked {{ args }}

bridge-lint:
    {{ build_env }} cargo clippy --workspace --all-targets --locked -- -D warnings

bridge-bench-smoke:
    {{ build_env }} cargo bench -p gerbil-scheme --bench ffi --profile dev --locked -- --test
    {{ build_env }} cargo bench -p gerbil-scheme --bench tokio_transport --profile dev --locked -- --test

bridge-benchmark *args:
    {{ build_env }} cargo bench -p gerbil-scheme --bench ffi --locked -- {{ args }}

bridge-transport-benchmark *args:
    {{ build_env }} cargo bench -p gerbil-scheme --bench tokio_transport --locked -- {{ args }}

bridge-actor-smoke:
    {{ build_env }} cargo bench -p gerbil-scheme-qualification --features actor-aot --bench actors --profile dev --locked -- --test

bridge-actor-benchmark *args:
    {{ build_env }} cargo bench -p gerbil-scheme-qualification --features actor-aot --bench actors --locked -- {{ args }}

# Same AOT image/owner/validation, ABBA, 20 complete samples per variant/load.
bridge-utf8-matched *args:
    {{ build_env }} cargo bench -p gerbil-scheme-qualification --features actor-aot --bench utf8 --locked -- {{ args }}

scheme-utf8-matched-contracts:
    GERBIL_PATH="{{ justfile_directory() }}/target/scheme-contracts" gxi -e '(import :gerbil/compiler) (for-each (lambda (source) (compile-module source [output-dir: "target/scheme-contracts/lib" optimize: #t invoke-gsc: #t])) ["crates/gerbil-scheme-qualification/scheme/utf8-inline-control.ss" "crates/gerbil-scheme-qualification/scheme/utf8-controls.ss"])'
    GERBIL_PATH="{{ justfile_directory() }}/target/scheme-contracts" gxi -e '(import :gerbil-scheme-rust/qualification/utf8-controls) (utf8-conformance)'

bridge-actor-lint:
    {{ build_env }} cargo clippy -p gerbil-scheme-qualification --features actor-aot --all-targets --locked -- -D warnings

# Experimental private runtime images; not independent-VM or parser admission.
bridge-instance-probe *args:
    {{ build_env }} cargo bench -p gerbil-scheme-qualification --features instance-probe --bench instances --locked -- {{ args }}

bridge-instance-lint:
    {{ build_env }} cargo clippy -p gerbil-scheme-qualification --features actor-aot,instance-probe --all-targets --locked -- -D warnings

bridge-bench-controls:
    {{ build_env }} cargo test -p gerbil-scheme --test ffi_benchmark ratios_reject_unresolved_clock_samples_and_overflow

# Producer qualification precedes any downstream dependency update. Diagnostic
# transport batches are not parser latency admission or Scheme SMP evidence.
bridge-handoff:
    {{ build_env }} cargo test -p gerbil-scheme --test tokio_handoff --locked -- --nocapture

bridge-qualify: bridge-path-check bridge-workspace bridge-bench-smoke bridge-lint

bridge-regenerate:
    {{ build_env }} GERBIL_SCHEME_RUST_UPDATE_GENERATED_SCM=1 GERBIL_SCHEME_RUST_CHECK_GENERATED_SCM=1 cargo check -p gerbil-scheme-sys --locked

smp-check:
    gxi scheme/runtime-smp-test.ss

# Initialization-free ABI observation; never admits independent VM execution.
bridge-vm-capabilities:
    {{ build_env }} cargo test -p gerbil-scheme --test unit_test vm_capabilities --locked -- --nocapture

# Execute a producer-owned SDK contract without copying it into the bridge.
bridge-sdk-contract source:
    gxi "{{ source }}"

# Official Gerbil module build and one source-owned test process.
scheme-contracts:
    GERBIL_PATH="{{ justfile_directory() }}/target/scheme-contracts" gxi build.ss compile
    GERBIL_PATH="{{ justfile_directory() }}/target/scheme-contracts" gxi scheme/runtime-test.ss

# Official process statistics; phase controls do not qualify parser latency.
scheme-utf8-cost *phases:
    GERBIL_PATH="{{ justfile_directory() }}/target/scheme-contracts" gxi t/build.ss compile
    GERBIL_PATH="{{ justfile_directory() }}/target/scheme-contracts" gxi t/utf8-cost-test.ss {{ phases }}
