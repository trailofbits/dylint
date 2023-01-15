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

cargo build -p cargo-dylint
CARGO_DYLINT="$PWD/target/debug/cargo-dylint"

EXAMPLE_DIRS="$(find examples -mindepth 2 -maxdepth 2 -type d)"

# smoelius: Remove `straggler`, as it is used only for testing purposes. Also, it uses a different
# toolchain than the other examples.
# shellcheck disable=SC2001
EXAMPLE_DIRS="$(echo "$EXAMPLE_DIRS" | sed 's,\<examples/testing/straggler\>[[:space:]]*,,')"

DIRS=". driver $EXAMPLE_DIRS"

# smoelius: `clippy` must be run separately because, for any lint not loaded alongside of it, rustc
# complains about the clippy-specific flags.
EXAMPLES="$(echo "$EXAMPLE_DIRS" | xargs -n 1 basename | sed 's/\<clippy\>[[:space:]]*//')"

for LINTS in "$EXAMPLES" clippy; do
    # smoelius: `cargo clean` can't be used here because it would remove cargo-dylint.
    # smoelius: The commented command doesn't do anything now that all workspaces in the
    # repository share a top-level target directory. Is the command still necessary?
    # smoelius: Yes, the next command is necessary to force `cargo check` to run.
    find "$WORKSPACE"/target -name .fingerprint -path '*/dylint/target/nightly-*' -exec rm -r {} \; || true

    unset DYLINT_RUSTFLAGS
    export DYLINT_RUSTFLAGS='-D warnings'
    if [[ "$LINTS" = clippy ]]; then
        DYLINT_RUSTFLAGS="$DYLINT_RUSTFLAGS
            -W clippy::pedantic
            -W clippy::nursery
            -A clippy::option-if-let-else
            -A clippy::missing-errors-doc
            -A clippy::missing-panics-doc
        "
        #     -W clippy::cargo
    fi

    for DIR in $DIRS; do
        pushd "$DIR"

        # smoelius: `--all-targets` cannot be used here. It would cause the command to fail on the
        # lint examples.
        # shellcheck disable=SC2086
        "$CARGO_DYLINT" dylint $LINTS --workspace -- --all-features --tests

        popd
    done
done
