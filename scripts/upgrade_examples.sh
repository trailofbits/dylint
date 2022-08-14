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

CARGO_DYLINT='timeout 10m cargo run -p cargo-dylint -- dylint'

for EXAMPLE in examples/*/* internal/template; do
    if [[ ! -d "$EXAMPLE" ]]; then
        continue
    fi

    # smoelius: `straggler` is handled with `clippy` below.
    if [[ "$EXAMPLE" = 'examples/testing/straggler' ]]; then
        continue
    fi

    # smoelius: If the example's directory has changes, assume the example was already upgraded and
    # the script had to be restarted.
    if ! git diff --exit-code "$EXAMPLE"; then
        continue
    fi

    # smoelius: `clippy` requires special care.
    if [[ "$EXAMPLE" = 'examples/testing/clippy' ]]; then
        PREV_REV="$(sed -n 's/^clippy_utils\>.*\(\<\(rev\|tag\) = "[^"]*"\).*$/\1/;T;p' "$EXAMPLE"/Cargo.toml)"
        PREV_CHANNEL="$(sed -n 's/^channel = "[^"]*"$/&/;T;p' "$EXAMPLE"/rust-toolchain)"

        $CARGO_DYLINT upgrade "$EXAMPLE" 2>/dev/null || true

        REV="$(sed -n 's/^clippy_utils\>.*\(\<\(rev\|tag\) = "[^"]*"\).*$/\1/;T;p' "$EXAMPLE"/Cargo.toml)"
        sed -i "s/^\(clippy_lints\>.*\)\<\(rev\|tag\) = \"[^\"]*\"\(.*\)$/\1$REV\3/" "$EXAMPLE"/Cargo.toml

        # smoelius: If `clippy`'s `rust-toolchain` file changed, upgrade `straggler` to the Rust
        # version that `clippy` used previously. Note that `clippy` can be upgraded without its
        # `rust-toolchain` file changing.
        if ! git diff --exit-code "$EXAMPLE"/rust-toolchain; then
            pushd examples/testing/straggler
            sed -i "s/^\(clippy_utils\>.*\)\<\(rev\|tag\) = \"[^\"]*\"\(.*\)$/\1$PREV_REV\3/" Cargo.toml
            sed -i "s/^channel = \"[^\"]*\"$/$PREV_CHANNEL/" rust-toolchain
            cargo build --tests
            popd
        fi
    fi

    if [[ "$EXAMPLE" = 'internal/template' ]]; then
        mv "$EXAMPLE"/Cargo.toml~ "$EXAMPLE"/Cargo.toml
    fi

    $CARGO_DYLINT upgrade "$EXAMPLE" --bisect

    if [[ "$EXAMPLE" = 'internal/template' ]]; then
        mv "$EXAMPLE"/Cargo.toml "$EXAMPLE"/Cargo.toml~
    fi
done
