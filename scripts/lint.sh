#! /bin/bash

# set -x
set -euo pipefail

if [[ $# -ne 0 ]]; then
    echo "$0: expect no arguments" >&2
    exit 1
fi

SCRIPTS="$(dirname "$(realpath "$0")")"

cargo build -p cargo-dylint
CARGO_DYLINT="$PWD/target/debug/cargo-dylint"

EXAMPLES="$(find examples -mindepth 1 -maxdepth 1 -type d | xargs -n 1 basename)"

# smoelius: Remove `allow_clippy` because it is just a joke. Also, for testing purposes, it uses a
# different toolchain than the other examples.
EXAMPLES="$(echo "$EXAMPLES" | sed 's/\<allow_clippy\>[[:space:]]*//')"

DIRS=". driver"
for EXAMPLE in $EXAMPLES; do
    DIRS="$DIRS examples/$EXAMPLE"
done

# smoelius: `clippy` must be run separately because, for any lint not loaded alongside of it, rustc
# complains about the clippy-specific flags.
EXAMPLES="$(echo "$EXAMPLES" | sed 's/\<clippy\>[[:space:]]*//')"

for DIR in $DIRS; do
    pushd "$DIR"
    for LINTS in "$EXAMPLES" clippy; do
        # smoelius: `cargo clean` can't be used here because it would remove cargo-dylint.
        # smoelius: The commented command doesn't do anything now that all workspaces in the
        # repository share a top-level target directory. Is the command still necesary?
        # rm -rf target/debug/deps

        unset DYLINT_RUSTFLAGS
        export DYLINT_RUSTFLAGS='-D warnings'
        if [[ "$LINTS" = clippy ]]; then
            DYLINT_RUSTFLAGS="$DYLINT_RUSTFLAGS
                -W clippy::pedantic
                -W clippy::nursery
                -A clippy::cargo-common-metadata
                -A clippy::empty-line-after-outer-attr
                -A clippy::missing-errors-doc
                -A clippy::missing-panics-doc
            "
            #     -W clippy::cargo
        fi

        "$CARGO_DYLINT" dylint $LINTS --workspace -- --all-features --tests
    done
    popd
done
