#!/bin/sh
# Reject missing protocol guards with real safety counterexamples, not CLI errors.
set -eu
capture=$(mktemp "${TMPDIR:-/tmp}/gerbil-owner-model.XXXXXX")
trap 'rm -f "$capture"' EXIT HUP INT TERM
for mutation in unsafePublicationStep unsafeExecutorStep unsafeJoinStep unsafeForeignStep; do
    if quint run specs/owner_lifecycle.qnt --main owner_lifecycle_small \
        --step "$mutation" --invariant safety --seed 1 \
        --max-samples 1000 --max-steps 32 --verbosity 1 >"$capture" 2>&1; then
        cat "$capture"
        printf '%s\n' "$mutation: unexpectedly preserved safety" >&2
        exit 1
    else
        status=$?
        if [ "$status" -ne 1 ] || ! grep -Fq 'error: Invariant violated' "$capture"; then
            cat "$capture"
            printf '%s\n' "$mutation: checker failed without a safety counterexample" >&2
            exit 1
        fi
    fi
    printf '%s\n' "$mutation: expected safety counterexample"
done
