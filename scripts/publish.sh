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

crates_io_versions() {
    curl "https://crates.io/api/v1/crates/$1/versions" |
    jq -r '.versions | map(.num) | .[]'
}

published() {
    crates_io_versions "$1" | grep "^$2$"
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

    # smoelius: It appears that crates.io sometimes needs a chance to update, and I haven't found a
    # reliable way to tell whether a package is ready to be depended upon.
    while ! cargo check; do
        sleep 10s
    done

    cargo publish

    popd
done
