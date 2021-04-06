#! /bin/bash

# set -x
set -euo pipefail

if [[ $# -ne 0 ]]; then
    echo "$0: expect no arguments" >&2
    exit 1
fi

package_name() {
    grep -o '^name = "[^"]*"$' Cargo.toml |
    sed 's/^name = "\([^"]*\)"$/\1/'
}

package_version() {
    grep -o '^version = "[^"]*"$' Cargo.toml |
    sed 's/^version = "\([^"]*\)"$/\1/'
}

crates_io_newest_version() {
    curl "https://crates.io/api/v1/crates/$1" |
    jq -r '.crate | .newest_version'
}

published() {
    [[ "$(crates_io_newest_version "$1")" = "$2" ]]
}

# smoelius: Publishing in this order ensures that all dependencies are met.
DIRS="internal driver dylint-link examples dylint cargo-dylint utils/linting utils/testing"

for DIR in $DIRS; do
    pushd "$DIR"

    NAME="$(package_name)"
    VERSION="$(package_version)"

    if published "$NAME" "$VERSION"; then
        popd
        continue
    fi

    cargo publish

    while ! published "$NAME" "$VERSION"; do
        sleep 10s
    done

    popd
done
