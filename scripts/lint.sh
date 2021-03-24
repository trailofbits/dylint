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

eval "$("$SCRIPTS"/build_examples.sh)"

# smoelius: Remove `allow_clippy` because it is just a joke. Also, for testing purposes, it uses a
# different toolchain than the other examples.
EXAMPLES="$(echo "$EXAMPLES" | sed 's/\<allow_clippy\>[[:space:]]*//')"

DIRS="."
for EXAMPLE in $EXAMPLES; do
    DIRS="$DIRS examples/$EXAMPLE"
done

# smoelius: `clippy` must be run separately because, for any lint not loaded alongside of it, rustc
# complains about the clippy-specific flags.
EXAMPLES="$(echo "$EXAMPLES" | sed 's/\<clippy\>[[:space:]]*//')"

for DIR in $DIRS; do
    pushd "$DIR"
    for LINTS in "$EXAMPLES" clippy; do
        unset DYLINT_RUSTFLAGS
        if [[ "$LINTS" = clippy ]]; then
            export DYLINT_RUSTFLAGS='
                -W clippy::style
                -W clippy::complexity
                -W clippy::perf
                -W clippy::pedantic
                -W clippy::nursery
            '
            #     -W clippy::cargo
        fi

        TMP="$(mktemp)"

        # "$CARGO_DYLINT" dylint $LINTS -- --workspace --tests --verbose
        "$CARGO_DYLINT" dylint $LINTS -- --workspace --tests --message-format=json |
        jq -r 'select(.reason == "compiler-message") | .message | select(.code != null) | .code | .code' |
        sort -u |
        while read X; do
            if ! grep "^$X$" "$SCRIPTS"/allow.txt >/dev/null; then
                echo "$X"
            fi
        done |
        cat > "$TMP"

        if [[ ! -s "$TMP" ]]; then
            continue
        fi

        cargo clean

        export DYLINT_RUSTFLAGS="$(
            cat "$TMP" |
            while read X; do
                echo -D "$X"
            done
        )"

        "$CARGO_DYLINT" dylint $LINTS -- --workspace --tests
        exit 1
    done
    popd
done
