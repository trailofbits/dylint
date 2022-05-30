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

CATEGORIES=(general restriction testing)
LISTED=

IFS=
cat README.md |
while read X; do
    if [[ "$X" =~ ^\| ]]; then
        if [[ -z "$LISTED" ]]; then
            CATEGORY="${CATEGORIES[0]}"
            CATEGORIES=(${CATEGORIES[@]:1})
            echo "| ${CATEGORY^} | Description |"
            echo '| - | - |'
            grep '^description = "[^"]*"$' "$CATEGORY"/*/Cargo.toml |
            sed 's,^\([^/]*/\([^/]*\)\)/Cargo.toml:description = "\([^"]*\)"$,| [`\2`](./\1) | \3 |,'
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

for EXAMPLE in */*; do
    if [[ ! -d "$EXAMPLE" ]]; then
        continue
    fi

    pushd "$EXAMPLE" >/dev/null

    (
        echo "# $(basename "$EXAMPLE")"
        echo
        cat src/*.rs |
        sed '\,^[[:space:]]*///[[:space:]]\?#[^![],d' |
        sed -n 's,^[[:space:]]*///[[:space:]]\?\(.*\)$,\1,;T;p'
    ) > README.md

    prettier --write README.md

    popd >/dev/null
done
