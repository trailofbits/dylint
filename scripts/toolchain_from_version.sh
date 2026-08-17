#! /bin/bash

set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "$0: expect one argument: Rust version" >&2
    exit 1
fi

VERSION="$1"

TMP="$(mktemp -d)"

git clone --branch rust-"$VERSION" 'https://github.com/rust-lang/rust-clippy' "$TMP" 2>/dev/null
cd "$TMP"

# smoelius: Clippy renamed its `rust-toolchain` files to `rust-toolchain.toml` in this commit:
# https://github.com/rust-lang/rust-clippy/commit/19c7c46d48dc659de44d85845d53ed594b95c286
# Both names must be handled because "$VERSION" could refer to either side of the rename.
if [[ -f rust-toolchain ]]; then
    TOOLCHAIN_FILE='rust-toolchain'
else
    TOOLCHAIN_FILE='rust-toolchain.toml'
fi

sed -n 's/^channel = "\([^"]*\)"$/\1/;T;p' "$TOOLCHAIN_FILE"
