#! /bin/bash

set -euo pipefail

EXPENSIVE=true
SCHEDULED=true

case "${GITHUB_EVENT_NAME:?}" in
    pull_request | merge_group)
        BASE="$(jq -er \
            '.pull_request.base.sha // .merge_group.base_sha' \
            "${GITHUB_EVENT_PATH:?}")"
        if [[ ! "$BASE" =~ ^([[:xdigit:]]{40}|[[:xdigit:]]{64})$ ]]; then
            echo "Invalid CI base commit" >&2
            exit 1
        fi
        CHANGED="$(git diff --name-only --no-renames "$BASE" HEAD -- expensive scheduled)"
        EXPENSIVE=false
        SCHEDULED=false
        if grep -q '^expensive/' <<< "$CHANGED"; then
            EXPENSIVE=true
        fi
        if grep -q '^scheduled/' <<< "$CHANGED"; then
            SCHEDULED=true
        fi
        ;;
esac

jq -cn --argjson expensive "$EXPENSIVE" --argjson scheduled "$SCHEDULED" '
    {
        scheduled: $scheduled,
        entry: ([
            {environment: "ubuntu-latest", package: "cargo-dylint"},
            {environment: "ubuntu-latest", package: "dylint_examples"},
            {environment: "ubuntu-latest", package: "other"},
            {environment: "macos-latest", package: "cargo-dylint"},
            {environment: "macos-latest", package: "dylint_examples"},
            {environment: "macos-latest", package: "other"},
            {environment: "windows-latest", package: "cargo-dylint"},
            {environment: "windows-latest", package: "dylint_examples"},
            {environment: "windows-latest", package: "other"},
            {environment: "ubuntu-latest", package: "cargo-dylint-ci"},
            {environment: "ubuntu-latest", package: "expensive-auto_correct"},
            {environment: "ubuntu-latest", package: "expensive-alpine"},
            {environment: "ubuntu-latest", package: "expensive-boundary_toolchains"},
            {environment: "ubuntu-latest", package: "expensive-custom_toolchain"},
            {environment: "macos-latest", package: "expensive-auto_correct"},
            {environment: "macos-latest", package: "expensive-alpine"},
            {environment: "macos-latest", package: "expensive-boundary_toolchains"},
            {environment: "macos-latest", package: "expensive-custom_toolchain"},
            {environment: "windows-latest", package: "expensive-auto_correct"},
            {environment: "windows-latest", package: "expensive-alpine"},
            {environment: "windows-latest", package: "expensive-boundary_toolchains"},
            {environment: "windows-latest", package: "expensive-custom_toolchain"}
        ] | map(select($expensive or (.package | startswith("expensive-") | not))))
    }
'
