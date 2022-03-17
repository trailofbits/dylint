#! /bin/bash

# set -x
set -euo pipefail

if [[ $# -ne 0 ]]; then
    echo "$0: expect no arguments" >&2
    exit 1
fi

cd "$(dirname "$0")"/..

find . -name '*.yml' -exec sed -i 's/^\([^#]*:[[:space:]]*\)"\(.*\)"\([[:space:]]*\)$/\1\2\3/' {} \;
