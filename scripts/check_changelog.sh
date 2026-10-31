#! /bin/bash

# set -x
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "$0: expect one argument: \`github.ref\`" >&2
    exit 1
fi

REF="$1"

if [[ ${REF::11} != 'refs/tags/v' ]]; then
    echo "$0: expect \`github.ref\` to start with \`refs/tags/v\`: $REF" >&2
    exit 1
fi

VERSION="${REF:11}"

SCRIPTS="$(dirname "$(realpath "$0")")"
WORKSPACE="$(realpath "$SCRIPTS"/..)"

cd "$WORKSPACE"

grep "^## $VERSION$" CHANGELOG.md

scripts/lint_changelog.sh
