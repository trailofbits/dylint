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

sed 's,^\[[^]]*\]: \.,&.,' README.md > cargo-dylint/README.md

cp cargo-dylint/README.md dylint/README.md
