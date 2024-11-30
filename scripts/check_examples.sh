#! /bin/bash

# smoelius: This script is no longer used in CI.

# set -x
set -euo pipefail

if [[ $# -ne 0 ]]; then
    echo "$0: expect no arguments" >&2
    exit 1
fi

SCRIPTS="$(dirname "$(realpath "$0")")"
WORKSPACE="$(realpath "$SCRIPTS"/..)"

cd "$WORKSPACE"/examples

for EXAMPLE in */*; do
    if [[ ! -d "$EXAMPLE" ]]; then
        continue
    fi

    pushd "$EXAMPLE"
    cargo check --all-targets
    popd
done
