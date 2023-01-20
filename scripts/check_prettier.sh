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

prettier --check --ignore-path <(echo examples; echo template) '**/*.md' '**/*.yml'

if ! git diff --exit-code; then
    echo "$0: aborting as repository is dirty" >&2
    exit 1
fi

prettier --write 'examples/**/*.md' 'internal/template/**/*.md' &&
    git diff --ignore-blank-lines | (! grep .) &&
    git checkout examples internal/template

scripts/unquote_yaml_strings.sh && git diff --exit-code
