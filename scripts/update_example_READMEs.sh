#! /bin/bash

# set -x
set -euo pipefail

if [[ $# -ne 0 ]]; then
    echo "$0: expect no arguments" >&2
    exit 1
fi

cd "$(dirname "$0")"/../examples

TMP="$(mktemp)"

LISTED=

IFS=
cat README.md |
while read X; do
    if [[ "$X" =~ ^\*[[:space:]] ]]; then
        if [[ -z "$LISTED" ]]; then
            grep '^description = "[^"]*"$' */Cargo.toml |
            sed 's,^\([^/]*\)/Cargo.toml:description = "\([^"]*\)"$,* [`\1`](./\1): \2,'
            LISTED=1
        fi
        continue
    fi
    echo "$X"
done |
cat > "$TMP"

mv "$TMP" README.md

for EXAMPLE in *; do
    if [[ ! -d "$EXAMPLE" || "$EXAMPLE" = src ]]; then
        continue
    fi

    pushd "$EXAMPLE" >/dev/null

    (
        echo "# $EXAMPLE"
        echo
        cat src/*.rs |
        sed -n 's,^[[:space:]]*///[[:space:]]*\(.*\)$,\1,;T;p'
    ) > README.md

    popd >/dev/null
done
