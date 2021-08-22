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
sed -n 's/^channel = "\([^"]*\)"$/\1/;T;p' rust-toolchain
