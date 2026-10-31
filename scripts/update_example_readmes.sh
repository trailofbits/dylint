#! /bin/bash

# set -x
set -euo pipefail

if [[ $# -ne 0 ]]; then
    echo "$0: expect no arguments" >&2
    exit 1
fi

SCRIPTS="$(dirname "$(realpath "$0")")"
WORKSPACE="$(realpath "$SCRIPTS"/..)"

cd "$WORKSPACE"/examples

TMP="$(mktemp)"

CATEGORIES=(general supplementary restriction experimental testing)
LISTED=

IFS=
cat README.md |
while read -r X; do
    if [[ "$X" =~ ^\| ]]; then
        if [[ -z "$LISTED" ]]; then
            CATEGORY="${CATEGORIES[0]}"
            # shellcheck disable=SC2206
            CATEGORIES=(${CATEGORIES[@]:1})
            echo '| Example | Description/check |'
            echo '| - | - |'
            # shellcheck disable=SC2016
            grep '^description = "[^"]*"$' "$CATEGORY"/*/Cargo.toml |
            sed 's,^\([^/]*/\([^/]*\)\)/Cargo.toml:description = "\([^"]*\)"$,| [`\2`](./\1) | \3 |,' |
            sed 's/| A lint to check for \([^|]*\) |$/| \u\1 |/'
            LISTED=1
        fi
        continue
    else
        LISTED=
    fi
    echo "$X"
done |
cat > "$TMP"

mv "$TMP" README.md

prettier --write README.md
