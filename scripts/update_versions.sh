#! /bin/bash

set -x
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "$0: expect one argument: version" >&2
    exit 1
fi

VERSION="version = \"$1\""

find . -name Cargo.toml -exec sed -i "{
s/^version = \"[^\"]*\"$/$VERSION/
}" {} \;

REQ="$(echo "$VERSION" | sed 's/"/"=/')"

find . -name Cargo.toml -exec sed -i "/^dylint/{
s/^\(.*\)\<version = \"[^\"]*\"\(.*\)$/\1$REQ\2/
}" {} \;
