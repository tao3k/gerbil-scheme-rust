#!/usr/bin/env bash
set -euo pipefail

: "${GERBIL_REF:?GERBIL_REF is required}"
: "${GERBIL_SRC:?GERBIL_SRC is required}"
: "${GERBIL_PREFIX:?GERBIL_PREFIX is required}"
: "${CCACHE_DIR:?CCACHE_DIR is required}"

receipt="$GERBIL_PREFIX/bootstrap.receipt.json"
lock_dir="$GERBIL_PREFIX.bootstrap.lock"
build_cores="${GERBIL_BUILD_CORES:-2}"
ccache_max_size="${CCACHE_MAXSIZE:-2G}"

validate_install() {
  [[ -x "$GERBIL_PREFIX/bin/gxi" ]] &&
    [[ -x "$GERBIL_PREFIX/bin/gsc" ]] &&
    [[ -d "$GERBIL_PREFIX/lib" ]] &&
    [[ -f "$receipt" ]] &&
    jq -e --arg ref "$GERBIL_REF" \
      '.schema == "gerbil-scheme-rust.gerbil-toolchain-bootstrap-receipt.v1" and
       .status == "success" and .ref == $ref' \
      "$receipt" >/dev/null
}

if validate_install; then
  echo "Gerbil capability cache hit: $GERBIL_PREFIX"
  exit 0
fi

if ! command -v ccache >/dev/null; then
  echo "ccache is required for a Gerbil source bootstrap" >&2
  exit 1
fi
mkdir -p "$(dirname "$GERBIL_SRC")" "$(dirname "$GERBIL_PREFIX")" "$CCACHE_DIR"
if ! mkdir "$lock_dir" 2>/dev/null; then
  echo "Gerbil bootstrap lock is already held: $lock_dir" >&2
  exit 1
fi
trap 'rmdir "$lock_dir" 2>/dev/null || true' EXIT

rm -rf "$GERBIL_SRC" "$GERBIL_PREFIX"
mkdir -p "$GERBIL_SRC" "$GERBIL_PREFIX" "$CCACHE_DIR"

compiler="$(command -v gcc)"
wrapper_dir="$(dirname "$GERBIL_SRC")/ccache-wrappers"
mkdir -p "$wrapper_dir"
ln -sf "$(command -v ccache)" "$wrapper_dir/gcc"
export PATH="$wrapper_dir:$PATH"
export CC=gcc
export CCACHE_DIR
ccache --max-size "$ccache_max_size"
ccache --zero-stats

git init --quiet "$GERBIL_SRC"
git -C "$GERBIL_SRC" remote add origin https://github.com/mighty-gerbils/gerbil.git
git -C "$GERBIL_SRC" fetch --depth=1 origin "$GERBIL_REF"
git -C "$GERBIL_SRC" checkout --quiet --detach FETCH_HEAD

started_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
start_seconds=$SECONDS
cd "$GERBIL_SRC"
./configure --prefix="$GERBIL_PREFIX"
export GERBIL_BUILD_CORES="$build_cores"
make -j"$build_cores"
make install

if [[ ! -x "$GERBIL_PREFIX/bin/gxi" ]] ||
  [[ ! -x "$GERBIL_PREFIX/bin/gsc" ]] ||
  [[ ! -d "$GERBIL_PREFIX/lib" ]]; then
  echo "Gerbil install is incomplete: $GERBIL_PREFIX" >&2
  exit 1
fi

completed_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
elapsed_seconds=$((SECONDS - start_seconds))
temporary_receipt="$receipt.tmp"
jq -n \
  --arg schema "gerbil-scheme-rust.gerbil-toolchain-bootstrap-receipt.v1" \
  --arg status "success" \
  --arg ref "$GERBIL_REF" \
  --arg prefix "$GERBIL_PREFIX" \
  --arg compiler "$compiler" \
  --arg startedAt "$started_at" \
  --arg completedAt "$completed_at" \
  --argjson buildCores "$build_cores" \
  --argjson elapsedSeconds "$elapsed_seconds" \
  '{schema: $schema, status: $status, ref: $ref, prefix: $prefix,
    compiler: $compiler, buildCores: $buildCores, startedAt: $startedAt,
    completedAt: $completedAt, elapsedSeconds: $elapsedSeconds}' \
  >"$temporary_receipt"
mv "$temporary_receipt" "$receipt"
ccache --show-stats
echo "Gerbil capability cache populated: $GERBIL_PREFIX"
