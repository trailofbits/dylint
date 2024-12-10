#! /bin/bash

# set -x
set -euo pipefail

if [[ $# -ne 0 ]]; then
    echo "$0: expect no arguments" >&2
    exit 1
fi

SCRIPTS="$(dirname "$(realpath "$0")")"
WORKSPACE="$(realpath "$SCRIPTS"/..)"

cd "$WORKSPACE"

find . -name Cargo.toml |
while read -r X; do
    if [[ "$X" = './examples/testing/marker/Cargo.toml' ]]; then
        continue
    fi
    cargo update --workspace --manifest-path "$X"
done
