#!/bin/sh
# SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
# Only this source-owned inode is executed. The per-build command is read by
# the shell, never execve'd while another fork may retain its writer descriptor.
exec /bin/sh "$0.command" "$@"
