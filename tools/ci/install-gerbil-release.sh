#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 tao3k team and Contributors
# SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

set -euo pipefail

release_root="${1:?usage: install-gerbil-release.sh RELEASE_ROOT}"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)
    # Native four-worker indexing and streaming compilation; Gambit is unchanged.
    revision=2591dcd9b7c6d2c4e9dd8611a17c5b1a5d82bbdb
    release_os=linux
    release_arch=x86_64
    tag=gerbil-v0.19-2591dcd9b7c6d2c4e9dd8611a17c5b1a5d82bbdb-linux-x86_64-portable-full-single-host-unlimited-patcha379bb244bfa
    sha256=3b345b1af68dfbbc0e73bae64941e4c8910d0ca192d81432ce699c5af385b565
    asset_id=690615494
    ;;
  Darwin-arm64)
    revision=1cfb032c7a1612637da6205f5f8a15683eba3ba0
    release_os=darwin
    release_arch=aarch64
    tag=gerbil-v0.19-1cfb032c7a1612637da6205f5f8a15683eba3ba0-darwin-aarch64-gcc16-arm64-aot-tools-single-host-unlimited-multiple-vms-patch618d76f9eb72
    sha256=0e18d063f888a73d9f9692d441fb3776898114e8440eada6876937d289578183
    asset_id=624730911
    ;;
  *)
    printf 'unsupported Gerbil release host: %s-%s\n' "$(uname -s)" "$(uname -m)" >&2
    exit 64
    ;;
esac

archive="$tag.tar.gz"
download="${RUNNER_TEMP:?}/$archive"
mkdir -p "$release_root"
if ! curl --fail --location --retry 3 --header 'Accept: application/octet-stream' \
  --output "$download" \
  "https://api.github.com/repos/tao3k/gerbil-bazel/releases/assets/$asset_id"; then
  # Both official endpoints select the same immutable asset. Verify its fixed
  # digest below before extracting, including after a transport failure.
  curl --fail --location --retry 3 --output "$download" \
    "https://github.com/tao3k/gerbil-bazel/releases/download/$tag/$archive"
fi
echo "$sha256  $download" | shasum -a 256 --check
tar -xzf "$download" -C "$release_root" --strip-components=1

jq -e --arg revision "$revision" --arg os "$release_os" --arg arch "$release_arch" \
  '.schema == "gerbil-bazel.toolchain-release.v1" and
   .upstreamRevision == $revision and
   .platform.os == $os and .platform.arch == $arch' \
  "$release_root/gerbil-toolchain-release.json" >/dev/null

# activate is supplied by the verified release archive.
# shellcheck disable=SC1091
source "$release_root/activate"
echo "$GERBIL_PREFIX/bin" >> "${GITHUB_PATH:?}"
{
  echo "GERBIL_PREFIX=$GERBIL_PREFIX"
  echo "GERBIL_HOME=$GERBIL_HOME"
  echo "GAMBOPT=$GAMBOPT"
} >> "${GITHUB_ENV:?}"

"$GERBIL_PREFIX/bin/gerbil" --version
"$GERBIL_PREFIX/bin/gerbil" interactive -e '(displayln "gerbil-ready")'
