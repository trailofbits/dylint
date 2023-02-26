#! /bin/bash

# set -x
set -euo pipefail

SCRIPTS="$(dirname "$(realpath "$0")")"
WORKSPACE="$(realpath "$SCRIPTS"/..)"

cd "$WORKSPACE"/utils

for UTIL in *; do
    pushd "$UTIL" >/dev/null

    cargo rdme "$@"

    popd >/dev/null
done
