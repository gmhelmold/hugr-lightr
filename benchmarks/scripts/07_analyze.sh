#!/usr/bin/env bash
set -euo pipefail

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
RUNNER="${BENCH_RUNNER:-$ROOT/benchmarks/runner/target/release/bench-runner}"
RAW_DIR="${RAW_DIR:-$ROOT/benchmarks/work/results/full}"
MERGED_DIR="${MERGED_DIR:-$ROOT/benchmarks/work/merged}"
REPORTS_DIR="${REPORTS_DIR:-$ROOT/benchmarks/work/reports}"

if [ ! -x "$RUNNER" ]; then
    printf 'bench-runner not executable: %s\n' "$RUNNER" >&2
    exit 1
fi
if [ ! -d "$RAW_DIR" ]; then
    printf 'raw evidence directory not found: %s\n' "$RAW_DIR" >&2
    exit 1
fi

mkdir -p "$MERGED_DIR" "$REPORTS_DIR"
# Runner merge produces: merged-raw.jsonl, merged.jsonl, summary.csv, summary.json, summary.md
"$RUNNER" merge --input "$RAW_DIR" --out "$MERGED_DIR"
# Copy merge outputs to reports dir
cp -f "$MERGED_DIR/summary.csv" "$REPORTS_DIR/"
cp -f "$MERGED_DIR/summary.json" "$REPORTS_DIR/"
cp -f "$MERGED_DIR/summary.md" "$REPORTS_DIR/"
cp -f "$MERGED_DIR/merged.jsonl" "$REPORTS_DIR/"
cp -f "$MERGED_DIR/merged-raw.jsonl" "$REPORTS_DIR/"
printf 'analysis complete, reports in %s\n' "$REPORTS_DIR"
