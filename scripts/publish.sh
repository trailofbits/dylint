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

# smoelius: Previously, I was checking `newest_version`, but that was producing false positives in
# the sense that `cargo build` couldn't find the new version. So now I am listing the available
# versions and checking whether the new one is included.
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

    cargo publish

    while ! published "$NAME" "$VERSION"; do
        sleep 10s
    done

    popd
done
