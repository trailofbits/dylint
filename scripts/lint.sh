#! /bin/bash

# shellcheck disable=SC2001

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

EXAMPLE_DIRS="$(find examples -mindepth 2 -maxdepth 2 -type d ! -name .cargo ! -name src)"

# smoelius: Remove `straggler`, as it uses a different toolchain than the other examples.
EXAMPLE_DIRS="$(echo "$EXAMPLE_DIRS" | sed 's,\<examples/testing/straggler\>[[:space:]]*,,')"

EXAMPLES="$(echo "$EXAMPLE_DIRS" | xargs -n 1 basename | tr '\n' ' ')"

# smoelius: `clippy` must be run separately because, for any lint not loaded alongside of it, rustc
# complains about the clippy-specific flags.
EXAMPLES="$(echo "$EXAMPLES" | sed 's/\<clippy\>[[:space:]]*//')"

RESTRICTION_DIRS="$(echo examples/restriction/*)"
RESTRICTIONS="$(echo "$RESTRICTION_DIRS" | xargs -n 1 basename | tr '\n' ' ')"

# smoelius: `overscoped_allow` must be run after other lints have been run. (See its documentation.)
EXAMPLES="$(echo "$EXAMPLES" | sed 's/\<overscoped_allow\>[[:space:]]*//')"
RESTRICTIONS="$(echo "$RESTRICTIONS" | sed 's/\<overscoped_allow\>[[:space:]]*//')"

# smoelius: `derive_opportunity` and `missing_doc_comment_openai` aren't ready for primetime yet.
EXAMPLES="$(echo "$EXAMPLES" | sed 's/\<derive_opportunity\>[[:space:]]*//')"
RESTRICTIONS="$(echo "$RESTRICTIONS" | sed 's/\<derive_opportunity\>[[:space:]]*//')"

EXAMPLES="$(echo "$EXAMPLES" | sed 's/\<missing_doc_comment_openai\>[[:space:]]*//')"
RESTRICTIONS="$(echo "$RESTRICTIONS" | sed 's/\<missing_doc_comment_openai\>[[:space:]]*//')"

RESTRICTIONS_AS_FLAGS="$(echo "$RESTRICTIONS" | sed 's/\<[^[:space:]]\+\>/--lib &/g')"

DIRS=". driver utils/linting examples/general examples/supplementary examples/testing/clippy $RESTRICTION_DIRS"

force_check() {
    find "$WORKSPACE"/target -name .fingerprint -path '*/dylint/target/nightly-*' -exec rm -r {} \; || true
}

for FLAGS in "--lib general --lib supplementary $RESTRICTIONS_AS_FLAGS" '--lib clippy'; do
    # smoelius: `cargo clean` can't be used here because it would remove cargo-dylint.
    # smoelius: The commented command doesn't do anything now that all workspaces in the
    # repository share a top-level target directory. Is the command still necessary?
    # smoelius: Yes, the next command is necessary to force `cargo check` to run.
    force_check

    DYLINT_RUSTFLAGS='-D warnings'
    if [[ "$FLAGS" = '--lib clippy' ]]; then
        # smoelius: `-W clippy::all` helps for checking those lints with `overscoped_allow`.
        DYLINT_RUSTFLAGS="$DYLINT_RUSTFLAGS
            -W clippy::all
            -W clippy::pedantic
            -W clippy::nursery
            -A clippy::option-if-let-else
            -A clippy::missing-errors-doc
            -A clippy::missing-panics-doc
            -A clippy::significant-drop-tightening
        "
    fi
    export DYLINT_RUSTFLAGS
    echo "DYLINT_RUSTFLAGS='$DYLINT_RUSTFLAGS'"

    # smoelius: `--all-targets` cannot be used here. It would cause the command to fail on the
    # lint examples.
    COMMAND="$CARGO_DYLINT dylint $FLAGS -- --all-features --tests"

    for DIR in $DIRS; do
        pushd "$DIR"
        bash -c "$COMMAND"
        popd
    done

    force_check

    # smoelius: For `overscoped_allow`.
    DYLINT_RUSTFLAGS="$(echo "$DYLINT_RUSTFLAGS" | sed 's/-D warnings\>//g;s/-W\>/--force-warn/g')"
    if [[ "$FLAGS" != '--lib clippy' ]]; then
        DYLINT_RUSTFLAGS="$DYLINT_RUSTFLAGS $(echo "$EXAMPLES" | sed 's/\<[^[:space:]]\+\>/--force-warn &/g')"
    fi
    export DYLINT_RUSTFLAGS
    echo "DYLINT_RUSTFLAGS='$DYLINT_RUSTFLAGS'"

    find . -name warnings.json -delete

    for DIR in $DIRS; do
        pushd "$DIR"
        bash -c "$COMMAND --message-format=json" >> warnings.json
        popd
    done

    force_check

    DYLINT_RUSTFLAGS='-D warnings'
    export DYLINT_RUSTFLAGS
    echo "DYLINT_RUSTFLAGS='$DYLINT_RUSTFLAGS'"

    # smoelius: All libraries must be named to enable their respective `cfg_attr`.
    COMMAND="$CARGO_DYLINT dylint $FLAGS --lib overscoped_allow -- --all-features --tests"

    for DIR in $DIRS; do
        pushd "$DIR"
        bash -c "$COMMAND"
        popd
    done
done
