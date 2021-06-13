#! /bin/bash

# set -x
set -euo pipefail

if [[ $# -ne 0 ]]; then
    echo "$0: expect no arguments" >&2
    exit 1
fi

cargo license |
while read X; do
    # smoelius: Exception for Cargo dependencies.
    if [[ "$X" = 'MPL-2.0+ (3): bitmaps, im-rc, sized-chunks' ]]; then
        continue
    fi
    echo "$X" | grep -w 'Apache\|MIT'
done
