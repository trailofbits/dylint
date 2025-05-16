#! /bin/bash

# set -x
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "$0: expect one argument: version" >&2
    exit 1
fi

VERSION="version = \"$1\""

SCRIPTS="$(dirname "$(realpath "$0")")"
WORKSPACE="$(realpath "$SCRIPTS"/..)"

cd "$WORKSPACE"

if ! scripts/check_CHANGELOG.sh refs/tags/v"$1"; then
    echo "$0: Please update CHANGELOG.md." >&2
    exit 1
fi

find . -name Cargo.toml |
grep -vw fixtures |
grep -vw template |
xargs -n 1 sed -i "{
s/^version = \"[^\"]*\"$/$VERSION/
}"

REQ="${VERSION/\"/\"=}"

find . -name Cargo.toml -exec sed -i "/^dylint/{
s/^\(.*\)\<version = \"[^\"]*\"\(.*\)$/\1$REQ\2/
}" {} \;

# smoelius: `template` must be handled specially because it does not use the `version = "..."`
# syntax.
sed -i "s/^\(dylint_[^ ]*\) = \"[^\"]*\"$/\1 = \"$1\"/" internal/template/Cargo.toml

scripts/update_lockfiles.sh
