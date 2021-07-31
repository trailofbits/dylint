#! /bin/bash

# smoelius: This script is currently unused.

# set -x
set -euo pipefail

if [[ $# -ne 0 ]]; then
    echo "$0: expect no arguments" >&2
    exit 1
fi

cd "$(dirname "$0")"/../examples

EXAMPLES=
DYLINT_LIBRARY_PATH=

for EXAMPLE in *; do
    if [[ ! -d "$EXAMPLE" ]]; then
        continue
    fi

    pushd "$EXAMPLE" >/dev/null
    cargo build

    if [[ -z "$EXAMPLES" ]]; then
        EXAMPLES="$EXAMPLE"
    else
        EXAMPLES="$EXAMPLES $EXAMPLE"
    fi

    DEBUG="$PWD/target/debug"
    if [[ -z "$DYLINT_LIBRARY_PATH" ]]; then
        DYLINT_LIBRARY_PATH="$DEBUG"
    else
        DYLINT_LIBRARY_PATH="$DYLINT_LIBRARY_PATH;$DEBUG"
    fi

    popd >/dev/null
done

echo export EXAMPLES=\'"$EXAMPLES"\'
echo export DYLINT_LIBRARY_PATH=\'"$DYLINT_LIBRARY_PATH"\'
