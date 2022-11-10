#! /bin/bash

# set -x
set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "$0: expect two arguments: from tag and to tag" >&2
    exit 1
fi

# FROM_TAG="$1"
TO_TAG="$2"

OUR_TOML_RS="$PWD"/"$(find . -name toml.rs)"
THEIR_TOML_RS='./src/cargo/util/toml/mod.rs'

TMP="$(mktemp -d)"

git clone 'https://github.com/rust-lang/cargo' "$TMP"
cd "$TMP"
# git checkout "$FROM_TAG"
# cp "$OUR_TOML_RS" "$THEIR_TOML_RS"
# git stash
git checkout "$TO_TAG"
# git stash pop || true
cp "$THEIR_TOML_RS" "$OUR_TOML_RS"
