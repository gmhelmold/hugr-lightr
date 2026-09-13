#!/usr/bin/env bash
set -euo pipefail

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
RAW_DIR="${RAW_DIR:-$ROOT/benchmarks/work/results/full}"
MERGED_DIR="${MERGED_DIR:-$ROOT/benchmarks/work/merged}"
REPORTS_DIR="${REPORTS_DIR:-$ROOT/benchmarks/work/reports}"
ARCHIVE_DIR="${ARCHIVE_DIR:-$ROOT/benchmarks/work/archive}"
ARCHIVE="$ARCHIVE_DIR/benchmark-evidence.tar.gz"
CHECKSUM="$ARCHIVE.sha256"

for path in "$RAW_DIR" "$MERGED_DIR" "$REPORTS_DIR"; do
    if [ ! -d "$path" ]; then
        printf 'required evidence directory not found: %s\n' "$path" >&2
        exit 1
    fi
done

mkdir -p "$ARCHIVE_DIR"
tar -czf "$ARCHIVE" "$ROOT/benchmarks/benchmark-spec.yaml" "$RAW_DIR" "$MERGED_DIR" "$REPORTS_DIR"
shasum -a 256 "$ARCHIVE" > "$CHECKSUM"
printf 'archive: %s\nchecksum: %s\n' "$ARCHIVE" "$CHECKSUM"
