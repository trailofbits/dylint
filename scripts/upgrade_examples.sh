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

EXPERIMENTAL_DIRS="$(find examples/experimental -mindepth 1 -maxdepth 1 -type d ! -name .cargo)"

for EXAMPLE in examples/general examples/supplementary examples/restriction $EXPERIMENTAL_DIRS examples/testing/clippy internal/template; do
    # smoelius: If the example's directory has changes, assume the example was already upgraded and
    # the script had to be restarted.
    if ! git diff --exit-code "$EXAMPLE"; then
        continue
    fi

    # smoelius: `clippy` requires special care.
    if [[ "$EXAMPLE" = 'examples/testing/clippy' ]]; then
        PREV_REV="$(sed -n 's/^clippy_utils\>.*\(\<\(rev\|tag\) = "[^"]*"\).*$/\1/;T;p' "$EXAMPLE"/Cargo.toml)"
        PREV_CHANNEL="$(sed -n 's/^channel = "[^"]*"$/&/;T;p' "$EXAMPLE"/rust-toolchain)"

        RUST_LOG=debug $CARGO_DYLINT upgrade "$EXAMPLE" --auto-correct

        REV="$(sed -n 's/^clippy_utils\>.*\(\<\(rev\|tag\) = "[^"]*"\).*$/\1/;T;p' "$EXAMPLE"/Cargo.toml)"
        sed -i "s/^\(clippy_config\>.*\)\<\(rev\|tag\) = \"[^\"]*\"\(.*\)$/\1$REV\3/" "$EXAMPLE"/Cargo.toml
        sed -i "s/^\(clippy_lints\>.*\)\<\(rev\|tag\) = \"[^\"]*\"\(.*\)$/\1$REV\3/" "$EXAMPLE"/Cargo.toml
        sed -i "s/^\(declare_clippy_lint\>.*\)\<\(rev\|tag\) = \"[^\"]*\"\(.*\)$/\1$REV\3/" "$EXAMPLE"/Cargo.toml

        # smoelius: If `clippy`'s `rust-toolchain` file changed, upgrade `straggler` to the Rust
        # version that `clippy` used previously. Note that `clippy` can be upgraded without its
        # `rust-toolchain` file changing.
        if ! git diff --exit-code "$EXAMPLE"/rust-toolchain; then
            pushd examples/testing/straggler
            sed -i "s/^\(clippy_utils\>.*\)\<\(rev\|tag\) = \"[^\"]*\"\(.*\)$/\1$PREV_REV\3/" Cargo.toml
            sed -i "s/^channel = \"[^\"]*\"$/$PREV_CHANNEL/" rust-toolchain
            # smoelius: If the upgraded library does not build, let CI fail after the PR has been
            # created, not now.
            # cargo build --all-targets
            popd
        fi
    fi

    RUST_LOG=debug $CARGO_DYLINT upgrade "$EXAMPLE" --auto-correct
done

if git diff --exit-code; then
    exit 0
fi

scripts/update_lockfiles.sh
