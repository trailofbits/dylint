#! /bin/bash

# set -x
set -euo pipefail

if [[ $# -ne 0 ]]; then
    echo "$0: expect no arguments" >&2
    exit 1
fi

find . -mindepth 2 -name Cargo.toml |
xargs -n 1 cargo license --manifest-path |
while read X; do
    # smoelius: Exception for Cargo dependencies.
    if [[ "$X" = 'MPL-2.0+ (3): bitmaps, im-rc, sized-chunks' ]]; then
        continue
    fi
    # smoelius: Good explanation of the differences between the BSD-3-Clause and MIT licenses:
    # https://opensource.stackexchange.com/a/582
    if ! grep -w 'Apache\|BSD-3-Clause\|ISC\|MIT\|N/A' <(echo "$X") >/dev/null; then
        echo "$X"
        exit 1
    fi
done
