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

# smoelius: Previously, I was checking crates.io using curl, but that was producing false positives
# in the sense that subsequent runs of `cargo build` couldn't find the package. So now I am using
# `cargo search`.
published() {
    cargo search "$1" | grep "^$1 = \"$2\""
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
