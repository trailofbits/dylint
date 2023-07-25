#! /bin/bash

# shellcheck disable=SC2001

# set -x
set -euo pipefail

if [[ $# -ne 0 ]]; then
    echo "$0: expect no arguments" >&2
    exit 1
fi

SCRIPTS="$(dirname "$(realpath "$0")")"
WORKSPACE="$(realpath "$SCRIPTS"/..)"

cd "$WORKSPACE"

grep -o '\[[0-9A-Fa-f]*\]([^)]*)' CHANGELOG.md |
while read -r LINK; do
    TEXT="$(echo "$LINK" | sed 's/^\[\([^]]*\)\](.*)$/\1/')"
    URL="$(echo "$LINK" | sed 's/^\[[^]]*\](\(.*\))$/\1/')"
    if [[
        ( ${#TEXT} -eq 7 || "$TEXT" = 'c28639ee' ) &&
        $(expr "$URL" : "https://.*/commit/${TEXT}[0-9a-z]*") -eq ${#URL}
    ]]; then
        continue
    fi
    echo "bad link: $LINK" >&2
    exit 1
done

grep -o '\[#[0-9]*\]([^)]*)' CHANGELOG.md |
while read -r LINK; do
    N="$(echo "$LINK" | sed 's/^\[#\([^]]*\)\](.*)$/\1/')"
    URL="$(echo "$LINK" | sed 's/^\[[^]]*\](\(.*\))$/\1/')"
    if [[
        $(expr "$URL" : "https://.*/issues/$N") -eq ${#URL} ||
        $(expr "$URL" : "https://.*/pull/$N") -eq ${#URL}
    ]]; then
        continue
    fi
    echo "bad link: $LINK" >&2
    exit 1
done
