#!/bin/sh
# Immutable compiler fixture: avoid executing a file just opened for writing.
output=''
while [ "$#" -gt 0 ]; do
  if [ "$1" = '-o' ]; then shift; output="$1"; fi
  shift
done
[ -n "$output" ] || exit 64
printf 'invoke\n' >> "$(dirname "$output")/compiler.log"
: > "$output"
