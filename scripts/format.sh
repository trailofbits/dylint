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

# smoelius: Hack to join multiline rustdoc comments.
find examples -name '*.rs' |
while read -r X; do
    TMP="$(mktemp)"
    cat "$X" | tr '\n' '\v' | sed 's,\(\v *//[!/] [(A-Za-z][^\v]*[)A-Za-z]\)\v *//[!/] \([(A-Za-z]\),\1 \2,g' | tr '\v' '\n' > "$TMP"
    mv "$TMP" "$X"
done

find examples -name '*.rs' |
while read -r X; do
    sed -i '/^[a-z_:]*_lint! {$/,/^}$/{
        s,^[a-z_:]*_lint! {$,mod x { // &,;
        s,^\( \+\)\([^ /].*\)$,\1mod y {} // \2,;
    }' "$X"
done

# smoelius: Skip root manifest. Related: https://github.com/rust-lang/rustfmt/issues/4432
find . -mindepth 2 -name Cargo.toml -exec cargo +nightly fmt --manifest-path {} \;

find examples -name '*.rs' |
while read -r X; do
    sed -i '/^mod x {$/,/^}$/{
        s,^ \+// \(.*\)$,\1,;
        s,^mod x { // \(.*\)$,\1,;
        s,^\( \+\)mod y {} // \(.*\)$,\1\2,;
    };
    /^mod x {$/d' "$X"
done
