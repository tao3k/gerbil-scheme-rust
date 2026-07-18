#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
set -euo pipefail

# Prefer the project `devenv` Cargo. Fall back to conventional rustup paths only
# when the Bazel test environment did not inherit a Cargo launcher.
if ! command -v cargo >/dev/null 2>&1; then
    user_home="${HOME:-}"
    export PATH="${CARGO_HOME:-${user_home}/.cargo}/bin:/Users/${USER:-}/.cargo/bin:/home/${USER:-}/.cargo/bin:${PATH}"
fi

nix_library_path=""
for linker_flag in ${NIX_LDFLAGS:-}; do
    case "$linker_flag" in
        -L*)
            library_dir="${linker_flag#-L}"
            nix_library_path="${nix_library_path:+${nix_library_path}:}${library_dir}"
            ;;
    esac
done
if [[ -n "$nix_library_path" ]]; then
    export LIBRARY_PATH="${nix_library_path}${LIBRARY_PATH:+:${LIBRARY_PATH}}"
fi

printf 'cargo_gate toolchain: cargo=%s rustc=%s xcrun=%s nix_ldflags=%s\n' \
    "$(command -v cargo || true)" \
    "$(command -v rustc || true)" \
    "$(command -v xcrun || true)" \
    "$(test -n "${NIX_LDFLAGS:-}" && printf present || printf missing)"

gate="${1:?usage: cargo_gate.sh check|test|clippy|all}"
workspace="${TEST_SRCDIR:?}/${TEST_WORKSPACE:?}"
export CARGO_TARGET_DIR="${TEST_TMPDIR:?}/cargo-target"
cd "${workspace}"

case "${gate}" in
  check)
    cargo check --workspace --locked
    ;;
  test)
    cargo test --workspace --locked
    ;;
  clippy)
    cargo clippy --workspace --all-targets --locked -- -D warnings
    ;;
  all)
    cargo check --workspace --locked
    cargo test --workspace --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    ;;
  *)
    echo "unknown Cargo gate: ${gate}" >&2
    exit 64
    ;;
esac
