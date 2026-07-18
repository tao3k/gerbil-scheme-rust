#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
set -euo pipefail

source_root="${TEST_SRCDIR:?}/${TEST_WORKSPACE:?}"
workspace="${TEST_TMPDIR:?}/gerbil-workspace"

mkdir -p "${workspace}/scheme"
cp "${source_root}/build.ss" "${source_root}/gerbil.pkg" "${workspace}/"
cp "${source_root}/scheme/native.ss" "${source_root}/scheme/test-native.ss" \
  "${workspace}/scheme/"
chmod +x "${workspace}/build.ss"

cd "${workspace}"
export GERBIL_PATH="${workspace}/.gerbil"
./build.ss compile
gxi scheme/test-native.ss
