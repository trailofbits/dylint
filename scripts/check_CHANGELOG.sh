#! /bin/bash

# set -x
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "$0: expect one argument: `github.ref`" >&2
    exit 1
fi

REF="$1"

if [[ ${REF::11} != 'refs/tags/v' ]]; then
    echo "$0: expect \`github.ref\` to start with \`refs/tags/v\`: $REF" >&2
    exit 1
fi

VERSION="${REF:11}"

cd "$(dirname "$0")"/..

grep "^## $VERSION$" CHANGELOG.md
