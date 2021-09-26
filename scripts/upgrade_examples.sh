#! /bin/bash

set -euo pipefail

if [[ $# -ne 0 ]]; then
    echo "$0: expect no arguments" >&2
    exit 1
fi

cd "$(dirname "$0")"/..

for EXAMPLE in examples/*; do
    if [[ ! -d "$EXAMPLE" ]]; then
        continue
    fi

    # smoelius: For now, ignore `allow_clippy`.
    if [[ "$EXAMPLE" = 'examples/allow_clippy' ]]; then
        continue
    fi

    cargo run -p cargo-dylint -- dylint --upgrade "$EXAMPLE"

    # smoelius: `clippy` requires special care.
    if [[ "$EXAMPLE" = 'examples/clippy' ]]; then
        pushd "$EXAMPLE"

        TAG="$(sed -n 's/^clippy_utils\>.*\(\<tag = "[^"]*"\).*$/\1/;T;p' Cargo.toml)"
        sed -i "s/^\\(clippy_lints\>.*\\)\<tag = \"[^\"]*\"\\(.*\\)$/\1$TAG\2/" Cargo.toml

        popd
    fi
done

