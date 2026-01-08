#!/usr/bin/env bash
# Example shell script for syntax highlighting preview.

set -euo pipefail

readonly MAX_RETRIES=3
readonly API_URL="https://api.example.com"

declare -A CONFIG=(
    [name]="example"
    [enabled]="true"
    [retries]="$MAX_RETRIES"
)

validate_config() {
    local name="${CONFIG[name]}"
    local retries="${CONFIG[retries]}"

    if [[ -z "$name" ]]; then
        echo "Error: Name cannot be empty" >&2
        return 1
    fi

    if (( retries > 10 )); then
        echo "Error: Retries $retries exceeds maximum" >&2
        return 1
    fi

    return 0
}

process_items() {
    local -a items=("$@")
    local -A result=()

    for item in "${items[@]}"; do
        if (( item % 2 == 0 )); then
            result[$item]="true"
        else
            result[$item]="false"
        fi
    done

    for key in "${!result[@]}"; do
        echo "$key: ${result[$key]}"
    done
}

main() {
    echo "Config: name=${CONFIG[name]}, enabled=${CONFIG[enabled]}"

    if ! validate_config; then
        exit 1
    fi

    local items=(1 2 3 4 5)
    echo "Processing items..."
    process_items "${items[@]}"

    echo "Done!"
}

main "$@"
