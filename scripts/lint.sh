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

RESTRICTION_DIRS="$(find examples/restriction -mindepth 1 -maxdepth 1 -type d ! -name .cargo)"
RESTRICTIONS="$(echo "$RESTRICTION_DIRS" | xargs -n 1 basename | tr '\n' ' ')"

EXPERIMENTAL_DIRS="$(find examples/experimental -mindepth 1 -maxdepth 1 -type d ! -name .cargo)"

RESTRICTIONS_AS_FLAGS="$(echo "$RESTRICTIONS" | sed 's/\<[^[:space:]]\+\>/--lib &/g')"

DIRS=". driver utils/linting examples/general examples/supplementary examples/restriction examples/testing/clippy $EXPERIMENTAL_DIRS"

force_check() {
    find "$WORKSPACE"/target -name .fingerprint -path '*/dylint/target/nightly-*' -exec rm -r {} \; || true
}

# smoelius: Since lint levels are now specified in Cargo.toml files, `clippy` no longer must be run
# in a separate pass.
FLAGS="--lib general --lib supplementary $RESTRICTIONS_AS_FLAGS --lib clippy"

# smoelius: `cargo clean` can't be used here because it would remove cargo-dylint.
# smoelius: The commented command doesn't do anything now that all workspaces in the
# repository share a top-level target directory. Is the command still necessary?
# smoelius: Yes, the next command is necessary to force `cargo check` to run.
force_check

DYLINT_RUSTFLAGS='-D warnings'
export DYLINT_RUSTFLAGS
echo "DYLINT_RUSTFLAGS='$DYLINT_RUSTFLAGS'"

# smoelius: `--all-targets` cannot be used here. It would cause the command to fail on the
# lint examples.
# smoelius: `--workspace` is needed because the `general` and `supplementary` workspaces contain
# root packages.
COMMAND="$CARGO_DYLINT dylint $FLAGS -- --all-features --tests --workspace"

for DIR in $DIRS; do
    pushd "$DIR"
    bash -c "$COMMAND"
    popd
done
