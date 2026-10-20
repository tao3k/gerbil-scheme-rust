#!/bin/sh
# Finite protocol checks only; native/runtime acceptance remains separate.
set -eu
quint typecheck specs/owner_lifecycle.qnt
quint test specs/owner_lifecycle.qnt --main owner_lifecycle_small --seed 1
quint test specs/owner_lifecycle.qnt --main owner_lifecycle_wide --seed 1
quint run specs/owner_lifecycle.qnt --main owner_lifecycle_small --invariant safety --seed 1 --max-samples 1000 --max-steps 64 --verbosity 1
quint run specs/owner_lifecycle.qnt --main owner_lifecycle_wide --invariant safety --seed 42 --max-samples 1000 --max-steps 64 --verbosity 1
sh tools/check-owner-model-mutations.sh
