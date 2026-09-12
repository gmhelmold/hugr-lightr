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
"$RUNNER" merge --input "$RAW_DIR" --out "$MERGED_DIR"
python3 "$ROOT/benchmarks/reporter/report.py" \
    --input "$MERGED_DIR/merged.jsonl" \
    --out "$REPORTS_DIR"
