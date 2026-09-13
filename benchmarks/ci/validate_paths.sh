#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
PATHS_FILE=${PATHS_FILE:-"$ROOT/benchmarks/ci/paths.txt"}

if [[ ! -f $PATHS_FILE ]]; then
    printf 'path manifest not found: %s\n' "$PATHS_FILE" >&2
    exit 1
fi

count=0
declare -A seen=()
mapfile -t paths < "$PATHS_FILE"
for path in "${paths[@]}"; do
    if [[ -z $path || $path == /* || $path == *".."* ]]; then
        printf 'invalid path manifest entry: %s\n' "$path" >&2
        exit 1
    fi
    if [[ -n ${seen[$path]+present} ]]; then
        printf 'duplicate path manifest entry: %s\n' "$path" >&2
        exit 1
    fi
    seen[$path]=1
    if [[ ! -f $ROOT/$path ]]; then
        printf 'referenced path not found: %s\n' "$path" >&2
        exit 1
    fi
    count=$((count + 1))
done

if [[ $count -eq 0 ]]; then
    printf 'path manifest is empty: %s\n' "$PATHS_FILE" >&2
    exit 1
fi

printf 'validated %d CI paths\n' "$count"
