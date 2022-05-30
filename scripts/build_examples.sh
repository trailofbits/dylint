#! /bin/bash

# smoelius: This script is currently unused.

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

    pushd "$EXAMPLE" >/dev/null
    cargo build
    popd >/dev/null
done
