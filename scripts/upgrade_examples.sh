#! /bin/bash

set -euo pipefail

if [[ $# -ne 0 ]]; then
    echo "$0: expect no arguments" >&2
    exit 1
fi

SCRIPTS="$(dirname "$(realpath "$0")")"

cd "$(dirname "$0")"/..

for EXAMPLE in examples/*; do
    if [[ ! -d "$EXAMPLE" ]]; then
        continue
    fi

    # smoelius: `allow_clippy` is handled with `clippy` below.
    if [[ "$EXAMPLE" = 'examples/allow_clippy' ]]; then
        continue
    fi

    PREV_TAG="$(sed -n 's/^clippy_utils\>.*\(\<tag = "[^"]*"\).*$/\1/;T;p' "$EXAMPLE"/Cargo.toml)"

    cargo run -p cargo-dylint -- dylint --upgrade "$EXAMPLE"

    # smoelius: `clippy` requires special care.
    if [[ "$EXAMPLE" = 'examples/clippy' ]]; then
        pushd "$EXAMPLE"

        TAG="$(sed -n 's/^clippy_utils\>.*\(\<tag = "[^"]*"\).*$/\1/;T;p' Cargo.toml)"
        sed -i "s/^\\(clippy_lints\>.*\\)\<tag = \"[^\"]*\"\\(.*\\)$/\1$TAG\2/" Cargo.toml

        popd

        # smoelius: If `clippy`'s `rust-toolchain` file changed, upgrade `allow_clippy` to the Rust
        # version that `clippy` used previously. Note that `clippy` can be upgraded without its
        # `rust-toolchain` file changing.
        if ! git diff --exit-code "$EXAMPLE"/rust-toolchain; then
            PREV_VERSION="$(echo "$PREV_TAG" | sed 's/^\<tag = "rust-\([^"]*\)"$/\1/')"
            cargo run -p cargo-dylint -- dylint --upgrade examples/allow_clippy --rust-version "$PREV_VERSION"
        fi
    fi
done

"$SCRIPTS"/update_lockfiles.sh
