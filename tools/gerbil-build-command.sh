#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

set -euo pipefail

if (( $# == 0 )); then
  printf 'usage: gerbil-build-command.sh PROGRAM [ARG ...]\n' >&2
  exit 64
fi

# Keep foreign Rust/Nix compiler settings out of the selected Gerbil SDK.
# The native-build crate applies this same boundary to its child commands.
exec env -u CC -u CFLAGS -u CPPFLAGS -u LDFLAGS \
  -u CPATH -u C_INCLUDE_PATH -u CPLUS_INCLUDE_PATH -u LIBRARY_PATH \
  -u NIX_CFLAGS_COMPILE -u NIX_LDFLAGS -u DEVELOPER_DIR -u SDKROOT \
  "$@"
