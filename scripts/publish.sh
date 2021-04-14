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

# smoelius: I have been getting messages like the following from `cargo publish` in cases where the
# matching package was just published:
#
#   error: failed to prepare local package for uploading
#
#   Caused by:
#     no matching package named `dylint` found
#     location searched: registry `https://github.com/rust-lang/crates.io-index`
#
# No doubt, there are better ways to check `crates.io-index` than this.
published() {
    pushd "$(mktemp --tmpdir -d tmp-XXXXXXXXXX)"
    trap popd RETURN
    cargo init
    sed -i "/^\[dependencies\]$/a $1 = \"$2\"" Cargo.toml
    cat >> Cargo.toml << EOF
[workspace]
members = []
EOF
    cat >> rust-toolchain << EOF
[toolchain]
channel = "nightly"
components = ["llvm-tools-preview", "rustc-dev"]
EOF
    RUSTFLAGS='-A non_snake_case' cargo check
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
