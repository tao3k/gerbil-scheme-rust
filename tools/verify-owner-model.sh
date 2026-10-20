#!/bin/sh
# Preserve symbolic output outside the checkout, including incomplete attempts.
set -eu
bound=${1:-12}
case "$bound" in
    8|12) ;;
    *) printf '%s\n' 'supported explicit bounds: 8 or 12' >&2; exit 2 ;;
esac
repo_root=$(CDPATH= cd "$(dirname "$0")/.." && pwd)
output=$(mktemp -d "${TMPDIR:-/tmp}/gerbil-owner-symbolic.XXXXXX")
printf 'Symbolic output: %s\n' "$output"
cd "$output"
exec quint verify "$repo_root/specs/owner_lifecycle.qnt" \
    --main owner_lifecycle_small --invariant safety --max-steps "$bound" \
    --apalache-config "$repo_root/specs/apalache.json"
