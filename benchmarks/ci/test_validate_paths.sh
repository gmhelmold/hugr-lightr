#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
GATE="$ROOT/benchmarks/ci/validate_paths.sh"
SOURCE_MANIFEST="$ROOT/benchmarks/ci/paths.txt"
TMP=$(mktemp -d)
MANIFEST="$TMP/paths.txt"

cleanup() {
    rm -rf "$TMP"
}
trap cleanup EXIT

cp "$SOURCE_MANIFEST" "$MANIFEST"
PATHS_FILE="$MANIFEST" bash "$GATE"

printf '%s\n' 'benchmarks/ci/absent-path.sh' >> "$MANIFEST"
if PATHS_FILE="$MANIFEST" bash "$GATE"; then
    printf 'path gate accepted absent manifest path\n' >&2
    exit 1
fi

cp "$SOURCE_MANIFEST" "$MANIFEST"
PATHS_FILE="$MANIFEST" bash "$GATE"
printf 'path gate mutation test passed\n'
