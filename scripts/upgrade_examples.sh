#! /bin/bash

# set -x
set -euo pipefail

if [[ $# -ne 0 ]]; then
    echo "$0: expect no arguments" >&2
    exit 1
fi

SCRIPTS="$(dirname "$(realpath "$0")")"

cd "$(dirname "$0")"/..

CARGO_DYLINT='timeout 10m cargo run -p cargo-dylint -- dylint'

for EXAMPLE in examples/*; do
    if [[ ! -d "$EXAMPLE" ]]; then
        continue
    fi

    # smoelius: `allow_clippy` is handled with `clippy` below.
    if [[ "$EXAMPLE" = 'examples/allow_clippy' ]]; then
        continue
    fi

    # smoelius: If the example's directory has changes, assume the example was already upgraded and
    # the script had to be restarted.
    if ! git diff --exit-code "$EXAMPLE"; then
        continue
    fi

    # smoelius: `clippy` requires special care.
    if [[ "$EXAMPLE" = 'examples/clippy' ]]; then
        PREV_REV="$(sed -n 's/^clippy_utils\>.*\(\<\(rev\|tag\) = "[^"]*"\).*$/\1/;T;p' "$EXAMPLE"/Cargo.toml)"
        PREV_CHANNEL="$(sed -n 's/^channel = "[^"]*"$/&/;T;p' "$EXAMPLE"/rust-toolchain)"

        $CARGO_DYLINT --upgrade "$EXAMPLE" 2>/dev/null || true

        REV="$(sed -n 's/^clippy_utils\>.*\(\<\(rev\|tag\) = "[^"]*"\).*$/\1/;T;p' "$EXAMPLE"/Cargo.toml)"
        sed -i "s/^\(clippy_lints\>.*\)\<\(rev\|tag\) = \"[^\"]*\"\(.*\)$/\1$REV\3/" "$EXAMPLE"/Cargo.toml

        # smoelius: If `clippy`'s `rust-toolchain` file changed, upgrade `allow_clippy` to the Rust
        # version that `clippy` used previously. Note that `clippy` can be upgraded without its
        # `rust-toolchain` file changing.
        if ! git diff --exit-code "$EXAMPLE"/rust-toolchain; then
            pushd examples/allow_clippy
            sed -i "s/^\(clippy_utils\>.*\)\<\(rev\|tag\) = \"[^\"]*\"\(.*\)$/\1$PREV_REV\3/" Cargo.toml
            sed -i "s/^channel = \"[^\"]*\"$/$PREV_CHANNEL/" rust-toolchain
            cargo build --tests
            popd
        fi
    fi

    $CARGO_DYLINT --upgrade "$EXAMPLE" --bisect
done
