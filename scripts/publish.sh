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
# My current method for checking whether a package is ready to be depended upon is to create a
# temporary package with one dependency (the matching package) and to run `cargo check` on it.
#
# This solution is less than ideal because, e.g., `cargo check` could fail for reasons other than
# the package being unavailable. But every other approach I've tried has definitely *not* worked,
# including checking `https://crates.io/api/v1/crates` with curl.
#
# The ideal solution would likely be to check `https://github.com/rust-lang/crates.io-index`. But
# the index's structure is not obvious, nor is how one would check it from the command line. This
# should be investigated further.
published() {
    pushd "$(mktemp --tmpdir -d tmp-XXXXXXXXXX)"
    trap popd RETURN
    cargo init
    sed -i "/^\[dependencies\]$/a $1 = \"$2\"" Cargo.toml
    echo '[workspace]' >> Cargo.toml
    cat > rust-toolchain << EOF
[toolchain]
channel = "nightly-2023-01-19"
components = ["llvm-tools-preview", "rustc-dev"]
EOF
    echo "Checking whether \`$1:$2\` is published ..." >&2
    RUSTFLAGS='-A non_snake_case' cargo check
}

# smoelius: Publishing in this order ensures that all dependencies are met.
DIRS="internal driver dylint-link dylint cargo-dylint utils/linting utils/testing"

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
