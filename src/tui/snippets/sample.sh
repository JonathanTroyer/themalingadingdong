#!/usr/bin/env bash
# Syntax preview: comments, keywords, strings, escapes, variables.

set -euo pipefail

readonly MAX_ITEMS=100
readonly VERSION="1.0.0"

declare -A CONFIG=([name]="example" [count]=0 [enabled]="true")

validate_config() {
    local name="${CONFIG[name]}"
    if [[ -z "$name" ]]; then
        echo "Error: Name cannot be empty" >&2
        return 1
    fi
    return 0
}

process() {
    local -a items=("$@")
    for item in "${items[@]}"; do
        if (( item < 0 )); then
            continue
        fi
        local is_even=$(( item % 2 == 0 ))
        echo "$item: $is_even"
    done
}

parse_email() {
    local text="$1"
    if [[ "$text" =~ [a-zA-Z0-9._-]+@[a-zA-Z0-9._-]+\.[a-zA-Z]+ ]]; then
        echo "${BASH_REMATCH[0]}"
    fi
}

msg=$'Hello\tWorld\n'
echo "Config: name=${CONFIG[name]}, msg=$msg, version=$VERSION"
validate_config && process 1 2 -3 4 5
