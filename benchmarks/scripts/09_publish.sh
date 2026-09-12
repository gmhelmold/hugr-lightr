#!/usr/bin/env bash
set -euo pipefail

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
ARCHIVE_DIR="${ARCHIVE_DIR:-$ROOT/results/archive}"
ARCHIVE="$ARCHIVE_DIR/benchmark-evidence.tar.gz"
CHECKSUM="$ARCHIVE.sha256"

for path in "$ROOT/results/raw" "$ROOT/results/merged" "$ROOT/results/reports"; do
    if [ ! -d "$path" ]; then
        printf 'required evidence directory not found: %s\n' "$path" >&2
        exit 1
    fi
done

mkdir -p "$ARCHIVE_DIR"
tar -czf "$ARCHIVE" -C "$ROOT" \
    benchmarks/benchmark-spec.yaml results/raw results/merged results/reports
shasum -a 256 "$ARCHIVE" > "$CHECKSUM"
printf 'archive: %s\nchecksum: %s\n' "$ARCHIVE" "$CHECKSUM"
