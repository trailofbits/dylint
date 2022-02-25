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

        # "$CARGO_DYLINT" dylint $LINTS --workspace -- --all-features --tests --verbose
        "$CARGO_DYLINT" dylint $LINTS --workspace -- --all-features --tests --message-format=json |
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

        rm -rf target/debug/deps

        export DYLINT_RUSTFLAGS="$(
            cat "$TMP" |
            while read X; do
                echo -D "$X"
            done
        )"

        "$CARGO_DYLINT" dylint $LINTS --workspace -- --all-features --tests
        exit 1
    done
    popd
done
