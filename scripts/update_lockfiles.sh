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

# smoelius: `template` must be handled specially.
find . -name Cargo.toml |
grep -vw template |
while read -r X; do
    cargo update --manifest-path "$X"
done
