#! /bin/bash

set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "$0: expect one argument: version" >&2
    exit 1
fi

set -x

VERSION="version = \"$1\""

SCRIPTS="$(dirname "$(realpath "$0")")"
WORKSPACE="$(realpath "$SCRIPTS"/..)"

cd "$WORKSPACE"

find . -name Cargo.toml |
grep -vw template |
xargs -n 1 sed -i "{
s/^version = \"[^\"]*\"$/$VERSION/
}"

REQ="$(echo "$VERSION" | sed 's/"/"=/')"

find . -name Cargo.toml -exec sed -i "/^dylint/{
s/^\(.*\)\<version = \"[^\"]*\"\(.*\)$/\1$REQ\2/
}" {} \;
