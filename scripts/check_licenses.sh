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
    # smoelius: Good explanation of the differences between the BSD-3-Clause and MIT licences:
    # https://opensource.stackexchange.com/a/582
    echo "$X" | grep -w 'Apache\|BSD-3-Clause\|MIT'
done
